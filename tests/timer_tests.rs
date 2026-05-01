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
