use std::sync::Mutex;
use webview2_com::Microsoft::Web::WebView2::Win32::*;
use webview2_com::CreateCoreWebView2EnvironmentCompletedHandler;
use windows::core::*;
use windows::Win32::System::Com::*;
use std::path::PathBuf;

struct SendSafeEnv(ICoreWebView2Environment);
unsafe impl Send for SendSafeEnv {}
unsafe impl Sync for SendSafeEnv {}

lazy_static::lazy_static! {
    static ref GLOBAL_ENV: Mutex<Option<SendSafeEnv>> = Mutex::new(None);
}

pub fn get_assets_path() -> PathBuf {
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
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let mut config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        config_dir.push("PauseCat");
        let mut webview_data = config_dir.clone();
        webview_data.push("WebViewData_Shared");
        let _ = std::fs::create_dir_all(&webview_data);
        
        let data_path_h = HSTRING::from(webview_data.to_str().unwrap_or_default());

        let (tx, rx) = std::sync::mpsc::channel();

        CreateCoreWebView2EnvironmentWithOptions(None, PCWSTR(data_path_h.as_ptr()), None, 
            &CreateCoreWebView2EnvironmentCompletedHandler::create(
                Box::new(move |_result, env| {
                    let env_res = match env {
                        Some(e) => Ok(e),
                        None => Err(windows::core::Error::from_hresult(HRESULT(0x80004005u32 as i32))), // E_FAIL
                    };
                    let _ = tx.send(env_res);
                    Ok(())
                })
            )
        )?;

        if let Ok(Ok(env)) = rx.recv() {
            let mut lock = GLOBAL_ENV.lock().unwrap();
            *lock = Some(SendSafeEnv(env));
        }
        
        Ok(())
    }
}

pub fn get_global_env() -> Option<ICoreWebView2Environment> {
    GLOBAL_ENV.lock().unwrap().as_ref().map(|e| e.0.clone())
}
