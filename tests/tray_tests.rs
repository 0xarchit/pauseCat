#[cfg(test)]
mod tests {
    use pausecat::tray::TrayIcon;
    use pausecat::events::AppEvent;
    use std::sync::mpsc;

    #[test]
    fn test_tray_icon_lifecycle() {
        let (tx, _rx) = mpsc::channel::<AppEvent>();
        let tray = TrayIcon::new(tx);
        assert!(tray.is_ok());
        let tray = tray.unwrap();
        tray.set_paused(true);
        tray.set_paused(false);
        drop(tray);
    }
}
