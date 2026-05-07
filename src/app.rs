use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Com::*;
use windows::Win32::Foundation::*;
use crate::settings::Settings;
use crate::tray::TrayIcon;
use crate::events::AppEvent;
use crate::timer;
use crate::overlay::{OverlayWindow, capture, blur, webview_env};
use crate::settings_ui::SettingsWindow;

pub struct App {
    pub settings: Arc<RwLock<Settings>>,
    pub paused: Arc<AtomicBool>,
    pub session_paused: Arc<AtomicBool>,
    pub event_tx: mpsc::Sender<AppEvent>,
    pub event_rx: mpsc::Receiver<AppEvent>,
    pub tray: Option<TrayIcon>,
    pub reminder_overlay: Option<OverlayWindow>,
    pub settings_window: Option<SettingsWindow>,
    pub was_media_playing: bool,
    pub is_dark_mode: bool,
    pub pre_captured_bg: Arc<RwLock<Option<(i32, i32, Vec<u8>)>>>,
}

impl App {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::channel::<AppEvent>();
        let settings = Arc::new(RwLock::new(Settings::load()));
        let paused = Arc::new(AtomicBool::new(false));
        let session_paused = Arc::new(AtomicBool::new(false));
        let is_dark_mode = crate::system::is_dark_mode();

        Self {
            settings,
            paused,
            session_paused,
            event_tx,
            event_rx,
            tray: None,
            reminder_overlay: None,
            settings_window: None,
            was_media_playing: false,
            is_dark_mode,
            pre_captured_bg: Arc::new(RwLock::new(None)),
        }
    }

    pub fn init(&mut self) -> windows::core::Result<()> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }

        let _ = webview_env::init_global_env();
        let _ = self.settings.read().unwrap().update_autostart();
        crate::system::set_tray_menu_theme(self.is_dark_mode);
        self.tray = Some(TrayIcon::new(self.event_tx.clone())?);

        crate::updater::cleanup_updates();

        let settings_clone = self.settings.clone();
        let event_tx_clone = self.event_tx.clone();
        let paused_clone = self.paused.clone();
        let session_paused_clone = self.session_paused.clone();
        let bg_clone = self.pre_captured_bg.clone();
        
        thread::spawn(move || {
            timer::run_optimized(
                settings_clone, 
                event_tx_clone, 
                paused_clone, 
                session_paused_clone,
                bg_clone
            );
        });

        Ok(())
    }

    pub fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::ShowOverlay => {
                if self.reminder_overlay.is_none() && !self.session_paused.load(Ordering::Relaxed) {
                    self.pause_media();
                    self.show_overlay_optimized();
                }
            }
            AppEvent::HideOverlay | AppEvent::UserDismissed => {
                if self.reminder_overlay.is_some() {
                    self.resume_media();
                }
                self.reminder_overlay = None;
            }
            AppEvent::SettingsClosed => {
                self.settings_window = None;
            }
            AppEvent::TogglePause => {
                let new_paused = !self.paused.load(Ordering::Relaxed);
                self.paused.store(new_paused, Ordering::Relaxed);
                if let Some(ref tray) = self.tray {
                    tray.set_paused(new_paused);
                }
            }
            AppEvent::OpenSettings => {
                if self.settings_window.is_none() {
                    let current_settings = self.settings.read().unwrap().clone();
                    if let Ok(win) = SettingsWindow::new(self.event_tx.clone(), current_settings) {
                        win.update_theme(self.is_dark_mode);
                        crate::system::apply_immersive_dark_mode(win.hwnd, self.is_dark_mode);
                        self.settings_window = Some(win);
                    }
                } else {
                    if let Some(ref win) = self.settings_window {
                        unsafe {
                            let _ = SetForegroundWindow(win.hwnd);
                            let _ = ShowWindow(win.hwnd, SW_SHOW);
                        }
                    }
                }
            }
            AppEvent::ConfigChanged(new_settings) => {
                let mut settings = self.settings.write().unwrap();
                *settings = new_settings;
                let _ = settings.save();
            }
            AppEvent::CheckForUpdates => {
                let event_tx = self.event_tx.clone();
                thread::spawn(move || {
                    match crate::updater::check_for_updates() {
                        Ok(info) => { let _ = event_tx.send(AppEvent::UpdateStatus(info)); }
                        Err(e) => { let _ = event_tx.send(AppEvent::UpdateError(e.to_string())); }
                    }
                });
            }
            AppEvent::UpdateStatus(info) => {
                if let Some(ref mut win) = self.settings_window {
                    win.send_update_status(info);
                }
            }
            AppEvent::StartUpdate => {
                let event_tx = self.event_tx.clone();
                thread::spawn(move || {
                    if let Err(e) = crate::updater::download_and_install(event_tx.clone()) {
                        let _ = event_tx.send(AppEvent::UpdateError(e.to_string()));
                    }
                });
            }
            AppEvent::UpdateProgress(percentage) => {
                if let Some(ref mut win) = self.settings_window {
                    win.send_update_progress(percentage);
                }
            }
            AppEvent::UpdateError(err) => {
                if let Some(ref mut win) = self.settings_window {
                    win.send_update_error(err);
                }
            }
            AppEvent::ThemeChanged(is_dark) => {
                self.is_dark_mode = is_dark;
                crate::system::set_tray_menu_theme(is_dark);
                if let Some(ref mut win) = self.settings_window {
                    win.update_theme(is_dark);
                    crate::system::apply_immersive_dark_mode(win.hwnd, is_dark);
                }
                if let Some(ref mut win) = self.reminder_overlay {
                    win.update_theme(is_dark);
                    crate::system::apply_immersive_dark_mode(win.hwnd, is_dark);
                }
            }
            AppEvent::SessionLocked => {
                self.session_paused.store(true, Ordering::Relaxed);
                self.reminder_overlay = None;
            }
            AppEvent::SessionUnlocked => {
                self.session_paused.store(false, Ordering::Relaxed);
            }
            AppEvent::Quit => {
                unsafe { PostQuitMessage(0) };
            }
        }
    }

    fn show_overlay_optimized(&mut self) {
        let (width, height, data) = if let Some(bg) = self.pre_captured_bg.read().unwrap().clone() {
            bg
        } else {
            if let Ok(captured) = capture::capture_virtual_screen() {
                let blurred = blur::blur(&captured.data, captured.width as usize, captured.height as usize, 10.0);
                (captured.width, captured.height, blurred)
            } else { return; }
        };

        let current_settings = self.settings.read().unwrap().clone();
        if let Ok(overlay) = OverlayWindow::new(self.event_tx.clone(), width, height, data, current_settings) {
            overlay.update_theme(self.is_dark_mode);
            crate::system::apply_immersive_dark_mode(overlay.hwnd, self.is_dark_mode);
            overlay.fade_in();
            self.reminder_overlay = Some(overlay);
        }
        
        let mut lock = self.pre_captured_bg.write().unwrap();
        *lock = None;
    }

    fn pause_media(&mut self) {
        if crate::system::is_media_playing() {
            unsafe {
                let _ = SendMessageW(HWND_BROADCAST, WM_APPCOMMAND, Some(WPARAM(0)), Some(LPARAM(47 << 16)));
            }
            self.was_media_playing = true; 
        } else {
            self.was_media_playing = false;
        }
    }

    fn resume_media(&mut self) {
        if self.was_media_playing {
            unsafe {
                let _ = SendMessageW(HWND_BROADCAST, WM_APPCOMMAND, Some(WPARAM(0)), Some(LPARAM(46 << 16)));
            }
            self.was_media_playing = false;
        }
    }

    pub fn drain_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            self.handle_event(event);
        }
    }
}

