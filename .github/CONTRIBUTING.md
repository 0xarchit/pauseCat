# Contributing to PauseCat

First off, thank you for considering contributing to PauseCat! It's people like you that make PauseCat a great tool for everyone.

## How Can I Contribute?

### Reporting Bugs
*   Check the [GitHub Issues](https://github.com/0xarchit/pauseCat/issues) to see if the bug has already been reported.
*   If not, open a new issue using the **Bug Report** template.
*   Include as many details as possible: your Windows version, a description of the bug, and steps to reproduce it.

### Suggesting Enhancements
*   Open a new issue using the **Feature Request** template.
*   Explain why the enhancement would be useful and how it should work.

### Pull Requests
1.  Fork the repository and create your branch from `main`.
2.  Follow the existing code style and conventions (Rust standard).
3.  Ensure your code builds without warnings or errors.
4.  Write a clear, concise commit message.
5.  Open a Pull Request with a detailed description of your changes.

## Development Setup

### Dependencies
- **Rust Stable**
- **WiX Toolset v4/v7** (for MSI builds)
- **WebView2 Runtime**

### Local Build
```powershell
$env:RUSTFLAGS="-C link-arg=/OPT:REF -C link-arg=/OPT:ICF"
cargo build --release
```

## Community

By contributing to this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).
