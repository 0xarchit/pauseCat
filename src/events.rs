use crate::settings::Settings;

#[derive(Debug, Clone)]
pub enum AppEvent {
    ShowOverlay,
    HideOverlay,
    UserDismissed,
    TogglePause,
    OpenSettings,
    Quit,
    ConfigChanged(Settings),
    ThemeChanged(bool), // true = Dark, false = Light
    CheckForUpdates,
    UpdateStatus(crate::updater::UpdateInfo),
    StartUpdate,
    UpdateProgress(u32), // Percentage 0-100
    UpdateError(String),
}
