use pausecat::overlay::blur::{blur, generate_gaussian_kernel};

#[test]
fn test_gaussian_kernel_sum() {
    let kernel = generate_gaussian_kernel(2.0);
    let sum: f32 = kernel.iter().sum();
    assert!((sum - 1.0).abs() < 1e-5);
}

#[test]
fn test_blur_small_image() {
    // 4x4 black image with one white pixel in the middle
    let width = 4;
    let height = 4;
    let mut data = vec![0u8; width * height * 4];
    
    // Set (2, 2) to white
    let idx = (2 * width + 2) * 4;
    data[idx] = 255;
    data[idx+1] = 255;
    data[idx+2] = 255;
    data[idx+3] = 255;

    let blurred = blur(&data, width, height, 1.0);
    
    assert_eq!(blurred.len(), data.len());
    
    // The white pixel should be spread to its neighbors
    let neighbor_idx = (2 * width + 1) * 4;
    assert!(blurred[neighbor_idx] > 0);
    assert!(blurred[idx] < 255);
}

#[test]
fn test_blur_edge_cases() {
    // Zero size
    let data = vec![];
    let blurred = blur(&data, 0, 0, 10.0);
    assert!(blurred.is_empty());
    
    // Large radius
    let data = vec![0u8; 100];
    let blurred = blur(&data, 5, 5, 100.0);
    assert_eq!(blurred.len(), 100);
}
