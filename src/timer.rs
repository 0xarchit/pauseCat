use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};
use crate::events::AppEvent;
use crate::settings::Settings;
use crate::overlay::{capture, blur};
use crate::system;
use crate::app::wakeup_main_thread;

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

                log::info!("Starting background pre-capture...");
                let bg_clone = pre_captured_bg.clone();
                thread::spawn(move || {
                    if let Ok(captured) = capture::capture_virtual_screen() {
                        let blurred = blur::blur(&captured.data, captured.width as usize, captured.height as usize, 10.0);
                        let mut lock = bg_clone.write().unwrap();
                        *lock = Some((captured.width, captured.height, blurred));
                        log::info!("Background pre-capture complete.");
                    } else {
                        log::error!("Background pre-capture failed.");
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

                log::info!("Break triggered: Showing overlay.");
                is_breaking = true;
                let _ = event_tx.send(AppEvent::ShowOverlay);
                wakeup_main_thread();
                
                let s = settings.read().unwrap();
                time_remaining = Duration::from_secs(s.break_duration_secs as u64);
            } else {
                log::info!("Break complete: Hiding overlay.");
                is_breaking = false;
                pre_capture_triggered = false;
                let _ = event_tx.send(AppEvent::HideOverlay);
                wakeup_main_thread();
                
                let s = settings.read().unwrap();
                time_remaining = Duration::from_secs(s.work_duration_secs as u64);
            }
        }
    }
}
