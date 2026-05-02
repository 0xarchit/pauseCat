use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};
use crate::events::AppEvent;
use crate::settings::Settings;
use crate::overlay::{capture, blur};

pub fn run(settings: Arc<RwLock<Settings>>, event_tx: Sender<AppEvent>, paused: Arc<AtomicBool>) {
    run_optimized(settings, event_tx, paused, Arc::new(RwLock::new(None)));
}

pub fn run_optimized(
    settings: Arc<RwLock<Settings>>, 
    event_tx: Sender<AppEvent>, 
    paused: Arc<AtomicBool>,
    pre_captured_bg: Arc<RwLock<Option<(i32, i32, Vec<u8>)>>>
) {
    let mut last_tick = Instant::now();
    let mut work_remaining;
    
    {
        let s = settings.read().unwrap();
        work_remaining = Duration::from_secs(s.work_duration_secs as u64);
    }

    loop {
        thread::sleep(Duration::from_millis(500));
        
        if paused.load(Ordering::Relaxed) {
            last_tick = Instant::now();
            continue;
        }

        let elapsed = last_tick.elapsed();
        last_tick = Instant::now();
        
        if work_remaining > elapsed {
            work_remaining -= elapsed;
            
            // Optimization: Pre-capture 5 seconds before break starts
            if work_remaining.as_secs() == 5 {
                let bg_clone = pre_captured_bg.clone();
                thread::spawn(move || {
                    if let Ok(captured) = capture::capture_virtual_screen() {
                        let blurred = blur::blur(&captured.data, captured.width as usize, captured.height as usize, 10.0);
                        let mut lock = bg_clone.write().unwrap();
                        *lock = Some((captured.width, captured.height, blurred));
                    }
                });
            }
        } else {
            // Start break
            let _ = event_tx.send(AppEvent::ShowOverlay);
            
            let break_duration;
            {
                let s = settings.read().unwrap();
                break_duration = Duration::from_secs(s.break_duration_secs as u64);
            }
            
            thread::sleep(break_duration);
            
            let _ = event_tx.send(AppEvent::HideOverlay);
            
            {
                let s = settings.read().unwrap();
                work_remaining = Duration::from_secs(s.work_duration_secs as u64);
            }
            last_tick = Instant::now();
        }
    }
}
