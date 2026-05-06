#[cfg(test)]
mod tests {
    use pausecat::app::App;
    use pausecat::events::AppEvent;
    use pausecat::settings::Settings;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    #[test]
    fn test_app_state_toggle_pause() {
        let mut app = App::new();
        
        // Initial state should be unpaused
        assert!(!app.paused.load(Ordering::Relaxed));

        // Send real event
        app.event_tx.send(AppEvent::TogglePause).unwrap();
        app.drain_events();

        // Assert state changed
        assert!(app.paused.load(Ordering::Relaxed));

        // Toggle back
        app.event_tx.send(AppEvent::TogglePause).unwrap();
        app.drain_events();
        assert!(!app.paused.load(Ordering::Relaxed));
    }

    #[test]
    fn test_app_state_session_lock() {
        let mut app = App::new();
        
        assert!(!app.session_paused.load(Ordering::Relaxed));

        app.event_tx.send(AppEvent::SessionLocked).unwrap();
        app.drain_events();
        assert!(app.session_paused.load(Ordering::Relaxed));

        app.event_tx.send(AppEvent::SessionUnlocked).unwrap();
        app.drain_events();
        assert!(!app.session_paused.load(Ordering::Relaxed));
    }

    #[test]
    fn test_app_config_change_integration() {
        let mut app = App::new();
        let mut new_settings = Settings::default();
        new_settings.work_duration_secs = 9999;

        app.event_tx.send(AppEvent::ConfigChanged(new_settings.clone())).unwrap();
        app.drain_events();

        let current_settings = app.settings.read().unwrap();
        assert_eq!(current_settings.work_duration_secs, 9999);
    }

    #[test]
    fn test_app_theme_change_integration() {
        let mut app = App::new();
        
        // Test switching to dark mode
        app.event_tx.send(AppEvent::ThemeChanged(true)).unwrap();
        app.drain_events();
        assert!(app.is_dark_mode);

        // Test switching to light mode
        app.event_tx.send(AppEvent::ThemeChanged(false)).unwrap();
        app.drain_events();
        assert!(!app.is_dark_mode);
    }
}
