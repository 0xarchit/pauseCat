pub fn generate_gaussian_kernel(sigma: f32) -> Vec<f32> {
    let radius = (sigma * 3.0).ceil() as usize;
    let size = radius * 2 + 1;
    let mut kernel = vec![0.0f32; size];
    let mut sum = 0.0f32;

    let two_sigma_sq = 2.0 * sigma * sigma;
    let root_two_pi_sigma = (two_sigma_sq * std::f32::consts::PI).sqrt();

    for i in 0..size {
        let x = i as f32 - radius as f32;
        kernel[i] = (-x * x / two_sigma_sq).exp() / root_two_pi_sigma;
        sum += kernel[i];
    }

    for val in kernel.iter_mut() {
        *val /= sum;
    }

    kernel
}

#[inline(always)]
fn get_pixel(data: &[u8], width: usize, _height: usize, x: usize, y: usize) -> (f32, f32, f32) {
    let idx = (y * width + x) * 4;
    (data[idx] as f32, data[idx+1] as f32, data[idx+2] as f32)
}

use std::sync::{Mutex, OnceLock};

static BLUR_TEMP_BUFFER: OnceLock<Mutex<Vec<u8>>> = OnceLock::new();
static BLUR_DST_BUFFER: OnceLock<Mutex<Vec<u8>>> = OnceLock::new();

fn get_temp_buffer() -> &'static Mutex<Vec<u8>> {
    BLUR_TEMP_BUFFER.get_or_init(|| Mutex::new(Vec::new()))
}

fn get_dst_buffer() -> &'static Mutex<Vec<u8>> {
    BLUR_DST_BUFFER.get_or_init(|| Mutex::new(Vec::new()))
}

static KERNEL_CACHE: OnceLock<Mutex<Option<(f32, Vec<f32>)>>> = OnceLock::new();

fn get_kernel(sigma: f32) -> Vec<f32> {
    let mut lock = KERNEL_CACHE.get_or_init(|| Mutex::new(None)).lock().unwrap();
    if let Some((cached_sigma, ref kernel)) = *lock {
        if (cached_sigma - sigma).abs() < f32::EPSILON {
            return kernel.clone();
        }
    }
    let kernel = generate_gaussian_kernel(sigma);
    *lock = Some((sigma, kernel.clone()));
    kernel
}

pub fn blur(src: &[u8], width: usize, height: usize, sigma: f32) -> Vec<u8> {
    let kernel = get_kernel(sigma);
    let radius = kernel.len() / 2;
    
    let mut temp_lock = get_temp_buffer().lock().unwrap();
    let mut dst_lock = get_dst_buffer().lock().unwrap();
    
    if temp_lock.len() < src.len() { temp_lock.resize(src.len(), 0); }
    if dst_lock.len() < src.len() { dst_lock.resize(src.len(), 0); }

    // Horizontal pass
    for y in 0..height {
        for x in 0..width {
            let (mut r, mut g, mut b) = (0.0f32, 0.0f32, 0.0f32);
            for (k, &weight) in kernel.iter().enumerate() {
                let ix = (x as isize + k as isize - radius as isize).clamp(0, width as isize - 1) as usize;
                let (pr, pg, pb) = get_pixel(src, width, height, ix, y);
                r += pr * weight;
                g += pg * weight;
                b += pb * weight;
            }
            let idx = (y * width + x) * 4;
            temp_lock[idx] = r as u8;
            temp_lock[idx+1] = g as u8;
            temp_lock[idx+2] = b as u8;
            temp_lock[idx+3] = 255;
        }
    }

    // Vertical pass
    for y in 0..height {
        for x in 0..width {
            let (mut r, mut g, mut b) = (0.0f32, 0.0f32, 0.0f32);
            for (k, &weight) in kernel.iter().enumerate() {
                let iy = (y as isize + k as isize - radius as isize).clamp(0, height as isize - 1) as usize;
                let (pr, pg, pb) = get_pixel(&temp_lock, width, height, x, iy);
                r += pr * weight;
                g += pg * weight;
                b += pb * weight;
            }
            let idx = (y * width + x) * 4;
            dst_lock[idx] = r as u8;
            dst_lock[idx+1] = g as u8;
            dst_lock[idx+2] = b as u8;
            dst_lock[idx+3] = 255;
        }
    }

    dst_lock[..src.len()].to_vec()
}

pub fn flush_buffers() {
    if let Some(lock) = BLUR_TEMP_BUFFER.get() {
        if let Ok(mut b) = lock.lock() { 
            b.clear();
            b.shrink_to_fit(); 
        }
    }
    if let Some(lock) = BLUR_DST_BUFFER.get() {
        if let Ok(mut b) = lock.lock() { 
            b.clear();
            b.shrink_to_fit(); 
        }
    }
}
