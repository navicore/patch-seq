//! Variant field access operations for Seq
//!
//! Provides runtime functions for accessing variant fields, tags, and metadata.
//! These are used to work with composite data created by operations like string-split.

use crate::stack::{Stack, peek_heap_mut_second, pop, push};
use crate::value::Value;
use std::sync::Arc;

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

// ============================================================================
// Type-safe variant constructors with fixed arity
// Now accept Symbol as tag for dynamic variant construction (SON support)
// ============================================================================

/// Create a variant with 0 fields (just a tag)
///
/// Stack effect: ( Symbol -- Variant )
///
/// # Safety
/// Stack must have at least one Symbol (the tag) on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_0(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-0: expected Symbol (tag), got {:?}", tag_val),
        };

        let variant = Value::Variant(Arc::new(VariantData::new(tag, vec![])));
        push(stack, variant)
    }
}

/// Create a variant with 1 field
///
/// Stack effect: ( field1 Symbol -- Variant )
///
/// # Safety
/// Stack must have field1 and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_1(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-1: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(tag, vec![field1])));
        push(stack, variant)
    }
}

/// Create a variant with 2 fields
///
/// Stack effect: ( field1 field2 Symbol -- Variant )
///
/// # Safety
/// Stack must have field1, field2, and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_2(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-2: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(tag, vec![field1, field2])));
        push(stack, variant)
    }
}

/// Create a variant with 3 fields
///
/// Stack effect: ( field1 field2 field3 Symbol -- Variant )
///
/// # Safety
/// Stack must have field1, field2, field3, and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_3(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-3: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![field1, field2, field3],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 4 fields
///
/// Stack effect: ( field1 field2 field3 field4 Symbol -- Variant )
///
/// # Safety
/// Stack must have field1, field2, field3, field4, and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_4(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-4: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![field1, field2, field3, field4],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 5 fields
///
/// Stack effect: ( field1 field2 field3 field4 field5 Symbol -- Variant )
///
/// # Safety
/// Stack must have 5 fields and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_5(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-5: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field5) = pop(stack);
        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![field1, field2, field3, field4, field5],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 6 fields
///
/// Stack effect: ( field1 field2 field3 field4 field5 field6 Symbol -- Variant )
///
/// # Safety
/// Stack must have 6 fields and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_6(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-6: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field6) = pop(stack);
        let (stack, field5) = pop(stack);
        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![field1, field2, field3, field4, field5, field6],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 7 fields
///
/// Stack effect: ( field1 field2 field3 field4 field5 field6 field7 Symbol -- Variant )
///
/// # Safety
/// Stack must have 7 fields and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_7(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-7: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field7) = pop(stack);
        let (stack, field6) = pop(stack);
        let (stack, field5) = pop(stack);
        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![field1, field2, field3, field4, field5, field6, field7],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 8 fields
///
/// Stack effect: ( field1 field2 field3 field4 field5 field6 field7 field8 Symbol -- Variant )
///
/// # Safety
/// Stack must have 8 fields and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_8(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-8: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field8) = pop(stack);
        let (stack, field7) = pop(stack);
        let (stack, field6) = pop(stack);
        let (stack, field5) = pop(stack);
        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![
                field1, field2, field3, field4, field5, field6, field7, field8,
            ],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 9 fields
///
/// Stack effect: ( field1 ... field9 Symbol -- Variant )
///
/// # Safety
/// Stack must have 9 fields and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_9(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-9: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field9) = pop(stack);
        let (stack, field8) = pop(stack);
        let (stack, field7) = pop(stack);
        let (stack, field6) = pop(stack);
        let (stack, field5) = pop(stack);
        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![
                field1, field2, field3, field4, field5, field6, field7, field8, field9,
            ],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 10 fields
///
/// Stack effect: ( field1 ... field10 Symbol -- Variant )
///
/// # Safety
/// Stack must have 10 fields and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_10(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-10: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field10) = pop(stack);
        let (stack, field9) = pop(stack);
        let (stack, field8) = pop(stack);
        let (stack, field7) = pop(stack);
        let (stack, field6) = pop(stack);
        let (stack, field5) = pop(stack);
        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![
                field1, field2, field3, field4, field5, field6, field7, field8, field9, field10,
            ],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 11 fields
///
/// Stack effect: ( field1 ... field11 Symbol -- Variant )
///
/// # Safety
/// Stack must have 11 fields and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_11(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-11: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field11) = pop(stack);
        let (stack, field10) = pop(stack);
        let (stack, field9) = pop(stack);
        let (stack, field8) = pop(stack);
        let (stack, field7) = pop(stack);
        let (stack, field6) = pop(stack);
        let (stack, field5) = pop(stack);
        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![
                field1, field2, field3, field4, field5, field6, field7, field8, field9, field10,
                field11,
            ],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 12 fields
///
/// Stack effect: ( field1 ... field12 Symbol -- Variant )
///
/// # Safety
/// Stack must have 12 fields and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_12(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-12: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field12) = pop(stack);
        let (stack, field11) = pop(stack);
        let (stack, field10) = pop(stack);
        let (stack, field9) = pop(stack);
        let (stack, field8) = pop(stack);
        let (stack, field7) = pop(stack);
        let (stack, field6) = pop(stack);
        let (stack, field5) = pop(stack);
        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![
                field1, field2, field3, field4, field5, field6, field7, field8, field9, field10,
                field11, field12,
            ],
        )));
        push(stack, variant)
    }
}

