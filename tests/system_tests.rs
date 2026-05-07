#[cfg(test)]
mod tests {
    use pausecat::system;

    #[test]
    fn test_is_dark_mode_readable() {
        // We can't guarantee the result (depends on OS), but we can verify it doesn't panic
        let _ = system::is_dark_mode();
    }

    #[test]
    fn test_get_running_apps_not_empty() {
        let apps = system::get_running_apps();
        // There should be at least one app running (the test runner itself)
        assert!(!apps.is_empty());
    }

    #[test]
    fn test_get_foreground_process_name() {
        let name = system::get_foreground_process_name();
        // Should either be None (if no window in focus) or some name
        // We can't predict the name, but we can verify it returns without error
        if let Some(n) = name {
            assert!(!n.is_empty());
        }
    }

    #[test]
    fn test_apply_immersive_dark_mode_smoke() {
        use windows::Win32::UI::WindowsAndMessaging::{CreateWindowExW, WINDOW_EX_STYLE, WS_OVERLAPPEDWINDOW, CW_USEDEFAULT, DestroyWindow};
        use windows::core::w;
        
        unsafe {
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("Static"),
                w!("Test"),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT, CW_USEDEFAULT, 10, 10,
                None, None, None, None
            ).unwrap();
            
            system::apply_immersive_dark_mode(hwnd, true);
            system::apply_immersive_dark_mode(hwnd, false);
            
            let _ = DestroyWindow(hwnd);
        }
    }

    #[test]
    fn test_tray_menu_theme_smoke() {
        system::set_tray_menu_theme(true);
        system::set_tray_menu_theme(false);
    }
}
