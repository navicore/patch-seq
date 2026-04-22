//! Mutating-by-clone variant helpers: `variant_append`, `variant_last`,
//! `variant_init`, and the field-unpacker used for match expressions.

use crate::stack::{Stack, peek_heap_mut_second, pop, push};
use crate::value::Value;
use std::sync::Arc;

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
