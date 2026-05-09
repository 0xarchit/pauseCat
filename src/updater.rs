use std::fs;
use std::thread;
use serde::{Deserialize, Serialize};
use semver::Version;
use crate::settings::Settings;
use std::sync::mpsc::Sender;
use crate::events::AppEvent;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Networking::WinHttp::*;

const GITHUB_API_URL: &str = "https://api.github.com/repos/0xarchit/pauseCat/releases/latest";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GithubRelease {
    pub tag_name: String,
    pub assets: Vec<GithubAsset>,
    pub body: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GithubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateInfo {
    pub available: bool,
    pub latest_version: String,
    pub changelog: String,
}

struct WinHttpHandle(*mut core::ffi::c_void);
impl Drop for WinHttpHandle {
    fn drop(&mut self) {
        if !self.0.is_null() { unsafe { let _ = WinHttpCloseHandle(self.0); } }
    }
}

fn winhttp_get(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    unsafe {
        let h_session = WinHttpHandle(WinHttpOpen(
            windows::core::w!("PauseCat-Updater/1.0"),
            WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
            None,
            None,
            0,
        ));
        if h_session.0.is_null() { 
            let err = windows::core::Error::from_thread();
            log::error!("WinHttpOpen failed: {:?}", err);
            return Err(format!("WinHttpOpen failed: {:?}", err).into()); 
        }

        let _ = WinHttpSetTimeouts(h_session.0, 5000, 5000, 10000, 10000);

        let mut url_components = URL_COMPONENTS {
            dwStructSize: std::mem::size_of::<URL_COMPONENTS>() as u32,
            dwHostNameLength: u32::MAX,
            dwUrlPathLength: u32::MAX,
            dwExtraInfoLength: u32::MAX,
            ..Default::default()
        };

        let url_u16: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
        if let Err(e) = WinHttpCrackUrl(&url_u16, 0, &mut url_components) {
            log::error!("WinHttpCrackUrl failed for {}: {:?}", url, e);
            return Err(e.into());
        }

        let host_name_u16: Vec<u16> = std::slice::from_raw_parts(url_components.lpszHostName.0, url_components.dwHostNameLength as usize)
            .iter().copied().chain(std::iter::once(0)).collect();
        let path_name_u16: Vec<u16> = std::slice::from_raw_parts(url_components.lpszUrlPath.0, url_components.dwUrlPathLength as usize)
            .iter().copied().chain(std::iter::once(0)).collect();

        let h_connect = WinHttpHandle(WinHttpConnect(h_session.0, PCWSTR(host_name_u16.as_ptr()), url_components.nPort, 0));
        if h_connect.0.is_null() { 
            let err = windows::core::Error::from_thread();
            log::error!("WinHttpConnect failed for {:?}: {:?}", host_name_u16, err);
            return Err(format!("WinHttpConnect failed: {:?}", err).into()); 
        }

        let h_request = WinHttpHandle(WinHttpOpenRequest(
            h_connect.0,
            windows::core::w!("GET"),
            PCWSTR(path_name_u16.as_ptr()),
            None,
            None,
            std::ptr::null(),
            if url.starts_with("https") { WINHTTP_FLAG_SECURE } else { WINHTTP_OPEN_REQUEST_FLAGS(0) },
        ));
        if h_request.0.is_null() { 
            let err = windows::core::Error::from_thread();
            log::error!("WinHttpOpenRequest failed: {:?}", err);
            return Err(format!("WinHttpOpenRequest failed: {:?}", err).into()); 
        }

        let headers = windows::core::w!("Accept: application/vnd.github+json\r\nUser-Agent: PauseCat-Updater-v1\r\n");
        let _ = WinHttpAddRequestHeaders(h_request.0, headers.as_wide(), WINHTTP_ADDREQ_FLAG_ADD);

        if let Err(e) = WinHttpSendRequest(h_request.0, None, None, 0, 0, 0) {
            log::error!("WinHttpSendRequest failed: {:?}", e);
            return Err(e.into());
        }

        if let Err(e) = WinHttpReceiveResponse(h_request.0, std::ptr::null_mut()) {
            log::error!("WinHttpReceiveResponse failed: {:?}", e);
            return Err(e.into());
        }

        let mut status_code: u32 = 0;
        let mut dw_size = std::mem::size_of::<u32>() as u32;
        WinHttpQueryHeaders(
            h_request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            None,
            Some(&mut status_code as *mut _ as *mut _),
            &mut dw_size,
            std::ptr::null_mut(),
        )?;

        if status_code == 301 || status_code == 302 || status_code == 307 || status_code == 308 {
             let mut redirect_url = [0u16; 4096];
             let mut dw_size = 8192u32;
             WinHttpQueryHeaders(h_request.0, WINHTTP_QUERY_LOCATION, None, Some(redirect_url.as_mut_ptr() as *mut _), &mut dw_size, std::ptr::null_mut())?;
             let new_url = String::from_utf16_lossy(&redirect_url[.. (dw_size as usize / 2)]).trim_matches('\0').to_string();
             return winhttp_get(&new_url);
        }

        if status_code != 200 { 
            log::error!("HTTP Error: {} for {}", status_code, url);
            return Err(format!("HTTP Error: {}", status_code).into()); 
        }

        let mut response_data = Vec::new();
        let mut dw_size: u32 = 0;
        loop {
            WinHttpQueryDataAvailable(h_request.0, &mut dw_size as *mut _)?;
            if dw_size == 0 { break; }
            let mut buffer = vec![0u8; dw_size as usize];
            let mut dw_read: u32 = 0;
            WinHttpReadData(h_request.0, buffer.as_mut_ptr() as *mut _, dw_size, &mut dw_read)?;
            response_data.extend_from_slice(&buffer[..dw_read as usize]);
        }

        Ok(response_data)
    }
}

pub fn check_for_updates() -> Result<UpdateInfo, Box<dyn std::error::Error>> {
    let data = winhttp_get(GITHUB_API_URL)?;
    let release: GithubRelease = serde_json::from_slice(&data)?;
    parse_and_check_version(&release, APP_VERSION)
}

pub fn parse_and_check_version(release_json: &GithubRelease, current_version: &str) -> Result<UpdateInfo, Box<dyn std::error::Error>> {
    let latest_ver_str = release_json.tag_name.trim_start_matches('v');
    let current_ver = Version::parse(current_version).unwrap_or_else(|_| Version::new(1, 0, 0));
    let latest_ver = Version::parse(latest_ver_str).unwrap_or_else(|_| Version::new(1, 0, 0));
    Ok(UpdateInfo {
        available: latest_ver > current_ver,
        latest_version: release_json.tag_name.clone(),
        changelog: release_json.body.clone(),
    })
}

pub fn download_and_install(event_tx: Sender<AppEvent>) -> Result<(), Box<dyn std::error::Error>> {
    let data = winhttp_get(GITHUB_API_URL)?;
    let release: GithubRelease = serde_json::from_slice(&data)?;
    let asset = release.assets.iter().find(|a| a.name.to_lowercase().ends_with(".msi")).ok_or("No MSI installer found")?;

    let mut update_dir = Settings::get_config_dir();
    update_dir.push("Updates");
    if update_dir.exists() { let _ = fs::remove_dir_all(&update_dir); }
    fs::create_dir_all(&update_dir)?;
    let dest_path = update_dir.join(&asset.name);

    download_file_with_progress(&asset.browser_download_url, &dest_path, event_tx)?;

    unsafe {
        let exe_path = std::env::current_exe().unwrap_or_default();
        let cmd = format!("/c start /wait msiexec.exe /i \"{}\" /passive /norestart && timeout /t 2 /nobreak && start \"\" \"{}\"", dest_path.to_str().unwrap_or_default(), exe_path.to_str().unwrap_or_default());
        ShellExecuteW(None, windows::core::w!("open"), windows::core::w!("cmd.exe"), windows::core::PCWSTR(HSTRING::from(cmd).as_ptr()), None, SW_HIDE);
    }
    std::process::exit(0);
}

fn download_file_with_progress(url: &str, dest_path: &std::path::Path, event_tx: Sender<AppEvent>) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let h_session = WinHttpHandle(WinHttpOpen(windows::core::w!("PauseCat-Updater/1.0"), WINHTTP_ACCESS_TYPE_DEFAULT_PROXY, None, None, 0));
        if h_session.0.is_null() { return Err(format!("WinHttpOpen failed: {:?}", windows::core::Error::from_thread()).into()); }

        let mut url_components = URL_COMPONENTS { dwStructSize: std::mem::size_of::<URL_COMPONENTS>() as u32, dwHostNameLength: u32::MAX, dwUrlPathLength: u32::MAX, ..Default::default() };
        let url_u16: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
        WinHttpCrackUrl(&url_u16, 0, &mut url_components)?;

        let host_name_u16: Vec<u16> = std::slice::from_raw_parts(url_components.lpszHostName.0, url_components.dwHostNameLength as usize)
            .iter().copied().chain(std::iter::once(0)).collect();
        let path_name_u16: Vec<u16> = std::slice::from_raw_parts(url_components.lpszUrlPath.0, url_components.dwUrlPathLength as usize)
            .iter().copied().chain(std::iter::once(0)).collect();

        let h_connect = WinHttpHandle(WinHttpConnect(h_session.0, PCWSTR(host_name_u16.as_ptr()), url_components.nPort, 0));
        if h_connect.0.is_null() { return Err(format!("WinHttpConnect failed: {:?}", windows::core::Error::from_thread()).into()); }

        let h_request = WinHttpHandle(WinHttpOpenRequest(h_connect.0, windows::core::w!("GET"), PCWSTR(path_name_u16.as_ptr()), None, None, std::ptr::null(), if url.starts_with("https") { WINHTTP_FLAG_SECURE } else { WINHTTP_OPEN_REQUEST_FLAGS(0) }));
        if h_request.0.is_null() { return Err(format!("WinHttpOpenRequest failed: {:?}", windows::core::Error::from_thread()).into()); }

        WinHttpSendRequest(h_request.0, None, None, 0, 0, 0)?;
        WinHttpReceiveResponse(h_request.0, std::ptr::null_mut())?;
        let mut status_code: u32 = 0;
        let mut dw_size = std::mem::size_of::<u32>() as u32;
        WinHttpQueryHeaders(h_request.0, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, None, Some(&mut status_code as *mut _ as *mut _), &mut dw_size, std::ptr::null_mut())?;
        
        if status_code == 301 || status_code == 302 || status_code == 307 || status_code == 308 {
             let mut redirect_url = [0u16; 4096]; let mut dw_size = 8192u32;
             WinHttpQueryHeaders(h_request.0, WINHTTP_QUERY_LOCATION, None, Some(redirect_url.as_mut_ptr() as *mut _), &mut dw_size, std::ptr::null_mut())?;
             let new_url = String::from_utf16_lossy(&redirect_url[.. (dw_size as usize / 2)]).trim_matches('\0').to_string();
             return download_file_with_progress(&new_url, dest_path, event_tx);
        }

        if status_code != 200 { return Err(format!("HTTP Error: {}", status_code).into()); }
        let mut content_length: u64 = 0; let mut dw_size = std::mem::size_of::<u64>() as u32;
        let _ = WinHttpQueryHeaders(h_request.0, WINHTTP_QUERY_CONTENT_LENGTH | WINHTTP_QUERY_FLAG_NUMBER64, None, Some(&mut content_length as *mut _ as *mut _), &mut dw_size, std::ptr::null_mut());
        let mut file = fs::File::create(dest_path)?;
        let mut downloaded = 0u64; let mut dw_size: u32 = 0;
        loop {
            WinHttpQueryDataAvailable(h_request.0, &mut dw_size as *mut _)?;
            if dw_size == 0 { break; }
            let mut buffer = vec![0u8; dw_size as usize];
            let mut dw_read: u32 = 0;
            WinHttpReadData(h_request.0, buffer.as_mut_ptr() as *mut _, dw_size, &mut dw_read)?;
            std::io::Write::write_all(&mut file, &buffer[..dw_read as usize])?;
            downloaded += dw_read as u64;
            if content_length > 0 { let _ = event_tx.send(AppEvent::UpdateProgress((downloaded as f64 / content_length as f64 * 100.0) as u32)); }
        }
        Ok(())
    }
}

pub fn cleanup_updates() {
    let mut update_dir = Settings::get_config_dir();
    update_dir.push("Updates");
    if update_dir.exists() { let _ = fs::remove_dir_all(&update_dir); }
}

pub fn ensure_assets_sync(event_tx: Sender<AppEvent>) {
    thread::spawn(move || {
        let mut config_asset_path = Settings::get_config_dir();
        config_asset_path.push("assets");
        if !config_asset_path.exists() { let _ = fs::create_dir_all(&config_asset_path); }
        config_asset_path.push("default.webm");
        if config_asset_path.exists() && config_asset_path.metadata().map(|m| m.len() > 1000).unwrap_or(false) { return; }
        if let Ok(data) = winhttp_get(GITHUB_API_URL) {
            if let Ok(release) = serde_json::from_slice::<GithubRelease>(&data) {
                if let Some(asset) = release.assets.iter().find(|a| a.name == "default.webm") {
                    let _ = download_file_with_progress(&asset.browser_download_url, &config_asset_path, event_tx.clone());
                    let _ = event_tx.send(AppEvent::AssetDownloaded("default.webm".to_string()));
                    crate::app::wakeup_main_thread();
                }
            }
        }
    });
}
