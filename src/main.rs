use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use pausecat::settings::Settings;
use pausecat::tray::TrayIcon;
use pausecat::events::AppEvent;

fn main() {
    println!("PauseCat starting...");

    let settings = Arc::new(RwLock::new(Settings::load()));
    let _paused = Arc::new(AtomicBool::new(false));
    let (tx, _rx) = mpsc::channel::<AppEvent>();

    let _tray = TrayIcon::new(tx).expect("Failed to create tray icon");

    println!("System tray initialized.");
}
