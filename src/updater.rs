use std::fs;
use std::thread;
use std::io::Read;
use serde::{Deserialize, Serialize};
use semver::Version;
use crate::settings::Settings;
use std::sync::mpsc::Sender;
use crate::events::AppEvent;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::core::HSTRING;

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

pub fn parse_and_check_version(release_json: &GithubRelease, current_version: &str) -> Result<UpdateInfo, Box<dyn std::error::Error>> {
    let latest_ver_str = release_json.tag_name.trim_start_matches('v');
    let current_ver = Version::parse(current_version)?;
    let latest_ver = Version::parse(latest_ver_str)?;

    Ok(UpdateInfo {
        available: latest_ver > current_ver,
        latest_version: release_json.tag_name.clone(),
        changelog: release_json.body.clone(),
    })
}

pub fn parse_github_release(json: &str) -> Result<GithubRelease, Box<dyn std::error::Error>> {
    serde_json::from_str(json).map_err(|e| e.into())
}

pub fn check_for_updates() -> Result<UpdateInfo, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("PauseCat-Updater-v1")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client.get(GITHUB_API_URL)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        if status == reqwest::StatusCode::FORBIDDEN {
            return Err("GitHub API Forbidden. The repository might be private or rate-limited.".into());
        }
        return Err(format!("GitHub API error: {}", status).into());
    }

    let release: GithubRelease = response.json()?;
    parse_and_check_version(&release, APP_VERSION)
}

pub fn find_msi_asset(release: &GithubRelease) -> Option<&GithubAsset> {
    release.assets.iter()
        .find(|a| a.name.to_lowercase().ends_with(".msi"))
}

pub fn download_and_install(event_tx: Sender<AppEvent>) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("PauseCat-Updater-v1")
        .build()?;

    let release: GithubRelease = client.get(GITHUB_API_URL).send()?.json()?;
    let asset = find_msi_asset(&release)
        .ok_or("No MSI installer found in the latest release")?;

    let mut update_dir = Settings::get_config_dir();
    update_dir.push("Updates");
    
    // Purge old updates
    if update_dir.exists() { let _ = fs::remove_dir_all(&update_dir); }
    fs::create_dir_all(&update_dir)?;

    let mut dest_path = update_dir.clone();
    dest_path.push(&asset.name);

    let mut response = client.get(&asset.browser_download_url).send()?;
    let total_size = asset.size;
    let mut downloaded = 0u64;
    let mut buffer = [0; 8192];
    let mut file = fs::File::create(&dest_path)?;

    use std::io::Read;
    loop {
        let n = response.read(&mut buffer)?;
        if n == 0 { break; }
        std::io::Write::write_all(&mut file, &buffer[..n])?;
        downloaded += n as u64;
        if total_size > 0 {
            let percentage = (downloaded as f64 / total_size as f64 * 100.0) as u32;
            let _ = event_tx.send(AppEvent::UpdateProgress(percentage));
        }
    }
    let _ = event_tx.send(AppEvent::UpdateProgress(100));

    // PRO AUTO-RELAUNCH LOGIC:
    // We launch a detached CMD.EXE that:
    // 1. Starts the MSI installer (/i /passive)
    // 2. Waits for msiexec to finish
    // 3. Immediately restarts PauseCat from the installation directory
    unsafe {
        let exe_path = std::env::current_exe().unwrap_or_default();
        let exe_path_str = exe_path.to_str().unwrap_or_default();
        let msi_path_str = dest_path.to_str().unwrap_or_default();

        let operation = HSTRING::from("open");
        let file = HSTRING::from("cmd.exe");
        
        // Command Chain: 
        // start /wait msiexec -> delay 2s for file release -> start PauseCat
        let command = format!(
            "/c start /wait msiexec.exe /i \"{}\" /passive /norestart && timeout /t 2 /nobreak && start \"\" \"{}\"",
            msi_path_str,
            exe_path_str
        );
        let parameters = HSTRING::from(command);
        
        ShellExecuteW(
            None,
            windows::core::PCWSTR(operation.as_ptr()),
            windows::core::PCWSTR(file.as_ptr()),
            windows::core::PCWSTR(parameters.as_ptr()),
            None,
            SW_HIDE, // Hide the black console window
        );
    }

    #[cfg(not(test))]
    std::process::exit(0);
    #[cfg(test)]
    Ok(())
}

pub fn cleanup_updates() {
    let mut update_dir = Settings::get_config_dir();
    update_dir.push("Updates");
    if update_dir.exists() {
        let _ = fs::remove_dir_all(&update_dir);
    }
}

