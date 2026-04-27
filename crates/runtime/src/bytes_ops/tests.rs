use super::*;
use crate::stack::{alloc_test_stack, pop};

fn pop_string_bytes(stack: Stack) -> Vec<u8> {
    let (_, v) = unsafe { pop(stack) };
    match v {
        Value::String(s) => s.as_bytes().to_vec(),
        _ => panic!("expected String, got {:?}", v),
    }
}

#[test]
fn int_to_bytes_i32_be_positive() {
    unsafe {
        let stack = alloc_test_stack();
        let stack = push(stack, Value::Int(1000));
        let stack = patch_seq_int_to_bytes_i32_be(stack);
        assert_eq!(pop_string_bytes(stack), vec![0x00, 0x00, 0x03, 0xE8]);
    }
}

#[test]
fn int_to_bytes_i32_be_negative() {
    unsafe {
        let stack = alloc_test_stack();
        let stack = push(stack, Value::Int(-1));
        let stack = patch_seq_int_to_bytes_i32_be(stack);
        assert_eq!(pop_string_bytes(stack), vec![0xFF, 0xFF, 0xFF, 0xFF]);
    }
}

#[test]
fn int_to_bytes_i32_be_truncates_high_bits() {
    // i64 with high bits set; `as i32` keeps the low 32 bits.
    unsafe {
        let stack = alloc_test_stack();
        let stack = push(stack, Value::Int(0x0000_0001_DEAD_BEEFu64 as i64));
        let stack = patch_seq_int_to_bytes_i32_be(stack);
        assert_eq!(pop_string_bytes(stack), vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }
}

#[test]
fn float_to_bytes_f32_be_440_hz() {
    // 440.0 f32 big-endian = 0x43DC0000 — the canonical OSC fixture.
    unsafe {
        let stack = alloc_test_stack();
        let stack = push(stack, Value::Float(440.0));
        let stack = patch_seq_float_to_bytes_f32_be(stack);
        assert_eq!(pop_string_bytes(stack), vec![0x43, 0xDC, 0x00, 0x00]);
    }
}

#[test]
fn float_to_bytes_f32_be_zero() {
    unsafe {
        let stack = alloc_test_stack();
        let stack = push(stack, Value::Float(0.0));
        let stack = patch_seq_float_to_bytes_f32_be(stack);
        assert_eq!(pop_string_bytes(stack), vec![0x00, 0x00, 0x00, 0x00]);
    }
}

#[test]
fn float_to_bytes_f32_be_negative() {
    // -1.0 f32 big-endian = 0xBF800000
    unsafe {
        let stack = alloc_test_stack();
        let stack = push(stack, Value::Float(-1.0));
        let stack = patch_seq_float_to_bytes_f32_be(stack);
        assert_eq!(pop_string_bytes(stack), vec![0xBF, 0x80, 0x00, 0x00]);
    }
}
