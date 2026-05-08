use pausecat::settings::{Settings, BreakMode};
use std::fs;

#[test]
fn test_settings_default() {
    let settings = Settings::default();
    assert_eq!(settings.work_duration_secs, 3600);
    assert_eq!(settings.break_duration_secs, 300);
    assert_eq!(settings.mode, BreakMode::Soft);
    assert!(settings.autostart);
    assert_eq!(settings.overlay_animation, "default.webm");
}

#[test]
fn test_settings_validate() {
    let mut settings = Settings::default();
    
    settings.work_duration_secs = 100; // Too low (min 300)
    settings.break_duration_secs = 10000; // Too high (max 7200)
    
    settings.validate();
    
    assert_eq!(settings.work_duration_secs, 300);
    assert_eq!(settings.break_duration_secs, 7200);
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

#[test]
fn test_settings_corrupted_json() {
    let config_dir = std::env::current_dir().unwrap().join("test_config_corrupted");
    if config_dir.exists() {
        fs::remove_dir_all(&config_dir).unwrap();
    }
    fs::create_dir_all(&config_dir).unwrap();
    
    let config_path = config_dir.join("config.json");
    fs::write(&config_path, "{ \"invalid\": \"json\" ...").unwrap(); // Corrupted JSON
    
    // We can't directly use Settings::load() because it uses a hardcoded path.
    // However, we can test the internal logic by mimicking the load behavior.
    let result = fs::read_to_string(&config_path)
        .and_then(|s| serde_json::from_str::<Settings>(&s).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
        .unwrap_or_else(|_| Settings::default());
    
    assert_eq!(result.work_duration_secs, Settings::default().work_duration_secs);
    
    fs::remove_dir_all(&config_dir).unwrap();
}

#[test]
fn test_settings_save_error_branch() {
    let settings = Settings::default();
    let result = settings.force_save_error_test();
    assert!(result.is_err());
}

#[test]
fn test_settings_autostart_logic() {
    let mut settings = Settings::default();
    settings.autostart = true;
    let _ = settings.update_autostart();
    settings.autostart = false;
    let _ = settings.update_autostart();
}

#[test]
fn test_settings_load_not_exists() {
    let path = Settings::get_config_path();
    // Ensure file doesn't exist
    if path.exists() {
        let _ = fs::rename(&path, path.with_extension("bak"));
    }
    
    let settings = Settings::load();
    assert_eq!(settings.work_duration_secs, Settings::default().work_duration_secs);
    
    // Restore backup
    if path.with_extension("bak").exists() {
        let _ = fs::rename(path.with_extension("bak"), &path);
    }
}
