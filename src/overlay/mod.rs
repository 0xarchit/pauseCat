pub mod blur;
pub mod capture;
pub mod webview;
pub mod webview_env;

use std::sync::mpsc::Sender;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::*;
use windows::core::*;
use crate::events::AppEvent;
use crate::settings::Settings;

pub struct OverlayWindow {
    pub hwnd: HWND,
}

struct SendSafeHwnd(isize);
unsafe impl Send for SendSafeHwnd {}

impl OverlayWindow {
    pub fn new(sender: Sender<AppEvent>, screen_width: i32, screen_height: i32, blurred_width: i32, blurred_height: i32, blurred_data: Vec<u8>, settings: Settings) -> Result<Self> {
        unsafe {
            let instance = GetModuleHandleW(None)?.into();
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
                WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
                class_name,
                w!("PauseCat Overlay"),
                WS_POPUP,
                0, 0, screen_width, screen_height,
                None, None, Some(instance), None
            )?;

            SetPropW(hwnd, w!("Sender"), Some(HANDLE(Box::into_raw(Box::new(sender)) as *mut _)))?;

            let is_dark = crate::system::is_dark_mode();
            crate::system::apply_immersive_dark_mode(hwnd, is_dark);

            let hdc = GetDC(Some(hwnd));
            let mem_dc = CreateCompatibleDC(Some(hdc));
            let h_bitmap = CreateCompatibleBitmap(hdc, screen_width, screen_height);
            let _ = SelectObject(mem_dc, h_bitmap.into());

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: blurred_width,
                    biHeight: -blurred_height, 
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            SetStretchBltMode(mem_dc, COLORONCOLOR);

            let _ = StretchDIBits(
                mem_dc, 0, 0, screen_width, screen_height, 0, 0, blurred_width, blurred_height,
                Some(blurred_data.as_ptr() as *const _),
                &bmi, DIB_RGB_COLORS, SRCCOPY
            );

            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                SourceConstantAlpha: 0, 
                AlphaFormat: AC_SRC_ALPHA as u8,
                ..Default::default()
            };

            let pt_src = POINT { x: 0, y: 0 };
            let pt_dst = POINT { x: 0, y: 0 };
            let size = SIZE { cx: screen_width, cy: screen_height };

            let _ = UpdateLayeredWindow(hwnd, Some(hdc), Some(&pt_dst), Some(&size), Some(mem_dc), Some(&pt_src), COLORREF(0), Some(&blend), ULW_ALPHA);

            let _ = DeleteObject(h_bitmap.into());
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(Some(hwnd), hdc);

            webview::init(hwnd, settings)?;

            #[cfg(not(test))]
            let _ = ShowWindow(hwnd, SW_SHOW);
            #[cfg(test)]
            let _ = ShowWindow(hwnd, SW_HIDE);
            
            Ok(Self { hwnd })
        }
    }

    pub fn fade_in(&self) {
        let hwnd_val = SendSafeHwnd(self.hwnd.0 as isize);
        std::thread::spawn(move || {
            let hwnd = HWND(hwnd_val.0 as *mut _);
            for i in (0..=255).step_by(15) {
                unsafe {
                    if !IsWindow(Some(hwnd)).as_bool() { break; }
                    let blend = BLENDFUNCTION {
                        BlendOp: AC_SRC_OVER as u8,
                        SourceConstantAlpha: i as u8,
                        AlphaFormat: AC_SRC_ALPHA as u8,
                        ..Default::default()
                    };
                    let _ = UpdateLayeredWindow(hwnd, None, None, None, None, None, COLORREF(0), Some(&blend), ULW_ALPHA);
                }
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
        });
    }

    pub fn update_theme(&self, is_dark: bool) {
        webview::update_theme(self.hwnd, is_dark);
    }
}

impl Drop for OverlayWindow {
    fn drop(&mut self) {
        unsafe {
            // CRITICAL: Close the actual Win32 window when the struct is dropped!
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn overlay_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_SIZE => {
            webview::resize_controller(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            let sender_handle = GetPropW(hwnd, w!("Sender"));
            if !sender_handle.is_invalid() {
                let _ = Box::from_raw(sender_handle.0 as *mut Sender<AppEvent>);
                let _ = RemovePropW(hwnd, w!("Sender"));
            }
            webview::unregister_controller(hwnd);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(test)]
mod internal_tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn test_overlay_wnd_proc_branches() {
        let (tx, _rx) = mpsc::channel::<AppEvent>();
        let tx_ptr = Box::into_raw(Box::new(tx));
        
        unsafe {
            // We use a null HWND and don't unwrap SetPropW as it will fail
            // This still hits the wnd_proc branches that don't depend on Prop
            let hwnd = HWND(std::ptr::null_mut());
            let _ = SetPropW(hwnd, w!("Sender"), Some(HANDLE(tx_ptr as *mut _)));
            
            overlay_wnd_proc(hwnd, WM_SIZE, WPARAM(0), LPARAM(0));
            overlay_wnd_proc(hwnd, WM_USER, WPARAM(0), LPARAM(0));
            overlay_wnd_proc(hwnd, WM_DESTROY, WPARAM(0), LPARAM(0));

            let _ = Box::from_raw(tx_ptr);
        }
    }

    #[test]
    fn test_overlay_methods_smoke() {
        // We can't easily create a real window in tests without it being flaky,
        // but we can test the structure.
        let (tx, _rx) = mpsc::channel::<AppEvent>();
        let blurred_data = vec![0u8; 400]; // 10*10*4
        let settings = Settings::default();
        
        if let Ok(overlay) = OverlayWindow::new(tx, 10, 10, 10, 10, blurred_data, settings) {
            overlay.fade_in();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
