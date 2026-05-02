use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Com::*;
use pausecat::settings::Settings;
use pausecat::tray::TrayIcon;
use pausecat::events::AppEvent;
use pausecat::timer;
use pausecat::overlay::{OverlayWindow, capture, blur};

const TIMER_ID_CHANNEL: usize = 1;

/// Main application structure to hold state and router logic.
struct App {
    settings: Arc<RwLock<Settings>>,
    paused: Arc<AtomicBool>,
    event_tx: mpsc::Sender<AppEvent>,
    event_rx: mpsc::Receiver<AppEvent>,
    tray: Option<TrayIcon>,
    overlay: Option<OverlayWindow>,
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
            overlay: None,
        }
    }

    fn init(&mut self) -> windows::core::Result<()> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }

        // 1. Initialize Tray
        self.tray = Some(TrayIcon::new(self.event_tx.clone())?);

        // 2. Start Timer Engine
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
                if self.overlay.is_none() {
                    self.show_overlay();
                }
            }
            AppEvent::HideOverlay | AppEvent::UserDismissed => {
                self.overlay = None;
            }
            AppEvent::TogglePause => {
                let new_paused = !self.paused.load(Ordering::Relaxed);
                self.paused.store(new_paused, Ordering::Relaxed);
                if let Some(ref tray) = self.tray {
                    tray.set_paused(new_paused);
                }
            }
            AppEvent::OpenSettings => {
                println!("Open Settings requested");
            }
            AppEvent::ConfigChanged(new_settings) => {
                let mut settings = self.settings.write().unwrap();
                *settings = new_settings;
                let _ = settings.save();
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
            
            if let Ok(overlay) = OverlayWindow::new(self.event_tx.clone(), captured.width, captured.height, blurred) {
                overlay.fade_in();
                self.overlay = Some(overlay);
            }
        }
    }

    fn drain_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            self.handle_event(event);
        }
    }
}

fn main() -> windows::core::Result<()> {
    let mut app = App::new();
    app.init()?;

    unsafe {
        // Set a Win32 timer to wake up the message loop and check our MPSC channel
        let _ = SetTimer(None, TIMER_ID_CHANNEL, 100, None);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);

            // After every message, and especially on our timer message, check for events
            app.drain_events();
        }
    }

    Ok(())
}
