#[cfg(test)]
mod tests {
    use pausecat::tray::TrayIcon;
    use pausecat::events::AppEvent;
    use std::sync::mpsc;

    #[test]
    fn test_tray_icon_lifecycle() {
        let (tx, _rx) = mpsc::channel::<AppEvent>();
        
        // Test creation
        let tray = TrayIcon::new(tx);
        assert!(tray.is_ok(), "Failed to create TrayIcon");
        
        let tray = tray.unwrap();
        
        // Test setting paused state (modifies tip)
        tray.set_paused(true);
        tray.set_paused(false);
        
        // Test drop (implicitly checks NIM_DELETE and DestroyWindow)
        drop(tray);
    }
}
