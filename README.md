# PauseCat

PauseCat is a lightweight Windows background application built in Rust that encourages users to take periodic breaks through a full-screen immersive reminder experience. The reminder uses background blur and stunning glassmorphism visuals to reduce fatigue and improve productivity.

## Tech Stack
- **Language:** Rust
- **System APIs:** Win32 (via windows-rs)
- **UI Rendering:** WebView2 (Fluent Design / Glassmorphism)

## Build Prerequisites
- **Rust stable toolchain**
- **WebView2 runtime** (pre-installed on Windows 10/11)
- **WiX Toolset v3** (if you want to build the installer)

## Building the App
To build the optimized release binary:
```powershell
$env:RUSTFLAGS="-C link-arg=/OPT:REF -C link-arg=/OPT:ICF"
cargo build --release
```

## Creating the Installer (Installation Dialog)
PauseCat uses the WiX Toolset to create a professional Windows installer (.msi).
1. Install [WiX Toolset v3](https://wixtoolset.org/releases/).
2. Install the `cargo-wix` helper: `cargo install cargo-wix`.
3. Run: `cargo wix --nocapture`.
This will generate an `.msi` file in `target/wix/`. Running this MSI will show the **Installation Dialog**.

## Usage
Once installed or running, PauseCat lives in your **System Tray** (🐾 icon).
- **Right-click** the tray icon to access Settings, Pause/Resume, or Exit.
- **Hard Mode** prevents you from skipping breaks for maximum focus.
- **Custom Media:** You can select your own images or videos to display during breaks in the settings panel.
