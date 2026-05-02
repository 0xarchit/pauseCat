fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        // ID 1 is the default for application icons
        res.set_icon("assets/pauseCat.ico");
        res.set_manifest_file("assets/app.manifest");
        res.compile().unwrap();
    }
}
