#[cfg(test)]
mod tests {
    use pausecat::events::AppEvent;
    use pausecat::settings::Settings;

    #[test]
    fn test_app_event_clone() {
        let event = AppEvent::ThemeChanged(true);
        let clone = event.clone();
        if let AppEvent::ThemeChanged(val) = clone {
            assert!(val);
        } else {
            panic!("Event type changed after clone");
        }
    }

    #[test]
    fn test_app_event_config_changed() {
        let settings = Settings::default();
        let event = AppEvent::ConfigChanged(settings.clone());
        if let AppEvent::ConfigChanged(s) = event {
            assert_eq!(s.work_duration_secs, settings.work_duration_secs);
        } else {
            panic!("Event type mismatch");
        }
    }
}
