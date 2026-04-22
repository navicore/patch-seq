//! Variant field access: count, tag lookup, field access.

use crate::stack::{Stack, pop, push};
use crate::value::Value;

/// Get the number of fields in a variant
///
/// Stack effect: ( Variant -- Int )
///
/// # Safety
/// Stack must have a Variant on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_variant_field_count(stack: Stack) -> Stack {
    unsafe {
        let (stack, value) = pop(stack);

        match value {
            Value::Variant(variant_data) => {
                let count = variant_data.fields.len() as i64;
                push(stack, Value::Int(count))
            }
            _ => panic!("variant-field-count: expected Variant, got {:?}", value),
        }
    }
}

/// Get the tag of a variant
///
/// Stack effect: ( Variant -- Symbol )
///
/// # Safety
/// Stack must have a Variant on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_variant_tag(stack: Stack) -> Stack {
    unsafe {
        let (stack, value) = pop(stack);

        match value {
            Value::Variant(variant_data) => {
                // Return the tag as a Symbol
                push(stack, Value::Symbol(variant_data.tag.clone()))
            }
            _ => panic!("variant-tag: expected Variant, got {:?}", value),
        }
    }
}

/// Compare a symbol tag with a C string literal
///
/// Used by pattern matching codegen to dispatch on variant tags.
/// The stack should have a Symbol on top (typically from variant-tag).
/// Compares with the provided C string and pushes Bool result.
///
/// Stack effect: ( Symbol -- Bool )
///
/// # Safety
/// - Stack must have a Symbol on top
/// - c_str must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_symbol_eq_cstr(stack: Stack, c_str: *const i8) -> Stack {
    use std::ffi::CStr;

    unsafe {
        let (stack, value) = pop(stack);
        let symbol_str = match value {
            Value::Symbol(s) => s,
            _ => panic!("symbol_eq_cstr: expected Symbol, got {:?}", value),
        };

        let expected = CStr::from_ptr(c_str)
            .to_str()
            .expect("Invalid UTF-8 in variant name");

        let is_equal = symbol_str.as_str() == expected;
        push(stack, Value::Bool(is_equal))
    }
}

/// Get a field from a variant at the given index
///
/// Stack effect: ( Variant Int -- Value )
///
/// Returns a clone of the field value at the specified index.
/// Panics if index is out of bounds (this is a programming bug, not recoverable).
/// Use variant.field-count to check bounds first if needed.
///
/// # Safety
/// Stack must have a Variant and Int on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_variant_field_at(stack: Stack) -> Stack {
    unsafe {
        let (stack, index_val) = pop(stack);
        let index = match index_val {
            Value::Int(i) => i,
            _ => panic!(
                "variant-field-at: expected Int (index), got {:?}",
                index_val
            ),
        };

        if index < 0 {
            panic!("variant-field-at: index cannot be negative: {}", index);
        }

        let (stack, variant_val) = pop(stack);

        match variant_val {
            Value::Variant(variant_data) => {
                let idx = index as usize;
                if idx >= variant_data.fields.len() {
                    panic!(
                        "variant-field-at: index {} out of bounds (variant has {} fields)",
                        index,
                        variant_data.fields.len()
                    );
                }

                // Clone the field value and push it
                let field = variant_data.fields[idx].clone();
                push(stack, field)
            }
            _ => panic!("variant-field-at: expected Variant, got {:?}", variant_val),
        }
    }
}
