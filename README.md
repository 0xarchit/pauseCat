# 🐾 PauseCat

**The Professional Break Reminder for Windows.**

PauseCat is a high-performance, lightweight Windows background application designed to reduce computer fatigue and improve productivity by encouraging periodic breaks. It provides a full-screen immersive reminder experience with beautiful glassmorphism visuals and intelligent system integration.

[![Status](https://img.shields.io/badge/Status-Active%20Development-000000.svg?style=for-the-badge&logo=rocket&logoColor=white&labelColor=000000&color=000000)](https://github.com/0xarchit/pauseCat/pulse)
[![License](https://img.shields.io/badge/License-Apache%202.0-000000.svg?style=for-the-badge&logo=apache&logoColor=white&labelColor=000000&color=000000)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-000000.svg?style=for-the-badge&logo=rust&logoColor=white&labelColor=000000&color=000000)](https://rust-lang.org)

---

## ✨ Features

- **🚀 Zero-Lag Overlay:** Uses advanced pre-capture and pre-blur technology to trigger break reminders instantly with no white flicker.
- **💎 Glassmorphism UI:** Stunning, translucent UI bubble with smooth floating animations and Inter typography.
- **🎵 Smart Media Control:** Automatically pauses your music (Spotify, YouTube, etc.) when a break starts and resumes it when you're done.
- **⚙️ Fluent Settings:** Native-feeling configuration panel to customize work/break durations, break mode (Soft/Hard), and auto-start.
- **🎨 Custom Media:** Support for user-selected images, videos (WebM/MP4), and GIFs for the break background.
- **📦 Tiny Footprint:** Optimized Rust binary (~1.2 MB) with shared WebView2 environments for low RAM usage.
- **🛡️ Secure & Private:** All settings and logs are stored locally in `%APPDATA%`. No data ever leaves your machine.

---

## 🛠 Installation

1. Download the latest `pausecat_installer.msi` from the [Releases](https://github.com/0xarchit/pauseCat/releases) page.
2. Run the installer and follow the professional wizard.
3. (Optional) Check "Launch PauseCat now" to start your first session immediately.

---

## 💻 Development

### Prerequisites
- [Rust](https://rustup.rs/) (Stable)
- [WiX Toolset v4/v7](https://wixtoolset.org/) (for building the installer)
- [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

### Build the Optimized Binary
```powershell
$env:RUSTFLAGS="-C link-arg=/OPT:REF -C link-arg=/OPT:ICF"
cargo build --release
```

### Build the Professional Installer
```powershell
wix build wix\main.wxs -ext WixToolset.UI.wixext -ext WixToolset.Util.wixext -o target\release\pausecat_installer.msi
```

---

## 🤝 Contributing

We welcome contributions! Please see our [CONTRIBUTING.md](.github/CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](.github/CODE_OF_CONDUCT.md) for details on how to get started.

## 🛡 Security

If you discover a security vulnerability, please refer to our [SECURITY.md](.github/SECURITY.md).

## 📄 License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

### Acknowledgements
The `assets/default.webm` file included in this project was sourced from the internet. It is intended for demonstration purposes only and is not intended to infringe upon any existing copyrights. If you are the owner of this content and wish to have it removed or credited differently, please contact us.

---
*Built with ❤️ in Rust for a healthier digital life.*
