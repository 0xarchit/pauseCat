#[cfg(test)]
mod tests {
    use pausecat::updater::*;

    #[test]
    fn test_updater_integration_smoke() {
        // Just verify types and module accessibility
        let _ = UpdateInfo { available: false, latest_version: "1.0.0".to_string(), changelog: "".to_string() };
    }
}
