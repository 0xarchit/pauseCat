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

pub fn capture_virtual_screen() -> Result<CapturedScreen> {
    unsafe {
        let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        let h_screen_dc = GetDC(None);
        let h_memory_dc = CreateCompatibleDC(Some(h_screen_dc));
        let h_bitmap = CreateCompatibleBitmap(h_screen_dc, width, height);

        let h_old_obj = SelectObject(h_memory_dc, h_bitmap.into());

        BitBlt(
            h_memory_dc,
            0,
            0,
            width,
            height,
            Some(h_screen_dc),
            x,
            y,
            SRCCOPY,
        )?;

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // Negative for top-down DIB
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut data = vec![0u8; (width * height * 4) as usize];

        GetDIBits(
            h_screen_dc,
            h_bitmap,
            0,
            height as u32,
            Some(data.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // Cleanup
        let _ = SelectObject(h_memory_dc, h_old_obj);
        let _ = DeleteObject(h_bitmap.into());
        let _ = DeleteDC(h_memory_dc);
        ReleaseDC(None, h_screen_dc);

        Ok(CapturedScreen {
            width,
            height,
            data,
        })
    }
}
