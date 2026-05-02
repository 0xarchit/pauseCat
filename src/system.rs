use winreg::enums::*;
use winreg::RegKey;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::*;
use windows::core::PCSTR;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::Win32::System::LibraryLoader::GetProcAddress;

pub fn is_dark_mode() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize") {
        let value: u32 = key.get_value("AppsUseLightTheme").unwrap_or(1);
        value == 0
    } else {
        false
    }
}

pub fn apply_immersive_dark_mode(hwnd: HWND, is_dark: bool) {
    unsafe {
        let value = if is_dark { 1i32 } else { 0i32 };
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &value as *const i32 as *const _,
            std::mem::size_of::<i32>() as u32,
        );
    }
}

/// This function uses undocumented Win32 APIs to force the native context menus 
/// (like the tray menu) to follow the dark mode preference.
pub fn set_tray_menu_theme(is_dark: bool) {
    unsafe {
        let uxtheme = GetModuleHandleA(PCSTR("uxtheme.dll\0".as_ptr())).unwrap_or_default();
        if uxtheme.is_invalid() { return; }

        // Ordinal 135 is SetPreferredAppMode
        // Mode 1 = Default, 2 = ForceDark, 3 = ForceLight
        if let Some(set_preferred_app_mode) = GetProcAddress(uxtheme, PCSTR(135 as *const u8)) {
            let mode = if is_dark { 2i32 } else { 3i32 };
            let func: extern "system" fn(i32) -> i32 = std::mem::transmute(set_preferred_app_mode);
            func(mode);
        }
    }
}
