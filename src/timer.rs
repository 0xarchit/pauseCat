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
    let mut time_remaining;
    let mut is_breaking = false;
    let mut pre_capture_triggered = false;
    
    let mut current_work_secs;
    let mut current_break_secs;
    
    {
        let s = settings.read().unwrap();
        current_work_secs = s.work_duration_secs;
        current_break_secs = s.break_duration_secs;
        time_remaining = Duration::from_secs(current_work_secs as u64);
    }

    loop {
        thread::sleep(Duration::from_millis(500));
        
        {
            let s = settings.read().unwrap();
            if s.work_duration_secs != current_work_secs || s.break_duration_secs != current_break_secs {
                current_work_secs = s.work_duration_secs;
                current_break_secs = s.break_duration_secs;
                
                if !is_breaking {
                    time_remaining = Duration::from_secs(current_work_secs as u64);
                    pre_capture_triggered = false;
                    last_tick = Instant::now();
                    log::info!("Timer reactively updated to {}m work / {}m break.", current_work_secs / 60, current_break_secs / 60);
                }
            }
        }
        
        if paused.load(Ordering::Relaxed) {
            last_tick = Instant::now();
            continue;
        }

        let elapsed = last_tick.elapsed();
        last_tick = Instant::now();
        
        if time_remaining > elapsed {
            time_remaining -= elapsed;
            
            if !is_breaking && time_remaining.as_secs() <= 5 && !pre_capture_triggered {
                pre_capture_triggered = true;
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
            if !is_breaking {
                is_breaking = true;
                let _ = event_tx.send(AppEvent::ShowOverlay);
                
                let s = settings.read().unwrap();
                time_remaining = Duration::from_secs(s.break_duration_secs as u64);
            } else {
                is_breaking = false;
                pre_capture_triggered = false;
                let _ = event_tx.send(AppEvent::HideOverlay);
                
                let s = settings.read().unwrap();
                time_remaining = Duration::from_secs(s.work_duration_secs as u64);
            }
        }
    }
}
