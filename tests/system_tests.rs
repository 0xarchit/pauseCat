#[cfg(test)]
mod tests {
    use pausecat::system::*;

    #[test]
    fn test_dark_mode_check() {
        // Just smoke test, depends on registry
        let _ = is_dark_mode();
    }

    #[test]
    fn test_get_running_apps() {
        let apps = get_running_apps();
        assert!(!apps.is_empty());
    }

    #[test]
    fn test_foreground_process() {
        let name = get_foreground_process_name();
        // Might be None in headless CI, but we hit the lines
        println!("Foreground app: {:?}", name);
    }

    #[test]
    fn test_is_media_playing_smoke() {
        let _ = is_media_playing();
    }
    
    #[test]
    fn test_apply_themes_smoke() {
        // We can't easily get a real HWND that is valid without a window,
        // but we can pass a null one to hit the lines.
        use windows::Win32::Foundation::HWND;
        apply_immersive_dark_mode(HWND(std::ptr::null_mut()), true);
        set_tray_menu_theme(true);
    }
}
