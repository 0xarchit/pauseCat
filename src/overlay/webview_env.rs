use std::sync::Mutex;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::CreateCoreWebView2EnvironmentCompletedHandler;
use windows::core::*;
use std::path::PathBuf;

struct SendSafeEnv(ICoreWebView2Environment);
unsafe impl Send for SendSafeEnv {}
unsafe impl Sync for SendSafeEnv {}

lazy_static::lazy_static! {
    static ref GLOBAL_ENV: Mutex<Option<SendSafeEnv>> = Mutex::new(None);
}

pub fn get_assets_path() -> PathBuf {
    // 1. Check Config Dir (Lazy-loaded assets)
    let mut config_path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    config_path.push("PauseCat");
    config_path.push("assets");
    if config_path.exists() && config_path.is_dir() {
        return config_path;
    }

    // 2. Check near EXE (Bundled assets)
    if let Ok(mut path) = std::env::current_exe() {
        path.pop();
        path.push("assets");
        if path.exists() { return path; }
    }
    
    // 3. Fallback to CWD
    let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    path.push("assets");
    path
}

pub fn init_global_env() -> Result<()> {
    unsafe {
        // Note: CoInitializeEx should be called on the main thread before this.
        let mut config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        config_dir.push("PauseCat");
        let mut webview_data = config_dir.clone();
        webview_data.push("WebViewData_Shared");
        let _ = std::fs::create_dir_all(&webview_data);
        
        let data_path_h = HSTRING::from(webview_data.to_str().unwrap_or_default());

        // Optimization: Non-blocking async initialization. 
        // The callback will set the global environment whenever it's ready.
        CreateCoreWebView2EnvironmentWithOptions(None, PCWSTR(data_path_h.as_ptr()), None, 
            &CreateCoreWebView2EnvironmentCompletedHandler::create(
                Box::new(move |_result, env| {
                    if let Some(e) = env {
                        if let Ok(mut lock) = GLOBAL_ENV.lock() {
                            *lock = Some(SendSafeEnv(e));
                        }
                    }
                    Ok(())
                })
            )
        )?;
        
        Ok(())
    }
}

pub fn get_global_env() -> Option<ICoreWebView2Environment> {
    GLOBAL_ENV.lock().unwrap().as_ref().map(|e| e.0.clone())
}
