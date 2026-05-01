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

pub fn blur(src: &[u8], width: usize, height: usize, sigma: f32) -> Vec<u8> {
    let kernel = generate_gaussian_kernel(sigma);
    let radius = kernel.len() / 2;
    let mut temp = vec![0u8; src.len()];
    let mut dst = vec![0u8; src.len()];

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
            temp[idx] = r as u8;
            temp[idx+1] = g as u8;
            temp[idx+2] = b as u8;
            temp[idx+3] = 255;
        }
    }

    // Vertical pass
    for y in 0..height {
        for x in 0..width {
            let (mut r, mut g, mut b) = (0.0f32, 0.0f32, 0.0f32);
            for (k, &weight) in kernel.iter().enumerate() {
                let iy = (y as isize + k as isize - radius as isize).clamp(0, height as isize - 1) as usize;
                let (pr, pg, pb) = get_pixel(&temp, width, height, x, iy);
                r += pr * weight;
                g += pg * weight;
                b += pb * weight;
            }
            let idx = (y * width + x) * 4;
            dst[idx] = r as u8;
            dst[idx+1] = g as u8;
            dst[idx+2] = b as u8;
            dst[idx+3] = 255;
        }
    }

    dst
}
