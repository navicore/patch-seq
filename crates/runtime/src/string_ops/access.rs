//! Character access, slicing, searching, and splitting/joining.

use crate::seqstring::{global_bytes, global_string};
use crate::stack::{Stack, pop, push};
use crate::value::Value;
use std::sync::Arc;

/// # Safety
/// Stack must have the expected values on top for this operation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_split(stack: Stack) -> Stack {
    use crate::value::VariantData;

    assert!(!stack.is_null(), "string_split: stack is empty");

    let (stack, delim_val) = unsafe { pop(stack) };
    assert!(!stack.is_null(), "string_split: need two strings");
    let (stack, str_val) = unsafe { pop(stack) };

    match (str_val, delim_val) {
        (Value::String(s), Value::String(d)) => {
            // Byte-clean split: separate the haystack at every byte
            // occurrence of the needle. The result is byte-faithful —
            // splitting an OSC payload on its NUL padding, splitting a
            // network frame on a binary delimiter, etc. all work.
            let bytes = s.as_bytes();
            let needle = d.as_bytes();
            let parts: Vec<Vec<u8>> = if needle.is_empty() {
                // Mirror Rust's `&str::split("")` shape: empty leading
                // and trailing pieces, one piece per byte in between.
                let mut parts = Vec::with_capacity(bytes.len() + 2);
                parts.push(Vec::new());
                for b in bytes {
                    parts.push(vec![*b]);
                }
                parts.push(Vec::new());
                parts
            } else {
                let mut parts: Vec<Vec<u8>> = Vec::new();
                let mut last = 0usize;
                let mut i = 0usize;
                while i + needle.len() <= bytes.len() {
                    if &bytes[i..i + needle.len()] == needle {
                        parts.push(bytes[last..i].to_vec());
                        i += needle.len();
                        last = i;
                    } else {
                        i += 1;
                    }
                }
                parts.push(bytes[last..].to_vec());
                parts
            };

            let fields: Vec<Value> = parts
                .into_iter()
                .map(|part| Value::String(global_bytes(part)))
                .collect();

            // Create a Variant with :List tag and the split parts as fields
            let variant = Value::Variant(Arc::new(VariantData::new(
                global_string("List".to_string()),
                fields,
            )));

            unsafe { push(stack, variant) }
        }
        _ => panic!("string_split: expected two strings on stack"),
    }
}

/// Check if a string is empty
///
/// Stack effect: ( str -- bool )
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_contains(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "string_contains: stack is empty");

    let (stack, substring_val) = unsafe { pop(stack) };
    assert!(!stack.is_null(), "string_contains: need two strings");
    let (stack, str_val) = unsafe { pop(stack) };

    match (str_val, substring_val) {
        (Value::String(s), Value::String(sub)) => {
            // Byte-clean substring search: scan the haystack for the
            // needle's bytes. Works on any input — text or binary.
            let contains = byte_contains(s.as_bytes(), sub.as_bytes());
            unsafe { push(stack, Value::Bool(contains)) }
        }
        _ => panic!("string_contains: expected two strings on stack"),
    }
}

/// Byte-level substring search. Empty needle is contained in any
/// haystack (matches Rust's `&str::contains` for the same convention).
fn byte_contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Check if a string starts with a prefix
///
/// Stack effect: ( str prefix -- bool )
///
/// # Safety
/// Stack must have two String values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_starts_with(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "string_starts_with: stack is empty");

    let (stack, prefix_val) = unsafe { pop(stack) };
    assert!(!stack.is_null(), "string_starts_with: need two strings");
    let (stack, str_val) = unsafe { pop(stack) };

    match (str_val, prefix_val) {
        (Value::String(s), Value::String(prefix)) => {
            // Byte-clean prefix check.
            let starts = s.as_bytes().starts_with(prefix.as_bytes());
            unsafe { push(stack, Value::Bool(starts)) }
        }
        _ => panic!("string_starts_with: expected two strings on stack"),
    }
}

/// Concatenate two strings
///
/// Stack effect: ( str1 str2 -- result )
///
/// # Safety
/// Stack must have two String values on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_char_at(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "string_char_at: stack is empty");

    let (stack, index_val) = unsafe { pop(stack) };
    assert!(!stack.is_null(), "string_char_at: need string and index");
    let (stack, str_val) = unsafe { pop(stack) };

    match (str_val, index_val) {
        (Value::String(s), Value::Int(index)) => {
            let result = if index < 0 {
                -1
            } else {
                s.as_str_or_empty()
                    .chars()
                    .nth(index as usize)
                    .map(|c| c as i64)
                    .unwrap_or(-1)
            };
            unsafe { push(stack, Value::Int(result)) }
        }
        _ => panic!("string_char_at: expected String and Int on stack"),
    }
}

