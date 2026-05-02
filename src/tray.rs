use std::sync::mpsc::Sender;
use crate::events::AppEvent;
use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::UI::WindowsAndMessaging::*,
    Win32::UI::Shell::*,
    Win32::System::LibraryLoader::*,
};

const WM_TRAY_ICON: u32 = WM_APP + 1;
const ID_TRAY_ICON: u32 = 1;
const ID_MENU_PAUSE: usize = 1001;
const ID_MENU_SETTINGS: usize = 1002;
const ID_MENU_EXIT: usize = 1003;

pub struct TrayIcon {
    hwnd: HWND,
}

impl TrayIcon {
    pub fn new(sender: Sender<AppEvent>) -> Result<Self> {
        unsafe {
            let instance: HINSTANCE = GetModuleHandleW(None)?.into();
            let class_name = w!("PauseCatTrayClass");

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(wnd_proc),
                hInstance: instance,
                lpszClassName: class_name,
                ..Default::default()
            };

            if RegisterClassExW(&wnd_class) == 0 {
                return Err(Error::from_hresult(HRESULT(-1))); 
            }

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name,
                w!("PauseCat Tray"),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT,
                None, None, Some(instance), Some(Box::into_raw(Box::new(sender)) as *mut _)
            )?;

            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: ID_TRAY_ICON,
                uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
                uCallbackMessage: WM_TRAY_ICON,
                hIcon: LoadIconW(None, IDI_APPLICATION)?,
                ..Default::default()
            };
            
            Self::copy_tip(&mut nid.szTip, "PauseCat");

            let _ = Shell_NotifyIconW(NIM_ADD, &nid);

            Ok(Self { hwnd })
        }
    }

    pub fn set_paused(&self, paused: bool) {
        unsafe {
            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd,
                uID: ID_TRAY_ICON,
                uFlags: NIF_TIP,
                ..Default::default()
            };
            let tip = if paused { "PauseCat (Paused)" } else { "PauseCat" };
            Self::copy_tip(&mut nid.szTip, tip);

            let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
    }

    fn copy_tip(dest: &mut [u16; 128], src: &str) {
        let wide: Vec<u16> = src.encode_utf16().collect();
        let len = wide.len().min(127);
        dest[..len].copy_from_slice(&wide[..len]);
        dest[len] = 0;
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        unsafe {
            let nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd,
                uID: ID_TRAY_ICON,
                ..Default::default()
            };
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let create_struct = lparam.0 as *const CREATESTRUCTW;
            let sender = (*create_struct).lpCreateParams;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, sender as isize);
            LRESULT(0)
        }
        WM_TRAY_ICON => {
            let event = lparam.0 as u32;
            if event == WM_RBUTTONUP || event == NIN_SELECT {
                show_context_menu(hwnd);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let sender_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Sender<AppEvent>;
            if !sender_ptr.is_null() {
                let sender = &*sender_ptr;
                match wparam.0 as usize {
                    ID_MENU_PAUSE => { let _ = sender.send(AppEvent::TogglePause); }
                    ID_MENU_SETTINGS => { let _ = sender.send(AppEvent::OpenSettings); }
                    ID_MENU_EXIT => { let _ = sender.send(AppEvent::Quit); }
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let sender_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Sender<AppEvent>;
            if !sender_ptr.is_null() {
                drop(Box::from_raw(sender_ptr));
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn show_context_menu(hwnd: HWND) {
    let menu = match CreatePopupMenu() {
        Ok(m) => m,
        Err(_) => return,
    };
    
    let _ = AppendMenuW(menu, MF_STRING, ID_MENU_PAUSE, w!("Pause / Resume"));
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
    let _ = AppendMenuW(menu, MF_STRING, ID_MENU_SETTINGS, w!("Settings..."));
    let _ = AppendMenuW(menu, MF_STRING, ID_MENU_EXIT, w!("Exit"));

    let mut pos = POINT::default();
    let _ = GetCursorPos(&mut pos);

    let _ = SetForegroundWindow(hwnd);
    let _ = TrackPopupMenu(menu, TPM_RIGHTBUTTON, pos.x, pos.y, Some(0), hwnd, None);
    let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
    let _ = DestroyMenu(menu);
}
