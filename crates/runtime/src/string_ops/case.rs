//! Case conversion and whitespace trimming.

use crate::seqstring::{global_bytes, global_string};
use crate::stack::{Stack, pop, push};
use crate::value::Value;

/// # Safety
/// Stack must have the expected values on top for this operation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_trim(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "string_trim: stack is empty");

    let (stack, str_val) = unsafe { pop(stack) };

    match str_val {
        Value::String(s) => {
            let trimmed = s.as_str_or_empty().trim();
            unsafe { push(stack, Value::String(global_string(trimmed.to_owned()))) }
        }
        _ => panic!("string_trim: expected String on stack"),
    }
}

/// Convert a string to uppercase
///
/// Stack effect: ( str -- upper )
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_to_upper(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "string_to_upper: stack is empty");

    let (stack, str_val) = unsafe { pop(stack) };

    match str_val {
        Value::String(s) => {
            let upper = s.as_str_or_empty().to_uppercase();
            unsafe { push(stack, Value::String(global_string(upper))) }
        }
        _ => panic!("string_to_upper: expected String on stack"),
    }
}

/// Convert a string to lowercase
///
/// Stack effect: ( str -- lower )
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_to_lower(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "string_to_lower: stack is empty");

    let (stack, str_val) = unsafe { pop(stack) };

    match str_val {
        Value::String(s) => {
            let lower = s.as_str_or_empty().to_lowercase();
            unsafe { push(stack, Value::String(global_string(lower))) }
        }
        _ => panic!("string_to_lower: expected String on stack"),
    }
}

/// Check if two strings are equal
///
/// Stack effect: ( str1 str2 -- bool )
///
/// # Safety
/// Stack must have two String values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_chomp(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "string_chomp: stack is empty");

    let (stack, str_val) = unsafe { pop(stack) };

    match str_val {
        Value::String(s) => {
            // Byte-clean: peel a trailing `\n` (and its preceding
            // `\r`, for CRLF line endings). All comparisons against
            // ASCII bytes — works on text and on binary content with
            // a trailing newline.
            let mut bytes = s.as_bytes().to_vec();
            if bytes.last() == Some(&b'\n') {
                bytes.pop();
                if bytes.last() == Some(&b'\r') {
                    bytes.pop();
                }
            }
            unsafe { push(stack, Value::String(global_bytes(bytes))) }
        }
        _ => panic!("string_chomp: expected String on stack"),
    }
}