// Re-exports for internal use
pub use patch_seq_make_variant_0 as make_variant_0;
pub use patch_seq_make_variant_1 as make_variant_1;
pub use patch_seq_make_variant_2 as make_variant_2;
pub use patch_seq_make_variant_3 as make_variant_3;
pub use patch_seq_make_variant_4 as make_variant_4;
pub use patch_seq_make_variant_5 as make_variant_5;
pub use patch_seq_make_variant_6 as make_variant_6;
pub use patch_seq_make_variant_7 as make_variant_7;
pub use patch_seq_make_variant_8 as make_variant_8;
pub use patch_seq_make_variant_9 as make_variant_9;
pub use patch_seq_make_variant_10 as make_variant_10;
pub use patch_seq_make_variant_11 as make_variant_11;
pub use patch_seq_make_variant_12 as make_variant_12;

/// Append a value to a variant, returning a new variant
///
/// Stack effect: ( Variant Value -- Variant' )
///
/// Creates a new variant with the same tag as the input, but with
/// the new value appended to its fields. The original variant is
/// not modified (functional/immutable style).
///
/// Example: For arrays, `[1, 2] 3 variant-append` produces `[1, 2, 3]`
/// Example: For objects, `{} "key" variant-append "val" variant-append` builds `{"key": "val"}`
///
/// # Safety
/// Stack must have a Variant and a Value on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_variant_append(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        // Fast path: peek at the variant at sp-2 without popping.
        // SAFETY: variant.append requires two values on the stack (enforced by
        // the type checker), so stack.sub(2) is valid.
        if let Some(Value::Variant(variant_arc)) = peek_heap_mut_second(stack)
            && let Some(data) = Arc::get_mut(variant_arc)
        {
            // Sole owner all the way down — mutate in place.
            // Safety: `data` points into the Value at sp-2. `pop` only
            // touches sp-1 (decrements sp, reads that slot), so sp-2's
            // memory is not accessed or invalidated by the pop.
            let (stack, value) = pop(stack);
            data.fields.push(value);
            return stack; // Variant is still at sp-1, mutated in place
        }

        // Slow path: pop both, clone if shared, push result
        let (stack, value) = pop(stack);
        let (stack, variant_val) = pop(stack);

        match variant_val {
            Value::Variant(mut arc) => {
                if let Some(data) = Arc::get_mut(&mut arc) {
                    data.fields.push(value);
                    push(stack, Value::Variant(arc))
                } else {
                    let mut new_fields = Vec::with_capacity(arc.fields.len() + 1);
                    new_fields.extend(arc.fields.iter().cloned());
                    new_fields.push(value);
                    let new_variant =
                        Value::Variant(Arc::new(VariantData::new(arc.tag.clone(), new_fields)));
                    push(stack, new_variant)
                }
            }
            _ => panic!("variant-append: expected Variant, got {:?}", variant_val),
        }
    }
}

/// Get the last field from a variant
///
/// Stack effect: ( Variant -- Value )
///
/// Returns a clone of the last field. Panics if the variant has no fields.
/// This is the "peek" operation for using variants as stacks.
///
/// # Safety
/// Stack must have a Variant on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_variant_last(stack: Stack) -> Stack {
    unsafe {
        let (stack, variant_val) = pop(stack);

        match variant_val {
            Value::Variant(variant_data) => {
                if variant_data.fields.is_empty() {
                    panic!("variant-last: variant has no fields");
                }

                let last = variant_data.fields.last().unwrap().clone();
                push(stack, last)
            }
            _ => panic!("variant-last: expected Variant, got {:?}", variant_val),
        }
    }
}

/// Get all but the last field from a variant
///
/// Stack effect: ( Variant -- Variant' )
///
/// Returns a new variant with the same tag but without the last field.
/// Panics if the variant has no fields.
/// This is the "pop" operation for using variants as stacks (without returning the popped value).
///
/// # Safety
/// Stack must have a Variant on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_variant_init(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, variant_val) = pop(stack);

        match variant_val {
            Value::Variant(variant_data) => {
                if variant_data.fields.is_empty() {
                    panic!("variant-init: variant has no fields");
                }

                // Create new fields without the last element
                let new_fields: Vec<Value> =
                    variant_data.fields[..variant_data.fields.len() - 1].to_vec();

                let new_variant = Value::Variant(Arc::new(VariantData::new(
                    variant_data.tag.clone(),
                    new_fields,
                )));

                push(stack, new_variant)
            }
            _ => panic!("variant-init: expected Variant, got {:?}", variant_val),
        }
    }
}

/// Unpack a variant's fields onto the stack
///
/// Takes a field count as parameter and:
/// - Pops the variant from the stack
/// - Pushes each field (0..field_count) in order
///
/// Stack effect: ( Variant -- field0 field1 ... fieldN-1 )
///
/// Used by match expression codegen to extract variant fields.
///
/// # Safety
/// Stack must have a Variant on top with at least `field_count` fields
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_unpack_variant(stack: Stack, field_count: i64) -> Stack {
    unsafe {
        let (mut stack, variant_val) = pop(stack);

        match variant_val {
            Value::Variant(variant_data) => {
                let count = field_count as usize;
                if count > variant_data.fields.len() {
                    panic!(
                        "unpack-variant: requested {} fields but variant only has {}",
                        count,
                        variant_data.fields.len()
                    );
                }

                // Push each field in order (field0 first, then field1, etc.)
                for i in 0..count {
                    stack = push(stack, variant_data.fields[i].clone());
                }

                stack
            }
            _ => panic!("unpack-variant: expected Variant, got {:?}", variant_val),
        }
    }
}

// Public re-exports with short names for internal use
pub use patch_seq_unpack_variant as unpack_variant;
pub use patch_seq_variant_append as variant_append;
pub use patch_seq_variant_field_at as variant_field_at;
pub use patch_seq_variant_field_count as variant_field_count;
pub use patch_seq_variant_init as variant_init;
pub use patch_seq_variant_last as variant_last;
pub use patch_seq_variant_tag as variant_tag;

#[cfg(test)]
mod tests;
