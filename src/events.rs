use crate::settings::Settings;

#[derive(Debug, Clone)]
pub enum AppEvent {
    ShowOverlay,           // timer -> main: time to take a break
    HideOverlay,           // timer -> main: break is over
    UserDismissed,         // overlay -> main: user clicked skip (soft mode)
    TogglePause,           // tray -> main: toggle pause state
    OpenSettings,          // tray -> main: open settings window
    Quit,                  // tray -> main: exit cleanly
    ConfigChanged(Settings), // settings window -> main: apply new config
}
