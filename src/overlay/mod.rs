use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::UI::WindowsAndMessaging::*,
    Win32::Graphics::Gdi::*,
    Win32::UI::Input::KeyboardAndMouse::*,
    Win32::System::LibraryLoader::*,
};
use std::sync::mpsc::Sender;
use crate::events::AppEvent;

pub mod capture;
pub mod blur;
pub mod webview;

static mut OVERLAY_SENDER: Option<Sender<AppEvent>> = None;
static mut KEYBOARD_HOOK: HHOOK = HHOOK(std::ptr::null_mut());

pub struct OverlayWindow {
    hwnd: HWND,
}

impl OverlayWindow {
    pub fn new(sender: Sender<AppEvent>, width: i32, height: i32, blur_data: Vec<u8>) -> Result<Self> {
        unsafe {
            OVERLAY_SENDER = Some(sender.clone());
            
            let instance: HINSTANCE = GetModuleHandleW(None)?.into();
            let class_name = w!("PauseCatOverlayClass");

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(overlay_wnd_proc),
                hInstance: instance,
                lpszClassName: class_name,
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                ..Default::default()
            };

            RegisterClassExW(&wnd_class);

            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
                class_name,
                w!("PauseCat Overlay"),
                WS_POPUP | WS_VISIBLE,
                0, 0, width, height,
                None, None, Some(instance), Some(Box::into_raw(Box::new(blur_data)) as *mut _)
            )?;

            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_ALPHA);
            
            // Initialize WebView2 layer
            webview::WebViewLayer::init(hwnd, sender)?;

            // Set up emergency exit hook
            KEYBOARD_HOOK = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), Some(instance), 0)?;

            Ok(Self { hwnd })
        }
    }

    pub fn fade_in(&self) {
        unsafe {
            for alpha in (0..=255).step_by(15) {
                let _ = SetLayeredWindowAttributes(self.hwnd, COLORREF(0), alpha as u8, LWA_ALPHA);
                
                // Process pending messages to keep UI responsive and allow painting
                let mut msg = MSG::default();
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).into() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            let _ = SetLayeredWindowAttributes(self.hwnd, COLORREF(0), 255, LWA_ALPHA);
        }
    }
}

impl Drop for OverlayWindow {
    fn drop(&mut self) {
        unsafe {
            if !KEYBOARD_HOOK.0.is_null() {
                let _ = UnhookWindowsHookEx(KEYBOARD_HOOK);
                KEYBOARD_HOOK = HHOOK(std::ptr::null_mut());
            }
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn overlay_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let create_struct = lparam.0 as *const CREATESTRUCTW;
            let blur_data_ptr = (*create_struct).lpCreateParams;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, blur_data_ptr as isize);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1), // Handle in WM_PAINT
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            
            let blur_data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Vec<u8>;
            if !blur_data_ptr.is_null() {
                let blur_data = &*blur_data_ptr;
                let mut rect = RECT::default();
                let _ = GetClientRect(hwnd, &mut rect);
                let width = rect.right - rect.left;
                let height = rect.bottom - rect.top;

                let bmi = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: width,
                        biHeight: -height,
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB.0,
                        ..Default::default()
                    },
                    ..Default::default()
                };

                let _ = StretchDIBits(
                    hdc,
                    0, 0, width, height,
                    0, 0, width, height,
                    Some(blur_data.as_ptr() as *const _),
                    &bmi,
                    DIB_RGB_COLORS,
                    SRCCOPY
                );
            }

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_KEYDOWN => {
            if wparam.0 as u32 == VK_ESCAPE.0 as u32 {
                if let Some(ref sender) = OVERLAY_SENDER {
                    let _ = sender.send(AppEvent::UserDismissed);
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let blur_data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Vec<u8>;
            if !blur_data_ptr.is_null() {
                drop(Box::from_raw(blur_data_ptr));
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && wparam.0 as u32 == WM_KEYDOWN {
        let kbd = *(lparam.0 as *const KBDLLHOOKSTRUCT);
        let ctrl = GetAsyncKeyState(VK_CONTROL.0 as i32) as i16 & 0x8000u16 as i16 != 0;
        let shift = GetAsyncKeyState(VK_SHIFT.0 as i32) as i16 & 0x8000u16 as i16 != 0;
        
        if ctrl && shift && kbd.vkCode == VK_Q.0 as u32 {
            if let Some(ref sender) = OVERLAY_SENDER {
                let _ = sender.send(AppEvent::Quit);
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}
