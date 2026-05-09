#![windows_subsystem = "windows"]

use pausecat::app::App;
use pausecat::settings::Settings;

struct SimpleFileLogger {
    file: std::sync::Mutex<std::fs::File>,
}

impl log::Log for SimpleFileLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            if let Ok(mut file) = self.file.lock() {
                use std::io::Write;
                let _ = writeln!(file, "[{}] {}", 
                    record.level(), 
                    record.args());
            }
        }
    }
    fn flush(&self) {}
}

fn setup_logging() -> windows::core::Result<()> {
    let mut path = Settings::get_config_dir();
    let _ = std::fs::create_dir_all(&path);
    path.push("app.log");
    
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| windows::core::Error::from_hresult(windows::core::HRESULT(e.raw_os_error().unwrap_or(-1) as i32)))?;

    let logger = SimpleFileLogger { file: std::sync::Mutex::new(file) };
    let _ = log::set_boxed_logger(Box::new(logger));
    log::set_max_level(log::LevelFilter::Info);

    log::info!("PauseCat started (Ultra-Optimized)");
    Ok(())
}

fn main() -> windows::core::Result<()> {
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_APARTMENTTHREADED);
        let _ = setup_logging();
        let mut app = App::new();
        app.run()?;
        windows::Win32::System::Com::CoUninitialize();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_startup_smoke() {
        unsafe {
            use windows::Win32::System::Threading::CreateMutexW;
            let mutex_name = windows::core::w!("Global\\PauseCatTestMutex");
            let _ = CreateMutexW(None, true, mutex_name);
            let _ = GetLastError();
        }
    }
}
