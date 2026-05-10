use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use windows::Win32::UI::Shell::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Registry::*;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::core::HSTRING;

#[derive(Debug)]
pub enum SettingsError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Registry(windows::core::Error),
}

impl From<std::io::Error> for SettingsError { fn from(e: std::io::Error) -> Self { Self::Io(e) } }
impl From<serde_json::Error> for SettingsError { fn from(e: serde_json::Error) -> Self { Self::Json(e) } }
impl From<windows::core::Error> for SettingsError { fn from(e: windows::core::Error) -> Self { Self::Registry(e) } }

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::Json(e) => write!(f, "Serialization error: {}", e),
            Self::Registry(e) => write!(f, "Registry error: {}", e),
        }
    }
}
impl std::error::Error for SettingsError {}

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
    pub break_style: String, 
    pub custom_text: String,
    pub video_volume: f32, 
    pub text_animation: String,
    pub text_rotation_x: i32,
    pub text_rotation_y: i32,
    pub text_rotation_z: i32,
    pub text_color: String,
    pub text_opacity: f32,
    pub text_glow: f32,
    pub text_glow_enabled: bool,
    pub text_glow_color: String,
    pub text_depth: i32,
    pub adaptive_text_color: bool,
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
                "Take a deep breath".to_string(),
                "Stretch your body".to_string(),
                "Rest your eyes for a moment".to_string(),
                "Time for a quick water break".to_string()
            ],
            randomize_messages: true,
            show_work_duration_status: true,
            bubble_opacity: 0.1,
            bubble_size: 580,
            bubble_pos_x: 5,
            bubble_pos_y: 5,
            animation_style: "float".to_string(),
            break_style: "text".to_string(),
            custom_text: "PAUSE".to_string(),
            video_volume: 0.0,
            text_animation: "float".to_string(),
            text_rotation_x: 20,
            text_rotation_y: -20,
            text_rotation_z: 0,
            text_color: "#ffffff".to_string(),
            text_opacity: 0.15,
            text_glow: 10.0,
            text_glow_enabled: true,
            text_glow_color: "#ffffff".to_string(),
            text_depth: 5,
            adaptive_text_color: true,
        }
    }
}

impl Settings {
    pub fn get_config_dir() -> PathBuf {
        unsafe {
            if let Ok(path_ptr) = SHGetKnownFolderPath(&FOLDERID_RoamingAppData, KNOWN_FOLDER_FLAG(0), None) {
                let path_str = path_ptr.to_string().unwrap_or_default();
                CoTaskMemFree(Some(path_ptr.0 as *const _));
                let mut path = PathBuf::from(path_str);
                path.push("PauseCat");
                return path;
            }
        }
        PathBuf::from(".")
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
        if !dir.exists() { fs::create_dir_all(&dir)?; }

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

    pub fn update_autostart(&self) -> Result<(), SettingsError> {
        unsafe {
            let mut h_key = HKEY::default();
            let sub_key = windows::core::w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
            
            if RegCreateKeyExW(HKEY_CURRENT_USER, sub_key, Some(0), None, REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut h_key, None) == ERROR_SUCCESS {
                if self.autostart {
                    if let Ok(exe_path) = std::env::current_exe() {
                        let path_h = HSTRING::from(exe_path.to_str().unwrap_or_default());
                        let _ = RegSetValueExW(h_key, windows::core::w!("PauseCat"), Some(0), REG_SZ, Some(std::slice::from_raw_parts(path_h.as_ptr() as *const u8, (path_h.len() * 2 + 2) as usize)));
                    }
                } else {
                    let _ = RegDeleteValueW(h_key, windows::core::w!("PauseCat"));
                }
                let _ = RegCloseKey(h_key);
            }
        }
        Ok(())
    }
}
