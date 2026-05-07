use pausecat::timer::sleep_interruptible;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread;

#[test]
fn test_sleep_interruptible_basic() {
    let paused = Arc::new(AtomicBool::new(false));
    let start = Instant::now();
    let duration = Duration::from_millis(500);
    
    sleep_interruptible(duration, &paused);
    
    assert!(start.elapsed() >= duration);
}

#[test]
fn test_sleep_interruptible_pause() {
    let paused = Arc::new(AtomicBool::new(true));
    let paused_clone = paused.clone();
    
    // Start a thread to unpause after 500ms
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));
        paused_clone.store(false, Ordering::Relaxed);
    });
    
    let start = Instant::now();
    let duration = Duration::from_millis(500);
    
    // Should take at least 1s (500ms pause + 500ms sleep)
    sleep_interruptible(duration, &paused);
    
    assert!(start.elapsed() >= Duration::from_millis(1000));
}

#[test]
fn test_timer_loop_fast_forward() {
    use pausecat::timer::run_optimized;
    use pausecat::settings::Settings;
    use pausecat::events::AppEvent;
    use std::sync::{Arc, RwLock, mpsc};
    use std::sync::atomic::AtomicBool;

    let mut settings = Settings::default();
    settings.work_duration_secs = 1; // 1s work
    settings.break_duration_secs = 1; // 1s break
    let settings = Arc::new(RwLock::new(settings));
    
    let (tx, rx) = mpsc::channel();
    let paused = Arc::new(AtomicBool::new(false));
    let session_paused = Arc::new(AtomicBool::new(false));
    let pre_captured = Arc::new(RwLock::new(None));

    // Run timer in a separate thread so we can monitor events
    let settings_clone = settings.clone();
    let paused_clone = paused.clone();
    let session_paused_clone = session_paused.clone();
    let pre_captured_clone = pre_captured.clone();
    
    std::thread::spawn(move || {
        run_optimized(settings_clone, tx, paused_clone, session_paused_clone, pre_captured_clone);
    });

    // Wait for ShowOverlay (work duration 1s + loop sleep 0.5s)
    let event = rx.recv_timeout(Duration::from_secs(3)).expect("Should receive ShowOverlay");
    assert!(matches!(event, AppEvent::ShowOverlay));

    // Wait for HideOverlay (break duration 1s + loop sleep 0.5s)
    let event = rx.recv_timeout(Duration::from_secs(3)).expect("Should receive HideOverlay");
    assert!(matches!(event, AppEvent::HideOverlay));
}

#[test]
fn test_sleep_interruptible_stress() {
    use pausecat::timer::sleep_interruptible;
    let paused = Arc::new(AtomicBool::new(false));
    let duration = Duration::from_millis(100);
    
    // Rapid toggles
    paused.store(true, Ordering::SeqCst);
    paused.store(false, Ordering::SeqCst);
    paused.store(true, Ordering::SeqCst);
    
    // Spawn thread to unpause after 50ms
    let p_clone = paused.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        p_clone.store(false, Ordering::SeqCst);
    });
    
    sleep_interruptible(duration, &paused);
}

