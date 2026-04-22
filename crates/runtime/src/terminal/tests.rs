use super::*;

#[test]
fn test_terminal_size() {
    // Should return reasonable values (not panic)
    let (width, height) = get_terminal_size();
    assert!(width > 0);
    assert!(height > 0);
}

#[test]
fn test_terminal_width_stack() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = terminal_width(stack);
        let (_, value) = pop(stack);
        match value {
            Value::Int(w) => assert!(w > 0),
            _ => panic!("expected Int"),
        }
    }
}

#[test]
fn test_terminal_height_stack() {
    unsafe {
        let stack = crate::stack::alloc_test_stack();
        let stack = terminal_height(stack);
        let (_, value) = pop(stack);
        match value {
            Value::Int(h) => assert!(h > 0),
            _ => panic!("expected Int"),
        }
    }
}

#[test]
fn test_raw_mode_toggle() {
    // Test that we can toggle raw mode without crashing
    // Note: This may not work in all test environments
    enable_raw_mode();
    disable_raw_mode();
    // Should be back to normal
    assert!(!RAW_MODE_ENABLED.load(Ordering::SeqCst));
}
