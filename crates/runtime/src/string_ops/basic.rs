//! Basic string operations: length, byte-length, empty-check, equality, concat.

use crate::seqstring::global_bytes;
use crate::stack::{Stack, pop, push};
use crate::value::Value;

/// # Safety
/// Stack must have the expected values on top for this operation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_empty(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "string_empty: stack is empty");

    let (stack, value) = unsafe { pop(stack) };

    match value {
        Value::String(s) => {
            // Byte-clean: empty iff there are no bytes, regardless of UTF-8.
            let is_empty = s.as_bytes().is_empty();
            unsafe { push(stack, Value::Bool(is_empty)) }
        }
        _ => panic!("string_empty: expected String on stack"),
    }
}

/// Check if a string contains a substring
///
/// Stack effect: ( str substring -- bool )
///
/// # Safety
/// Stack must have two String values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_concat(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "string_concat: stack is empty");

    let (stack, str2_val) = unsafe { pop(stack) };
    assert!(!stack.is_null(), "string_concat: need two strings");
    let (stack, str1_val) = unsafe { pop(stack) };

    match (str1_val, str2_val) {
        (Value::String(s1), Value::String(s2)) => {
            // Byte-clean concat: catenate the underlying byte buffers.
            // OSC payload assembly, binary file building, MessagePack,
            // anything that needs to glue arbitrary bytes together
            // depends on this not going through `&str`.
            let mut result = Vec::with_capacity(s1.as_bytes().len() + s2.as_bytes().len());
            result.extend_from_slice(s1.as_bytes());
            result.extend_from_slice(s2.as_bytes());
            unsafe { push(stack, Value::String(global_bytes(result))) }
        }
        _ => panic!("string_concat: expected two strings on stack"),
    }
}

/// Get the length of a string in Unicode characters (code points)
///
/// Stack effect: ( str -- int )
///
/// Note: This returns character count, not byte count.
/// For UTF-8 byte length (e.g., HTTP Content-Length), use `string-byte-length`.
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_length(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "string_length: stack is empty");

    let (stack, str_val) = unsafe { pop(stack) };

    match str_val {
        Value::String(s) => {
            let len = s.as_str_or_empty().chars().count() as i64;
            unsafe { push(stack, Value::Int(len)) }
        }
        _ => panic!("string_length: expected String on stack"),
    }
}

/// Get the byte length of a string (UTF-8 encoded)
///
/// Stack effect: ( str -- int )
///
/// Use this for HTTP Content-Length headers and buffer allocation.
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_byte_length(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "string_byte_length: stack is empty");

    let (stack, str_val) = unsafe { pop(stack) };

    match str_val {
        Value::String(s) => {
            // Byte-clean: count of underlying bytes, no UTF-8 validation.
            let len = s.as_bytes().len() as i64;
            unsafe { push(stack, Value::Int(len)) }
        }
        _ => panic!("string_byte_length: expected String on stack"),
    }
}

/// Get the Unicode code point at a character index
///
/// Stack effect: ( str int -- int )
///
/// Returns the code point value at the given character index.
/// Returns -1 if index is out of bounds.
///
/// # Safety
/// Stack must have a String and Int on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_equal(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "string_equal: stack is empty");

    let (stack, str2_val) = unsafe { pop(stack) };
    assert!(!stack.is_null(), "string_equal: need two strings");
    let (stack, str1_val) = unsafe { pop(stack) };

    match (str1_val, str2_val) {
        (Value::String(s1), Value::String(s2)) => {
            // Byte-clean: equality is byte-for-byte. Matches the
            // `PartialEq for SeqString` impl in seqstring.rs, which
            // also compares `as_bytes`.
            let equal = s1.as_bytes() == s2.as_bytes();
            unsafe { push(stack, Value::Bool(equal)) }
        }
        _ => panic!("string_equal: expected two strings on stack"),
    }
}
