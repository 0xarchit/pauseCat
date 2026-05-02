#![windows_subsystem = "windows"]

use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Com::*;
use windows::Win32::Foundation::*;
use pausecat::settings::Settings;
use pausecat::tray::TrayIcon;
use pausecat::events::AppEvent;
use pausecat::timer;
use pausecat::overlay::{OverlayWindow, capture, blur};
use pausecat::settings_ui::SettingsWindow;

const TIMER_ID_CHANNEL: usize = 1;

/// Main application structure to hold state and router logic.
struct App {
    settings: Arc<RwLock<Settings>>,
    paused: Arc<AtomicBool>,
    event_tx: mpsc::Sender<AppEvent>,
    event_rx: mpsc::Receiver<AppEvent>,
    tray: Option<TrayIcon>,
    reminder_overlay: Option<OverlayWindow>,
    settings_window: Option<SettingsWindow>,
    // Track if we paused media to resume it later
    was_media_playing: bool,
}

impl App {
    fn new() -> Self {
        let (event_tx, event_rx) = mpsc::channel::<AppEvent>();
        let settings = Arc::new(RwLock::new(Settings::load()));
        let paused = Arc::new(AtomicBool::new(false));

        Self {
            settings,
            paused,
            event_tx,
            event_rx,
            tray: None,
            reminder_overlay: None,
            settings_window: None,
            was_media_playing: false,
        }
    }

    fn init(&mut self) -> windows::core::Result<()> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }

        // Ensure registry autostart matches current settings
        let _ = self.settings.read().unwrap().update_autostart();

        self.tray = Some(TrayIcon::new(self.event_tx.clone())?);

        let settings_clone = self.settings.clone();
        let event_tx_clone = self.event_tx.clone();
        let paused_clone = self.paused.clone();
        
        thread::spawn(move || {
            timer::run(settings_clone, event_tx_clone, paused_clone);
        });

        Ok(())
    }

    fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::ShowOverlay => {
                if self.reminder_overlay.is_none() {
                    self.pause_media();
                    self.show_overlay();
                }
            }
            AppEvent::HideOverlay | AppEvent::UserDismissed => {
                if self.reminder_overlay.is_some() {
                    self.resume_media();
                }
                self.reminder_overlay = None;
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
                        self.settings_window = Some(win);
                    }
                }
            }
            AppEvent::ConfigChanged(new_settings) => {
                let mut settings = self.settings.write().unwrap();
                *settings = new_settings;
                let _ = settings.save();
                log::info!("Settings updated and saved.");
            }
            AppEvent::Quit => {
                unsafe { PostQuitMessage(0) };
            }
        }
    }

    fn show_overlay(&mut self) {
        let capture_result = capture::capture_virtual_screen();
        if let Ok(captured) = capture_result {
            let blurred = blur::blur(&captured.data, captured.width as usize, captured.height as usize, 10.0);
            
            let current_settings = self.settings.read().unwrap().clone();
            if let Ok(overlay) = OverlayWindow::new(self.event_tx.clone(), captured.width, captured.height, blurred, current_settings) {
                overlay.fade_in();
                self.reminder_overlay = Some(overlay);
            }
        }
    }

    fn pause_media(&mut self) {
        // Use an explicit PAUSE command instead of a toggle.
        // Command 47 is APPCOMMAND_MEDIA_PAUSE.
        // This is safer as it won't start media if it's already stopped.
        unsafe {
            let _ = SendMessageW(HWND_BROADCAST, WM_APPCOMMAND, Some(WPARAM(0)), Some(LPARAM(47 << 16)));
        }
        self.was_media_playing = true; 
    }

    fn resume_media(&mut self) {
        if self.was_media_playing {
            // Command 46 is APPCOMMAND_MEDIA_PLAY.
            unsafe {
                let _ = SendMessageW(HWND_BROADCAST, WM_APPCOMMAND, Some(WPARAM(0)), Some(LPARAM(46 << 16)));
            }
            self.was_media_playing = false;
        }
    }

    fn drain_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            self.handle_event(event);
        }
    }
}

fn setup_logging() -> windows::core::Result<()> {
    let mut path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("PauseCat");
    std::fs::create_dir_all(&path).map_err(|e| windows::core::Error::from_hresult(windows::core::HRESULT(e.raw_os_error().unwrap_or(-1) as i32)))?;
    path.push("app.log");
    
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| windows::core::Error::from_hresult(windows::core::HRESULT(e.raw_os_error().unwrap_or(-1) as i32)))?;

    let target = Box::new(file);
    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Pipe(target))
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("PauseCat started");
    Ok(())
}

fn check_webview2() -> windows::core::Result<bool> {
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    unsafe {
        let mut version = windows::core::PWSTR::null();
        let result = GetAvailableCoreWebView2BrowserVersionString(windows::core::PCWSTR::null(), &mut version);
        Ok(result.is_ok())
    }
}

fn main() -> windows::core::Result<()> {
    let _ = setup_logging();

    unsafe {
        use windows::Win32::System::Threading::CreateMutexW;
        let _handle = CreateMutexW(None, true, windows::core::w!("Global\\PauseCatSingleInstanceMutex"));
        if GetLastError() == ERROR_ALREADY_EXISTS {
            log::warn!("Another instance of PauseCat is already running. exiting.");
            return Ok(());
        }
    }

    match check_webview2() {
        Ok(true) => log::info!("WebView2 runtime found."),
        _ => {
            log::error!("WebView2 runtime not found.");
            unsafe {
                MessageBoxW(
                    None,
                    windows::core::w!("PauseCat requires the Microsoft Edge WebView2 Runtime to be installed. Please install it and try again."),
                    windows::core::w!("PauseCat Error"),
                    MB_OK | MB_ICONERROR,
                );
            }
            return Ok(());
        }
    }

    let mut app = App::new();
    if let Err(e) = app.init() {
        log::error!("Failed to initialize app: {:?}", e);
        unsafe {
            MessageBoxW(
                None,
                windows::core::w!("Failed to initialize PauseCat. Please check the logs."),
                windows::core::w!("PauseCat Error"),
                MB_OK | MB_ICONERROR,
            );
        }
        return Err(e);
    }

    unsafe {
        let _ = SetTimer(None, TIMER_ID_CHANNEL, 100, None);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);

            app.drain_events();
        }
    }

    Ok(())
}
