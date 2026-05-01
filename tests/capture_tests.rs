use pausecat::overlay::capture::capture_virtual_screen;

#[test]
fn test_capture_virtual_screen() {
    let result = capture_virtual_screen();
    assert!(result.is_ok());
    
    let captured = result.unwrap();
    assert!(captured.width > 0);
    assert!(captured.height > 0);
    assert_eq!(captured.data.len(), (captured.width * captured.height * 4) as usize);
}