#[cfg(test)]
mod internal_tests {
    use super::*;

    #[test]
    fn test_app_logic_methods() {
        let mut app = App::new();
        
        // Test media logic (smoke)
        app.pause_media();
        app.resume_media();
        
        // Test handle_event with variants that don't trigger real UI popups
        app.handle_event(AppEvent::TogglePause);
        app.handle_event(AppEvent::SettingsClosed);
        app.handle_event(AppEvent::SessionLocked);
        app.handle_event(AppEvent::SessionUnlocked);
        app.handle_event(AppEvent::ThemeChanged(true));
        app.handle_event(AppEvent::UserDismissed);
        app.handle_event(AppEvent::HideOverlay);
        app.handle_event(AppEvent::UpdateStatus(crate::updater::UpdateInfo { available: true, latest_version: "1.1.0".to_string(), changelog: "test".to_string() }));
        app.handle_event(AppEvent::UpdateProgress(50));
        app.handle_event(AppEvent::UpdateError("error".to_string()));
        app.handle_event(AppEvent::ConfigChanged(Settings::default()));
        app.handle_event(AppEvent::ShowOverlay);
        app.handle_event(AppEvent::OpenSettings);
        app.handle_event(AppEvent::CheckForUpdates);
        app.handle_event(AppEvent::StartUpdate);
        app.handle_event(AppEvent::Quit);
        
        // Test media logic with mock-ish calls
        app.pause_media();
        app.resume_media();

        // Test init (smoke)
        // Now safe because windows are hidden in tests
        let _ = app.init();

        // Test draining
        app.event_tx.send(AppEvent::UserDismissed).unwrap();
        app.drain_events();
    }
}
