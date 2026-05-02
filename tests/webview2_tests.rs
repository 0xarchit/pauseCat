#[test]
fn test_webview2_presence() {
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use windows::core::PWSTR;
    unsafe {
        let mut version = PWSTR::null();
        let result = GetAvailableCoreWebView2BrowserVersionString(windows::core::PCWSTR::null(), &mut version);
        println!("WebView2 installed: {}", result.is_ok());
    }
}