pub fn ensure_assets_sync(event_tx: Sender<AppEvent>) {
    thread::spawn(move || {
        log::info!("Starting asset sync check...");
        
        // 1. Determine the target path for the download (Config Dir)
        let mut config_asset_path = Settings::get_config_dir();
        config_asset_path.push("assets");
        if !config_asset_path.exists() {
            let _ = fs::create_dir_all(&config_asset_path);
        }
        config_asset_path.push("default.webm");

        // 2. Check if asset already exists and is valid
        if config_asset_path.exists() && config_asset_path.metadata().map(|m| m.len() > 1000).unwrap_or(false) {
            log::info!("Asset already exists and is valid: {:?}", config_asset_path);
            return;
        }

        log::info!("Asset missing or invalid, starting background fetch...");

        let client = match reqwest::blocking::Client::builder()
            .user_agent("PauseCat-Asset-Syncer-v1")
            .timeout(std::time::Duration::from_secs(60))
            .build() {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed to create HTTP client: {}", e);
                    let _ = event_tx.send(AppEvent::AssetDownloadError(format!("Client error: {}", e)));
                    return;
                }
            };

        // Retry loop for GitHub API
        let mut release: Option<GithubRelease> = None;
        for attempt in 1..=3 {
            log::info!("Fetching latest release info (attempt {}/3)...", attempt);
            match client.get(GITHUB_API_URL)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .send()
                .and_then(|r| r.json::<GithubRelease>()) {
                    Ok(r) => {
                        release = Some(r);
                        break;
                    }
                    Err(e) => {
                        log::error!("Attempt {} failed: {}", attempt, e);
                        if attempt < 3 { thread::sleep(std::time::Duration::from_secs(5)); }
                        else {
                            let _ = event_tx.send(AppEvent::AssetDownloadError(format!("GitHub API error: {}", e)));
                            return;
                        }
                    }
                }
        }

        let release = release.unwrap();
        let asset = match release.assets.iter().find(|a| a.name == "default.webm") {
            Some(a) => a,
            None => {
                log::warn!("default.webm not found in latest release assets");
                let _ = event_tx.send(AppEvent::AssetDownloadError("Asset not found in latest release".to_string()));
                return;
            }
        };

        log::info!("Downloading default.webm ({} bytes) from {}", asset.size, asset.browser_download_url);
        // Initial progress update to trigger UI state
        let _ = event_tx.send(AppEvent::AssetDownloadProgress(0));

        match client.get(&asset.browser_download_url).send() {
            Ok(mut response) => {
                let total_size = response.content_length().unwrap_or(asset.size as u64);
                match fs::File::create(&config_asset_path) {
                    Ok(mut file) => {
                        let mut buffer = [0; 16384];
                        let mut downloaded = 0;
                        let mut last_update = std::time::Instant::now();
                        
                        loop {
                            match response.read(&mut buffer) {
                                Ok(0) => break,
                                Ok(n) => {
                                    if let Err(e) = std::io::Write::write_all(&mut file, &buffer[..n]) {
                                        log::error!("Failed to write to file: {}", e);
                                        let _ = event_tx.send(AppEvent::AssetDownloadError(format!("IO error: {}", e)));
                                        return;
                                    }
                                    downloaded += n;
                                    
                                    if last_update.elapsed().as_millis() > 200 {
                                        let percentage = (downloaded as f32 / total_size as f32 * 100.0) as u32;
                                        let _ = event_tx.send(AppEvent::AssetDownloadProgress(percentage.min(99)));
                                        last_update = std::time::Instant::now();
                                    }
                                }
                                Err(e) => {
                                    log::error!("Failed to read response: {}", e);
                                    let _ = event_tx.send(AppEvent::AssetDownloadError(format!("Download interrupted: {}", e)));
                                    return;
                                }
                            }
                        }
                        log::info!("Successfully downloaded {} bytes to {:?}", downloaded, config_asset_path);
                        let _ = event_tx.send(AppEvent::AssetDownloadProgress(100));
                        let _ = event_tx.send(AppEvent::AssetDownloaded("default.webm".to_string()));
                    }
                    Err(e) => {
                        log::error!("Failed to create file: {}", e);
                        let _ = event_tx.send(AppEvent::AssetDownloadError(format!("File creation error: {}", e)));
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to start download: {}", e);
                let _ = event_tx.send(AppEvent::AssetDownloadError(format!("Download start error: {}", e)));
            }
        }
    });
}

#[cfg(test)]
mod internal_tests {
    use super::*;

    #[test]
    fn test_github_release_parsing_logic() {
        let json = r#"{
            "tag_name": "v1.2.0",
            "body": "New version",
            "assets": [
                {"name": "pausecat.msi", "browser_download_url": "http://test.com/msi", "size": 1000},
                {"name": "readme.txt", "browser_download_url": "http://test.com/txt", "size": 500}
            ]
        }"#;
        
        let release = parse_github_release(json).unwrap();
        assert_eq!(release.tag_name, "v1.2.0");
        assert_eq!(release.assets.len(), 2);
        
        let info = parse_and_check_version(&release, "1.1.0").unwrap();
        assert!(info.available);
        assert_eq!(info.latest_version, "v1.2.0");
        
        let msi = find_msi_asset(&release).unwrap();
        assert_eq!(msi.name, "pausecat.msi");
    }

    #[test]
    fn test_no_update_available_logic() {
        let release = GithubRelease {
            tag_name: "v1.0.0".to_string(),
            body: "No changes".to_string(),
            assets: vec![],
        };
        let info = parse_and_check_version(&release, "1.0.0").unwrap();
        assert!(!info.available);
    }

    #[test]
    fn test_cleanup_updates_smoke() {
        cleanup_updates();
    }
}
