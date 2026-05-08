use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};
use crate::events::AppEvent;
use crate::settings::Settings;
use crate::overlay::{capture, blur};
use crate::system;

pub fn sleep_interruptible(duration: Duration, paused: &AtomicBool) {
    let mut elapsed = Duration::from_millis(0);
    while elapsed < duration {
        if paused.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(50));
            continue;
        }
        thread::sleep(Duration::from_millis(50));
        elapsed += Duration::from_millis(50);
    }
}

pub fn run(settings: Arc<RwLock<Settings>>, event_tx: Sender<AppEvent>, paused: Arc<AtomicBool>) {
    run_optimized(settings, event_tx, paused, Arc::new(AtomicBool::new(false)), Arc::new(RwLock::new(None)));
}

pub fn run_optimized(
    settings: Arc<RwLock<Settings>>, 
    event_tx: Sender<AppEvent>, 
    paused: Arc<AtomicBool>,
    session_paused: Arc<AtomicBool>,
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
                    log::info!("Timer reactively updated.");
                }
            }
        }
        
        // PAUSE CHECK: Manual Pause OR Session Lock
        if paused.load(Ordering::Relaxed) || session_paused.load(Ordering::Relaxed) {
            last_tick = Instant::now();
            continue;
        }

        let elapsed = last_tick.elapsed();
        last_tick = Instant::now();
        
        if time_remaining > elapsed {
            time_remaining -= elapsed;
            
            if !is_breaking && time_remaining.as_secs() <= 5 && !pre_capture_triggered {
                pre_capture_triggered = true;
                
                let should_skip = {
                    let s = settings.read().unwrap();
                    if let Some(fg_process) = system::get_foreground_process_name() {
                        s.whitelist.iter().any(|p| p.to_lowercase() == fg_process.to_lowercase())
                    } else {
                        false
                    }
                };

                if should_skip {
                    time_remaining = Duration::from_secs(60);
                    pre_capture_triggered = false;
                    log::info!("Break postponed: Whitelisted app in focus.");
                    continue;
                }

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
                let should_skip = {
                    let s = settings.read().unwrap();
                    if let Some(fg_process) = system::get_foreground_process_name() {
                        s.whitelist.iter().any(|p| p.to_lowercase() == fg_process.to_lowercase())
                    } else {
                        false
                    }
                };

                if should_skip {
                    time_remaining = Duration::from_secs(60);
                    pre_capture_triggered = false;
                    log::info!("Break skipped: Whitelisted app in focus.");
                    continue;
                }

                if session_paused.load(Ordering::Relaxed) {
                    time_remaining = Duration::from_secs(5);
                    continue;
                }

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

#[cfg(test)]
mod internal_tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn test_sleep_interruptible() {
        let paused = Arc::new(AtomicBool::new(false));
        let start = Instant::now();
        sleep_interruptible(Duration::from_millis(100), &paused);
        assert!(start.elapsed() >= Duration::from_millis(100));
    }

    #[test]
    fn test_timer_reactive_and_pause() {
        let settings = Arc::new(RwLock::new(Settings::default()));
        let (tx, _rx) = mpsc::channel();
        let paused = Arc::new(AtomicBool::new(false));
        let session_paused = Arc::new(AtomicBool::new(false));
        let bg = Arc::new(RwLock::new(None));

        let s_clone = settings.clone();
        let tx_clone = tx.clone();
        let p_clone = paused.clone();
        let sp_clone = session_paused.clone();
        let bg_clone = bg.clone();

        thread::spawn(move || {
            run_optimized(s_clone, tx_clone, p_clone, sp_clone, bg_clone);
        });

        thread::sleep(Duration::from_millis(600));

        paused.store(true, Ordering::Relaxed);
        thread::sleep(Duration::from_millis(600));
        paused.store(false, Ordering::Relaxed);

        session_paused.store(true, Ordering::Relaxed);
        thread::sleep(Duration::from_millis(600));
        session_paused.store(false, Ordering::Relaxed);

        {
            let mut s = settings.write().unwrap();
            s.work_duration_secs = 5000;
        }
        thread::sleep(Duration::from_millis(600));
    }
}
