use std::fs;
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

pub fn download_and_install(event_tx: Sender<AppEvent>) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("PauseCat-Updater-v1")
        .build()?;

    let release: GithubRelease = client.get(GITHUB_API_URL).send()?.json()?;
    let asset = release.assets.iter()
        .find(|a| a.name.to_lowercase().ends_with(".msi"))
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

    std::process::exit(0);
}

pub fn cleanup_updates() {
    let mut update_dir = Settings::get_config_dir();
    update_dir.push("Updates");
    if update_dir.exists() {
        let _ = fs::remove_dir_all(&update_dir);
    }
}
