use pausecat::settings::{Settings, BreakMode};
use std::fs;

#[test]
fn test_settings_default() {
    let settings = Settings::default();
    assert_eq!(settings.work_duration_secs, 3600);
    assert_eq!(settings.break_duration_secs, 300);
    assert_eq!(settings.mode, BreakMode::Soft);
    assert!(settings.autostart);
    assert_eq!(settings.overlay_animation, "cat.webp");
}

#[test]
fn test_settings_validate() {
    let mut settings = Settings::default();
    
    settings.work_duration_secs = 100; // Too low
    settings.break_duration_secs = 5000; // Too high
    
    settings.validate();
    
    assert_eq!(settings.work_duration_secs, 300);
    assert_eq!(settings.break_duration_secs, 1800);
}

#[test]
fn test_settings_serialization() {
    let settings = Settings::default();
    let json = serde_json::to_string(&settings).unwrap();
    let deserialized: Settings = serde_json::from_str(&json).unwrap();
    
    assert_eq!(settings.work_duration_secs, deserialized.work_duration_secs);
    assert_eq!(settings.break_duration_secs, deserialized.break_duration_secs);
    assert_eq!(settings.mode, deserialized.mode);
}

#[test]
fn test_settings_io() {
    // We use a temporary directory for IO tests
    let mut settings = Settings::default();
    settings.work_duration_secs = 1234;
    
    let config_dir = std::env::current_dir().unwrap().join("test_config");
    if config_dir.exists() {
        fs::remove_dir_all(&config_dir).unwrap();
    }
    fs::create_dir_all(&config_dir).unwrap();
    
    let config_path = config_dir.join("config.json");
    
    // Manual save to custom path for testing
    let json = serde_json::to_string_pretty(&settings).unwrap();
    fs::write(&config_path, json).unwrap();
    
    // Load and check
    let loaded_json = fs::read_to_string(&config_path).unwrap();
    let loaded: Settings = serde_json::from_str(&loaded_json).unwrap();
    
    assert_eq!(loaded.work_duration_secs, 1234);
    
    // Clean up
    fs::remove_dir_all(&config_dir).unwrap();
}
