<div align="center">
  <img src="assets/pauseCat-banner.jpeg" alt="PauseCat Banner" width="80%">

  # 🐾 PauseCat
  **The High-Performance, SaaS-Grade Break Reminder for Windows.**  
  [![Status](https://img.shields.io/badge/Status-Active%20Development-000000.svg?style=for-the-badge&logo=rocket&logoColor=white&labelColor=000000&color=000000)](https://github.com/0xarchit/pauseCat/pulse)
  [![License](https://img.shields.io/badge/License-Apache%202.0-000000.svg?style=for-the-badge&logo=apache&logoColor=white&labelColor=000000&color=000000)](LICENSE)
  [![Rust](https://img.shields.io/badge/Rust-1.94+-000000.svg?style=for-the-badge&logo=rust&logoColor=white&labelColor=000000&color=000000)](https://rust-lang.org)
  [![Coverage](https://img.shields.io/badge/Coverage-57.66%25-000000.svg?style=for-the-badge&logo=codecov&logoColor=white&labelColor=000000&color=000000)](https://github.com/0xarchit/pauseCat)

  ---
</div>

## 🌟 Overview

PauseCat is more than just a timer. It's a professional-grade Windows utility engineered in **Rust** and powered by **WebView2** to provide a seamless, non-intrusive break experience. Designed for high-performance workstations, it ensures you stay healthy without interrupting your flow with clunky or flickering overlays.

<div align="center">
  <img src="assets/pauseCat-break.png" alt="PauseCat Break Interface" width="60%">
  <br>
  <em>The immersive, glassmorphic break interface.</em>
</div>

---

## 🚀 Key Features

### 💎 Elite Visuals & UX
- **Ultra-Fast Overlay:** Advanced pre-capture and Gaussian blur engine ensures reminders trigger with zero lag and no white flicker.
- **Pro Designer UI:** Precision sliders for bubble size, glass translucency, and positioning with real-time live preview.
- **Hardened Native UX:** 100% protection against accidental zoom gestures and hidden native scrollbars for a pure application feel.

### 🧠 Intelligent Logic
- **Smart Session Guard:** Native awareness of Windows Lock (Win+L) and System Sleep states. The timer pauses and resumes exactly when you are at your desk.
- **Intelligent Media Sync:** Native SMTC integration detects if music/video is playing and only pauses/resumes your media if it was active before the break.

### 🛠️ Engineering Excellence
- **Lightweight Footprint:** Optimized Rust binary (< 2.5 MB) with shared WebView2 environments for professional resource management.
- **Auto-Update Cycle:** Fully automated self-update system that downloads, installs, and relaunches the application from GitHub Releases.
- **Security First:** Automated CodeQL v4 scanning and pinned GitHub Actions for total supply-chain security.

---

## 💻 Development Workflow

### 🛠 Build Standards
To achieve the optimal binary size and performance, always use the following MSVC linker flags:
```powershell
$env:RUSTFLAGS="-C link-arg=/OPT:REF -C link-arg=/OPT:ICF"
cargo build --release
```

### 🌿 Git Policy
- **Feature Isolation:** One branch per feature: `feature/<description>` or `fix/<description>`.
- **Conventional Commits:** Use `feat:`, `fix:`, `refactor:`, `chore:`, `test:`.
- **Main is Read-Only:** Never commit directly to `main`.

---

## 🏗 Installation

1. Download the latest `PauseCat_Installer.msi` from the [Releases](https://github.com/0xarchit/pauseCat/releases) page.
2. The WiX installer handles everything: installation, autostart registry, and WebView2 runtime checks.
3. Once installed, PauseCat lives in your **System Tray**.

---

## 🤝 Contributing

We welcome contributions! Please see our [CONTRIBUTING.md](.github/CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](.github/CODE_OF_CONDUCT.md) for details.

---
<div align="center">
  Built with ❤️ in Rust for a healthier digital life.
</div>
