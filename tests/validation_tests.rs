#[cfg(test)]
mod tests {
    use pausecat::settings::Settings;

    #[test]
    fn test_break_duration_limits() {
        let mut settings = Settings::default();
        
        // Test normal value
        settings.break_duration_secs = 600; // 10m
        settings.validate();
        assert_eq!(settings.break_duration_secs, 600);

        // Test over limit (should be capped at 7200)
        settings.break_duration_secs = 10000;
        settings.validate();
        assert_eq!(settings.break_duration_secs, 7200);

        // Test under limit (should be capped at 10)
        settings.break_duration_secs = 5;
        settings.validate();
        assert_eq!(settings.break_duration_secs, 10);
    }

    #[test]
    fn test_work_duration_limits() {
        let mut settings = Settings::default();
        
        // Test normal value
        settings.work_duration_secs = 3600; // 1h
        settings.validate();
        assert_eq!(settings.work_duration_secs, 3600);

        // Test over limit (should be capped at 14400)
        settings.work_duration_secs = 20000;
        settings.validate();
        assert_eq!(settings.work_duration_secs, 14400);

        // Test under limit (should be capped at 300)
        settings.work_duration_secs = 100;
        settings.validate();
        assert_eq!(settings.work_duration_secs, 300);
    }

    #[test]
    fn test_save_sanitization() {
        let mut settings = Settings::default();
        settings.break_duration_secs = 99999;
        settings.work_duration_secs = 99999;
        
        // save() internally calls validate() on a clone before writing to disk
        // We can't easily check the file without side effects, but we can check the logic again
        let mut clone = settings.clone();
        clone.validate();
        assert_eq!(clone.break_duration_secs, 7200);
        assert_eq!(clone.work_duration_secs, 14400);
    }
}
