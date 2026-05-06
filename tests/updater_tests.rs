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
}
