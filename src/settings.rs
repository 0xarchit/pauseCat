use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use winreg::enums::*;
use winreg::RegKey;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SettingsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Registry error: {0}")]
    Registry(std::io::Error),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BreakMode {
    Soft,
    Hard,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    pub work_duration_secs: u32,
    pub break_duration_secs: u32,
    pub mode: BreakMode,
    pub autostart: bool,
    pub overlay_animation: String,
    pub whitelist: Vec<String>,
    pub break_messages: Vec<String>,
    pub randomize_messages: bool,
    pub show_work_duration_status: bool,
    pub bubble_opacity: f32,
    pub bubble_size: u32,
    pub bubble_pos_x: i32,
    pub bubble_pos_y: i32,
    pub animation_style: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            work_duration_secs: 3600,
            break_duration_secs: 300,
            mode: BreakMode::Soft,
            autostart: true,
            overlay_animation: "default.webm".to_string(),
            whitelist: Vec::new(),
            break_messages: vec![
                "Breathe In...".to_string(),
                "Take a stretch".to_string(),
                "Rest your eyes".to_string(),
                "Hydrate yourself".to_string()
            ],
            randomize_messages: false,
            show_work_duration_status: true,
            bubble_opacity: 0.1,
            bubble_size: 580,
            bubble_pos_x: 5,
            bubble_pos_y: 5,
            animation_style: "float".to_string(),
        }
    }
}

impl Settings {
    pub fn get_config_dir() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("PauseCat");
        path
    }

    pub fn get_config_path() -> PathBuf {
        let mut path = Self::get_config_dir();
        path.push("config.json");
        path
    }

    pub fn load() -> Self {
        let path = Self::get_config_path();
        if !path.exists() {
            return Self::default();
        }

        fs::read_to_string(&path)
            .and_then(|s| serde_json::from_str(&s).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
            .unwrap_or_else(|_| Self::default())
    }

    pub fn save(&self) -> Result<(), SettingsError> {
        let mut settings_to_save = self.clone();
        settings_to_save.validate();

        let dir = Self::get_config_dir();
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        let path = Self::get_config_path();
        let tmp_path = path.with_extension("tmp");
        
        let json = serde_json::to_string_pretty(&settings_to_save)?;
        fs::write(&tmp_path, json)?;
        fs::rename(&tmp_path, &path)?;

        self.update_autostart()?;

        Ok(())
    }

    pub fn validate(&mut self) {
        if self.work_duration_secs < 300 { self.work_duration_secs = 300; }
        if self.work_duration_secs > 14400 { self.work_duration_secs = 14400; }
        if self.break_duration_secs < 10 { self.break_duration_secs = 10; }
        if self.break_duration_secs > 7200 { self.break_duration_secs = 7200; }
        if self.bubble_opacity < 0.0 { self.bubble_opacity = 0.0; }
        if self.bubble_opacity > 1.0 { self.bubble_opacity = 1.0; }
    }

    pub fn force_save_error_test(&self) -> Result<(), SettingsError> {
        let invalid_path = std::path::PathBuf::from("/invalid/path/settings.json");
        let json = serde_json::to_string_pretty(self)?;
        fs::write(invalid_path, json).map_err(SettingsError::Io)
    }

    pub fn update_autostart(&self) -> Result<(), SettingsError> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = r"Software\Microsoft\Windows\CurrentVersion\Run";
        let (key, _) = hkcu.create_subkey(path).map_err(SettingsError::Registry)?;

        if self.autostart {
            let exe_path = std::env::current_exe().map_err(SettingsError::Io)?;
            key.set_value("PauseCat", &exe_path.to_str().unwrap_or(""))
                .map_err(SettingsError::Registry)?;
        } else {
            let _ = key.delete_value("PauseCat");
        }

        Ok(())
    }
}

#[cfg(test)]
mod internal_tests {
    use super::*;

    #[test]
    fn test_settings_sabotage_and_validation() {
        let s = Settings::default();
        let _ = s.force_save_error_test();
        
        let mut s2 = Settings::default();
        s2.work_duration_secs = 10;
        s2.validate();
        assert_eq!(s2.work_duration_secs, 300);
        
        s2.work_duration_secs = 20000;
        s2.validate();
        assert_eq!(s2.work_duration_secs, 14400);

        s2.break_duration_secs = 5;
        s2.validate();
        assert_eq!(s2.break_duration_secs, 10);

        s2.break_duration_secs = 10000;
        s2.validate();
        assert_eq!(s2.break_duration_secs, 7200);

        s2.bubble_opacity = -0.5;
        s2.validate();
        assert_eq!(s2.bubble_opacity, 0.0);

        s2.bubble_opacity = 1.5;
        s2.validate();
        assert_eq!(s2.bubble_opacity, 1.0);
    }
}