/// Extract a substring using character indices
///
/// Stack effect: ( str start len -- str )
///
/// Arguments:
/// - str: The source string
/// - start: Starting character index
/// - len: Number of characters to extract
///
/// Edge cases:
/// - Start beyond end: returns empty string
/// - Length extends past end: clamps to available
///
/// # Safety
/// Stack must have String, Int, Int on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_substring(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "string_substring: stack is empty");

    let (stack, len_val) = unsafe { pop(stack) };
    assert!(
        !stack.is_null(),
        "string_substring: need string, start, len"
    );
    let (stack, start_val) = unsafe { pop(stack) };
    assert!(
        !stack.is_null(),
        "string_substring: need string, start, len"
    );
    let (stack, str_val) = unsafe { pop(stack) };

    match (str_val, start_val, len_val) {
        (Value::String(s), Value::Int(start), Value::Int(len)) => {
            let result = if start < 0 || len < 0 {
                String::new()
            } else {
                s.as_str_or_empty()
                    .chars()
                    .skip(start as usize)
                    .take(len as usize)
                    .collect()
            };
            unsafe { push(stack, Value::String(global_string(result))) }
        }
        _ => panic!("string_substring: expected String, Int, Int on stack"),
    }
}

/// Convert a Unicode code point to a single-character string
///
/// Stack effect: ( int -- str )
///
/// Creates a string containing the single character represented by the code point.
/// Panics if the code point is invalid.
///
/// # Safety
/// Stack must have an Int on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_char_to_string(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "char_to_string: stack is empty");

    let (stack, code_point_val) = unsafe { pop(stack) };

    match code_point_val {
        Value::Int(code_point) => {
            let result = if !(0..=0x10FFFF).contains(&code_point) {
                // Invalid code point - return empty string
                String::new()
            } else {
                match char::from_u32(code_point as u32) {
                    Some(c) => c.to_string(),
                    None => String::new(), // Invalid code point (e.g., surrogate)
                }
            };
            unsafe { push(stack, Value::String(global_string(result))) }
        }
        _ => panic!("char_to_string: expected Int on stack"),
    }
}

/// Find the first occurrence of a substring
///
/// Stack effect: ( str needle -- int )
///
/// Returns the character index of the first occurrence of needle in str.
/// Returns -1 if not found.
///
/// # Safety
/// Stack must have two Strings on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_find(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "string_find: stack is empty");

    let (stack, needle_val) = unsafe { pop(stack) };
    assert!(!stack.is_null(), "string_find: need string and needle");
    let (stack, str_val) = unsafe { pop(stack) };

    match (str_val, needle_val) {
        (Value::String(haystack), Value::String(needle)) => {
            let haystack_str = haystack.as_str_or_empty();
            let needle_str = needle.as_str_or_empty();

            // Find byte position then convert to character position
            let result = match haystack_str.find(needle_str) {
                Some(byte_pos) => {
                    // Count characters up to byte_pos
                    haystack_str[..byte_pos].chars().count() as i64
                }
                None => -1,
            };
            unsafe { push(stack, Value::Int(result)) }
        }
        _ => panic!("string_find: expected two Strings on stack"),
    }
}

/// Trim whitespace from both ends of a string
///
/// Stack effect: ( str -- trimmed )
///
/// # Safety
/// Stack must have a String value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_string_join(stack: Stack) -> Stack {
    unsafe {
        // Pop separator
        let (stack, sep_val) = pop(stack);
        let sep = match &sep_val {
            Value::String(s) => s.as_str_or_empty().to_owned(),
            _ => panic!("string.join: expected String separator, got {:?}", sep_val),
        };

        // Pop list (variant)
        let (stack, list_val) = pop(stack);
        let variant_data = match &list_val {
            Value::Variant(v) => v,
            _ => panic!("string.join: expected Variant (list), got {:?}", list_val),
        };

        // Convert each element to string and join
        let parts: Vec<String> = variant_data
            .fields
            .iter()
            .map(|v| match v {
                Value::String(s) => s.as_str_or_empty().to_owned(),
                Value::Int(n) => n.to_string(),
                Value::Float(f) => f.to_string(),
                Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
                Value::Symbol(s) => format!(":{}", s.as_str_or_empty()),
                _ => format!("{:?}", v),
            })
            .collect();

        let result = parts.join(&sep);
        push(stack, Value::String(global_string(result)))
    }
}
