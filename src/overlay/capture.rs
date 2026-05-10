use windows::{
    core::*,
    Win32::Graphics::Gdi::*,
    Win32::UI::WindowsAndMessaging::*,
};

pub struct CapturedScreen {
    pub width: i32,
    pub height: i32,
    pub data: Vec<u8>, // BGRA pixels
}

use std::sync::{Mutex, OnceLock};

static CAPTURE_BUFFER: OnceLock<Mutex<Vec<u8>>> = OnceLock::new();

fn get_capture_buffer() -> &'static Mutex<Vec<u8>> {
    CAPTURE_BUFFER.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn capture_virtual_screen() -> Result<CapturedScreen> {
    unsafe {
        let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
        
        let capture_width = width / 4;
        let capture_height = height / 4;

        let h_screen_dc = GetDC(None);
        let h_memory_dc = CreateCompatibleDC(Some(h_screen_dc));
        let h_bitmap = CreateCompatibleBitmap(h_screen_dc, capture_width, capture_height);

        let h_old_obj = SelectObject(h_memory_dc, h_bitmap.into());

        SetStretchBltMode(h_memory_dc, HALFTONE);
        
        StretchBlt(
            h_memory_dc,
            0,
            0,
            capture_width,
            capture_height,
            Some(h_screen_dc),
            x,
            y,
            width,
            height,
            SRCCOPY,
        ).ok()?;

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: capture_width,
                biHeight: -capture_height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut lock = get_capture_buffer().lock().unwrap();
        let buffer_size = (capture_width * capture_height * 4) as usize;
        if lock.len() < buffer_size {
            lock.resize(buffer_size, 0);
        }

        GetDIBits(
            h_memory_dc,
            h_bitmap,
            0,
            capture_height as u32,
            Some(lock.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        let data = lock[..buffer_size].to_vec();

        // Cleanup
        let _ = SelectObject(h_memory_dc, h_old_obj);
        let _ = DeleteObject(h_bitmap.into());
        let _ = DeleteDC(h_memory_dc);
        ReleaseDC(None, h_screen_dc);

        Ok(CapturedScreen {
            width: capture_width,
            height: capture_height,
            data,
        })
    }
}

pub fn flush_buffer() {
    if let Some(lock) = CAPTURE_BUFFER.get() {
        if let Ok(mut b) = lock.lock() { 
            b.clear();
            b.shrink_to_fit(); 
        }
    }
}
