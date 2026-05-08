#[cfg(test)]
mod tests {
    use pausecat::overlay::webview_env;

    #[test]
    fn test_webview_env_singleton_init() {
        // Initialize for the first time
        let _ = webview_env::init_global_env();
        
        // Initialize again to ensure it handles "already initialized" state gracefully
        let result = webview_env::init_global_env();
        
        // Even if it returns an error (already initialized), it shouldn't panic
        println!("Second init result: {:?}", result);
    }
}
