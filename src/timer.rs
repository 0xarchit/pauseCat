use crate::events::AppEvent;
use crate::settings::Settings;
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TimerState {
    Working,
    OnBreak,
}

pub fn run(config: Arc<RwLock<Settings>>, sender: Sender<AppEvent>, paused: Arc<AtomicBool>) {
    let mut state = TimerState::Working;
    
    loop {
        let duration = match state {
            TimerState::Working => {
                Duration::from_secs(config.read().unwrap().work_duration_secs as u64)
            }
            TimerState::OnBreak => {
                Duration::from_secs(config.read().unwrap().break_duration_secs as u64)
            }
        };

        sleep_interruptible(duration, &paused);
        
        match state {
            TimerState::Working => {
                sender.send(AppEvent::ShowOverlay).ok();
                state = TimerState::OnBreak;
            }
            TimerState::OnBreak => {
                sender.send(AppEvent::HideOverlay).ok();
                state = TimerState::Working;
            }
        }
    }
}

pub fn sleep_interruptible(duration: Duration, paused: &AtomicBool) {
    let mut remaining = duration;
    let step = Duration::from_millis(1000);
    
    while remaining > Duration::from_secs(0) {
        if paused.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(200));
            continue;
        }

        let actual_step = if remaining < step { remaining } else { step };
        thread::sleep(actual_step);
        remaining = remaining.saturating_sub(actual_step);
    }
}
