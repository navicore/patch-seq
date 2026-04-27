use super::*;
use seq_core::stack::alloc_stack;

#[test]
fn test_gzip_roundtrip() {
    let stack = alloc_stack();
    let stack = unsafe {
        push(
            stack,
            Value::String(global_string("hello world".to_string())),
        )
    };

    // Compress
    let stack = unsafe { patch_seq_compress_gzip(stack) };

    // Check compress success flag
    let (stack, compress_success) = unsafe { pop(stack) };
    assert_eq!(compress_success, Value::Bool(true));

    // Decompress
    let stack = unsafe { patch_seq_compress_gunzip(stack) };

    // Check decompress success flag
    let (stack, success) = unsafe { pop(stack) };
    assert_eq!(success, Value::Bool(true));

    let (_, result) = unsafe { pop(stack) };
    if let Value::String(s) = result {
        assert_eq!(s.as_str_or_empty(), "hello world");
    } else {
        panic!("expected string");
    }
}

#[test]
fn test_gzip_level() {
    let stack = alloc_stack();
    let stack = unsafe {
        push(
            stack,
            Value::String(global_string("hello world".to_string())),
        )
    };
    let stack = unsafe { push(stack, Value::Int(9)) };

    // Compress with max level
    let stack = unsafe { patch_seq_compress_gzip_level(stack) };

    // Check compress success flag
    let (stack, compress_success) = unsafe { pop(stack) };
    assert_eq!(compress_success, Value::Bool(true));

    // Decompress
    let stack = unsafe { patch_seq_compress_gunzip(stack) };
    let (stack, success) = unsafe { pop(stack) };
    assert_eq!(success, Value::Bool(true));

    let (_, result) = unsafe { pop(stack) };
    if let Value::String(s) = result {
        assert_eq!(s.as_str_or_empty(), "hello world");
    } else {
        panic!("expected string");
    }
}

#[test]
fn test_zstd_roundtrip() {
    let stack = alloc_stack();
    let stack = unsafe {
        push(
            stack,
            Value::String(global_string("hello world".to_string())),
        )
    };

    // Compress
    let stack = unsafe { patch_seq_compress_zstd(stack) };

    // Check compress success flag
    let (stack, compress_success) = unsafe { pop(stack) };
    assert_eq!(compress_success, Value::Bool(true));

    // Decompress
    let stack = unsafe { patch_seq_compress_unzstd(stack) };

    // Check decompress success flag
    let (stack, success) = unsafe { pop(stack) };
    assert_eq!(success, Value::Bool(true));

    let (_, result) = unsafe { pop(stack) };
    if let Value::String(s) = result {
        assert_eq!(s.as_str_or_empty(), "hello world");
    } else {
        panic!("expected string");
    }
}

#[test]
fn test_zstd_level() {
    let stack = alloc_stack();
    let stack = unsafe {
        push(
            stack,
            Value::String(global_string("hello world".to_string())),
        )
    };
    let stack = unsafe { push(stack, Value::Int(19)) };

    // Compress with high level
    let stack = unsafe { patch_seq_compress_zstd_level(stack) };

    // Check compress success flag
    let (stack, compress_success) = unsafe { pop(stack) };
    assert_eq!(compress_success, Value::Bool(true));

    // Decompress
    let stack = unsafe { patch_seq_compress_unzstd(stack) };
    let (stack, success) = unsafe { pop(stack) };
    assert_eq!(success, Value::Bool(true));

    let (_, result) = unsafe { pop(stack) };
    if let Value::String(s) = result {
        assert_eq!(s.as_str_or_empty(), "hello world");
    } else {
        panic!("expected string");
    }
}

#[test]
fn test_gunzip_invalid_base64() {
    let stack = alloc_stack();
    let stack = unsafe {
        push(
            stack,
            Value::String(global_string("not valid base64!!!".to_string())),
        )
    };

    let stack = unsafe { patch_seq_compress_gunzip(stack) };
    let (_, success) = unsafe { pop(stack) };
    assert_eq!(success, Value::Bool(false));
}

#[test]
fn test_gunzip_invalid_gzip() {
    let stack = alloc_stack();
    // Valid base64 but not gzip data
    let stack = unsafe {
        push(
            stack,
            Value::String(global_string("aGVsbG8gd29ybGQ=".to_string())),
        )
    };

    let stack = unsafe { patch_seq_compress_gunzip(stack) };
    let (_, success) = unsafe { pop(stack) };
    assert_eq!(success, Value::Bool(false));
}

#[test]
fn test_unzstd_invalid() {
    let stack = alloc_stack();
    // Valid base64 but not zstd data
    let stack = unsafe {
        push(
            stack,
            Value::String(global_string("aGVsbG8gd29ybGQ=".to_string())),
        )
    };

    let stack = unsafe { patch_seq_compress_unzstd(stack) };
    let (_, success) = unsafe { pop(stack) };
    assert_eq!(success, Value::Bool(false));
}

#[test]
fn test_empty_string() {
    let stack = alloc_stack();
    let stack = unsafe { push(stack, Value::String(global_string(String::new()))) };

    // Compress empty string
    let stack = unsafe { patch_seq_compress_gzip(stack) };

    // Check compress success flag
    let (stack, compress_success) = unsafe { pop(stack) };
    assert_eq!(compress_success, Value::Bool(true));

    // Decompress
    let stack = unsafe { patch_seq_compress_gunzip(stack) };
    let (stack, success) = unsafe { pop(stack) };
    assert_eq!(success, Value::Bool(true));

    let (_, result) = unsafe { pop(stack) };
    if let Value::String(s) = result {
        assert_eq!(s.as_str_or_empty(), "");
    } else {
        panic!("expected string");
    }
}

#[test]
fn test_large_data() {
    let stack = alloc_stack();
    let large_data = "x".repeat(10000);
    let stack = unsafe { push(stack, Value::String(global_string(large_data.clone()))) };

    // Compress
    let stack = unsafe { patch_seq_compress_zstd(stack) };

    // Check compress success flag
    let (stack, compress_success) = unsafe { pop(stack) };
    assert_eq!(compress_success, Value::Bool(true));

    // Decompress
    let stack = unsafe { patch_seq_compress_unzstd(stack) };
    let (stack, success) = unsafe { pop(stack) };
    assert_eq!(success, Value::Bool(true));

    let (_, result) = unsafe { pop(stack) };
    if let Value::String(s) = result {
        assert_eq!(s.as_str_or_empty(), large_data);
    } else {
        panic!("expected string");
    }
}
