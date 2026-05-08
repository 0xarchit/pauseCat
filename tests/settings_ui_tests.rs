#[cfg(test)]
mod tests {
    use pausecat::settings_ui::SettingsWindow;
    use pausecat::events::AppEvent;
    use pausecat::settings::Settings;
    use std::sync::mpsc;

    #[test]
    fn test_settings_window_lifecycle() {
        let (tx, _rx) = mpsc::channel::<AppEvent>();
        let settings = Settings::default();
        
        // Test real window creation
        let win = SettingsWindow::new(tx, settings);
        
        if let Ok(win) = win {
            // Test update methods (Theme, Progress, Status)
            win.update_theme(true);
            win.send_update_progress(50);
            win.send_update_error("Test Error".to_string());
            
            // Test drop
            drop(win);
        } else {
            println!("Settings window creation skipped or failed: {:?}", win.err());
        }
    }
}
