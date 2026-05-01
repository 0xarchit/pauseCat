use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use pausecat::settings::Settings;
use pausecat::timer;

fn main() {
    println!("PauseCat starting...");

    let settings = Arc::new(RwLock::new(Settings::load()));
    let paused = Arc::new(AtomicBool::new(false));
    let (tx, _rx) = mpsc::channel();

    // In a real scenario, this would be spawned in a thread
    // let timer_thread = thread::spawn(move || {
    //     timer::run(settings, tx, paused);
    // });

    println!("Timer engine scaffolded.");
}
