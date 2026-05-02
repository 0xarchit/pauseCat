#[test]
fn test_logging() {
    let mut path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("PauseCat");
    std::fs::create_dir_all(&path).unwrap();
    path.push("app.log");
    
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .unwrap();

    let target = Box::new(file);
    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Pipe(target))
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("Test logging");
    println!("Logged to {:?}", path);
}
