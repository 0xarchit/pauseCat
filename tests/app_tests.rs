#[cfg(test)]
mod tests {
    use pausecat::app::App;
    use pausecat::events::AppEvent;
    use pausecat::settings::Settings;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_app_basic_state() {
        let mut app = App::new();
        
        // Theme
        app.event_tx.send(AppEvent::ThemeChanged(true)).unwrap();
        app.drain_events();
        assert!(app.is_dark_mode);
        
        // Pause
        app.event_tx.send(AppEvent::TogglePause).unwrap();
        app.drain_events();
        assert!(app.paused.load(Ordering::Relaxed));
        
        // Config
        let mut s = Settings::default();
        s.work_duration_secs = 123;
        app.event_tx.send(AppEvent::ConfigChanged(s)).unwrap();
        app.drain_events();
        assert_eq!(app.settings.read().unwrap().work_duration_secs, 123);
    }

    #[test]
    fn test_app_session_logic() {
        let mut app = App::new();
        app.event_tx.send(AppEvent::SessionLocked).unwrap();
        app.drain_events();
        assert!(app.session_paused.load(Ordering::Relaxed));
        
        app.event_tx.send(AppEvent::ShowOverlay).unwrap();
        app.drain_events();
        assert!(app.reminder_overlay.is_none()); // Blocked by session lock
    }

    #[test]
    fn test_app_ui_events_smoke() {
        let mut app = App::new();
        // These shouldn't panic
        app.event_tx.send(AppEvent::SettingsClosed).unwrap();
        app.event_tx.send(AppEvent::UserDismissed).unwrap();
        app.drain_events();
    }

    #[test]
    fn test_app_overlay_pre_captured() {
        let mut app = App::new();
        let bg_data = vec![0u8; 400];
        {
            let mut lock = app.pre_captured_bg.write().unwrap();
            *lock = Some((10, 10, bg_data));
        }
        
        app.event_tx.send(AppEvent::ShowOverlay).unwrap();
        app.drain_events();
        
        // Background should be cleared after showing
        let lock = app.pre_captured_bg.read().unwrap();
        assert!(lock.is_none());
    }
}
