#[cfg(test)]
mod tests {
    use pausecat::updater::{GithubRelease, parse_and_check_version};

    #[test]
    fn test_version_comparison_available() {
        let release = GithubRelease {
            tag_name: "v2.0.0".to_string(),
            assets: vec![],
            body: "Great new features".to_string(),
        };
        
        let result = parse_and_check_version(&release, "1.0.1").unwrap();
        assert!(result.available);
        assert_eq!(result.latest_version, "v2.0.0");
    }

    #[test]
    fn test_version_comparison_unavailable() {
        let release = GithubRelease {
            tag_name: "v1.0.0".to_string(),
            assets: vec![],
            body: "Old version".to_string(),
        };
        
        let result = parse_and_check_version(&release, "1.0.1").unwrap();
        assert!(!result.available);
    }

    #[test]
    fn test_version_comparison_same() {
        let release = GithubRelease {
            tag_name: "v1.0.1".to_string(),
            assets: vec![],
            body: "Current version".to_string(),
        };
        
        let result = parse_and_check_version(&release, "1.0.1").unwrap();
        assert!(!result.available);
    }

    #[test]
    fn test_version_parsing_error() {
        let release = GithubRelease {
            tag_name: "invalid-version".to_string(),
            assets: vec![],
            body: "".to_string(),
        };
        
        let result = parse_and_check_version(&release, "1.0.1");
        assert!(result.is_err());
    }

    #[test]
    fn test_version_semver_edge_cases() {
        let mut release = GithubRelease {
            tag_name: "v1.0.2-alpha".to_string(),
            assets: vec![],
            body: "".to_string(),
        };
        
        // 1.0.2-alpha < 1.0.1 (Wait, is it? No, alpha is usually lower than stable of same version but higher than lower stable)
        // SemVer: 1.0.2-alpha > 1.0.1
        let result = parse_and_check_version(&release, "1.0.1").unwrap();
        assert!(result.available);
        
        release.tag_name = "1.1.0".to_string();
        let result = parse_and_check_version(&release, "1.0.99").unwrap();
        assert!(result.available);
    }

    #[test]
    fn test_asset_filtering() {
        use pausecat::updater::{GithubAsset, find_msi_asset};
        
        let release = GithubRelease {
            tag_name: "v1.0.0".to_string(),
            body: "".to_string(),
            assets: vec![
                GithubAsset { name: "cat.zip".to_string(), browser_download_url: "".to_string(), size: 100 },
                GithubAsset { name: "installer.msi".to_string(), browser_download_url: "found-it".to_string(), size: 200 },
                GithubAsset { name: "readme.txt".to_string(), browser_download_url: "".to_string(), size: 50 },
            ],
        };
        
        let asset = find_msi_asset(&release).expect("Should find MSI");
        assert_eq!(asset.browser_download_url, "found-it");
    }

    #[test]
    fn test_asset_filtering_multiple() {
        use pausecat::updater::{GithubAsset, find_msi_asset};
        let release = GithubRelease {
            tag_name: "v1".to_string(),
            body: "".to_string(),
            assets: vec![
                GithubAsset { name: "1.zip".to_string(), browser_download_url: "u1".to_string(), size: 1 },
                GithubAsset { name: "2.MSI".to_string(), browser_download_url: "u2".to_string(), size: 2 },
                GithubAsset { name: "3.exe".to_string(), browser_download_url: "u3".to_string(), size: 3 },
            ],
        };
        let asset = find_msi_asset(&release).unwrap();
        assert_eq!(asset.browser_download_url, "u2");
    }

    #[test]
    fn test_asset_filtering_none() {
        use pausecat::updater::find_msi_asset;
        let release = GithubRelease {
            tag_name: "v1".to_string(),
            body: "".to_string(),
            assets: vec![],
        };
        assert!(find_msi_asset(&release).is_none());
    }

    #[test]
    fn test_cleanup_logic() {
        use pausecat::updater::cleanup_updates;
        use pausecat::settings::Settings;
        use std::fs;

        let mut update_dir = Settings::get_config_dir();
        update_dir.push("Updates");
        if update_dir.exists() {
            fs::remove_dir_all(&update_dir).unwrap();
        }
        fs::create_dir_all(&update_dir).unwrap();
        fs::write(update_dir.join("dummy.tmp"), "test").unwrap();
        
        assert!(update_dir.exists());
        cleanup_updates();
        assert!(!update_dir.exists());
    }
}
