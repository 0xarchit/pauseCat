use winreg::enums::*;
use winreg::RegKey;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::*;
use windows::core::{PCSTR, BOOL};
use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Threading::*;
use windows::Win32::System::ProcessStatus::*;
use std::collections::HashSet;

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

pub fn set_tray_menu_theme(is_dark: bool) {
    unsafe {
        let uxtheme = GetModuleHandleA(PCSTR("uxtheme.dll\0".as_ptr())).unwrap_or_default();
        if uxtheme.is_invalid() { return; }

        if let Some(set_preferred_app_mode) = GetProcAddress(uxtheme, PCSTR(135 as *const u8)) {
            let mode = if is_dark { 2i32 } else { 3i32 };
            let func: extern "system" fn(i32) -> i32 = std::mem::transmute(set_preferred_app_mode);
            func(mode);
        }
    }
}

pub fn get_running_apps() -> Vec<String> {
    let mut apps = HashSet::new();
    unsafe {
        let _ = EnumWindows(Some(enum_window_callback), LPARAM(&mut apps as *mut _ as isize));
    }
    let mut result: Vec<String> = apps.into_iter().collect();
    result.sort();
    result
}

unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let apps = &mut *(lparam.0 as *mut HashSet<String>);

    if IsWindowVisible(hwnd).as_bool() {
        let mut text = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut text);
        if len > 0 {
            if let Some(process_name) = get_process_name_from_hwnd(hwnd) {
                apps.insert(process_name);
            }
        }
    }
    true.into()
}

pub fn get_foreground_process_name() -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() { return None; }
        get_process_name_from_hwnd(hwnd)
    }
}

fn get_process_name_from_hwnd(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut process_id = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        if process_id == 0 { return None; }

        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, process_id).ok()?;
        let mut name = [0u16; 512];
        let len = GetModuleBaseNameW(handle, None, &mut name);
        let _ = CloseHandle(handle);

        if len > 0 {
            Some(String::from_utf16_lossy(&name[..len as usize]))
        } else {
            None
        }
    }
}
