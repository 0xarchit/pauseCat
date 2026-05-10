use std::sync::{Mutex, OnceLock};
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::{CreateCoreWebView2EnvironmentCompletedHandler, CoreWebView2EnvironmentOptions};
use windows::core::*;
use std::path::PathBuf;
use crate::settings::Settings;

struct SendSafeEnv(ICoreWebView2Environment);
unsafe impl Send for SendSafeEnv {}
unsafe impl Sync for SendSafeEnv {}

static GLOBAL_ENV: OnceLock<Mutex<Option<SendSafeEnv>>> = OnceLock::new();

fn get_env_lock() -> &'static Mutex<Option<SendSafeEnv>> {
    GLOBAL_ENV.get_or_init(|| Mutex::new(None))
}

pub fn get_assets_path() -> PathBuf {
    let mut config_path = Settings::get_config_dir();
    config_path.push("assets");
    if config_path.exists() && config_path.is_dir() {
        return config_path;
    }

    if let Ok(mut path) = std::env::current_exe() {
        path.pop();
        path.push("assets");
        if path.exists() { return path; }
    }
    
    let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    path.push("assets");
    path
}

pub fn init_global_env() -> Result<()> {
    unsafe {
        let config_dir = Settings::get_config_dir();
        let mut webview_data = config_dir.clone();
        webview_data.push("WebViewData_Shared");
        let _ = std::fs::create_dir_all(&webview_data);
        
        let data_path_h = HSTRING::from(webview_data.to_str().unwrap_or_default());

        let options: ICoreWebView2EnvironmentOptions = CoreWebView2EnvironmentOptions::default().into();
        let _ = options.SetAdditionalBrowserArguments(w!("--process-per-site --disk-cache-size=10485760 --disable-features=Translate,EdgeCollections,EdgeWorkspaces"));

        CreateCoreWebView2EnvironmentWithOptions(None, PCWSTR(data_path_h.as_ptr()), Some(&options), 
            &CreateCoreWebView2EnvironmentCompletedHandler::create(
                Box::new(move |_result, env| {
                    if let Some(e) = env {
                        if let Ok(mut lock) = get_env_lock().lock() {
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
    get_env_lock().lock().unwrap().as_ref().map(|e| e.0.clone())
}
