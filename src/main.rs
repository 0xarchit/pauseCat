#![windows_subsystem = "windows"]

use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Foundation::*;
use pausecat::app::App;

fn setup_logging() -> windows::core::Result<()> {
    let mut path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("PauseCat");
    let _ = std::fs::create_dir_all(&path);
    path.push("app.log");
    
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| windows::core::Error::from_hresult(windows::core::HRESULT(e.raw_os_error().unwrap_or(-1) as i32)))?;

    let target = Box::new(file);
    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Pipe(target))
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("PauseCat started (Optimized)");
    Ok(())
}

fn check_webview2() -> windows::core::Result<bool> {
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    unsafe {
        let mut version = windows::core::PWSTR::null();
        let result = GetAvailableCoreWebView2BrowserVersionString(windows::core::PCWSTR::null(), &mut version);
        Ok(result.is_ok())
    }
}

fn main() -> windows::core::Result<()> {
    let _ = setup_logging();

    unsafe {
        use windows::Win32::System::Threading::CreateMutexW;
        let _handle = CreateMutexW(None, true, windows::core::w!("Global\\PauseCatSingleInstanceMutex"));
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return Ok(());
        }
    }

    match check_webview2() {
        Ok(true) => log::info!("WebView2 found."),
        _ => {
            unsafe {
                MessageBoxW(
                    None,
                    windows::core::w!("PauseCat requires WebView2 Runtime."),
                    windows::core::w!("Error"),
                    MB_OK | MB_ICONERROR,
                );
            }
            return Ok(());
        }
    }

    let mut app = App::new();
    if let Err(e) = app.init() {
        return Err(e);
    }

    unsafe {
        let _ = SetTimer(None, 1, 100, None);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
            app.drain_events();
        }
    }

    Ok(())
}
