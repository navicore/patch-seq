//! Basic list operations: `list_length`, `list_empty`, `list_make`,
//! `list_push`, `list_reverse`.

use crate::stack::{Stack, peek_heap_mut_second, pop, push};
use crate::value::{Value, VariantData};
use std::sync::Arc;

/// # Safety
/// Stack must have the expected values on top for this operation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_list_length(stack: Stack) -> Stack {
    unsafe { crate::variant_ops::patch_seq_variant_field_count(stack) }
}

/// Check if a list is empty
///
/// Stack effect: ( Variant -- Bool )
///
/// Returns true if the list has no elements, false otherwise.
///
/// # Safety
/// Stack must have a Variant on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_list_empty(stack: Stack) -> Stack {
    unsafe {
        let (stack, list_val) = pop(stack);

        let is_empty = match list_val {
            Value::Variant(v) => v.fields.is_empty(),
            _ => panic!("list-empty?: expected Variant (list), got {:?}", list_val),
        };

        push(stack, Value::Bool(is_empty))
    }
}

/// Create an empty list
///
/// Stack effect: ( -- Variant )
///
/// Returns a new empty list (Variant with tag "List" and no fields).
///
/// # Safety
/// No requirements on stack
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_list_make(stack: Stack) -> Stack {
    unsafe {
        let list = Value::Variant(Arc::new(VariantData::new(
            crate::seqstring::global_string("List".to_string()),
            vec![],
        )));
        push(stack, list)
    }
}

/// Append an element to a list with COW optimization.
///
/// Stack effect: ( Variant Value -- Variant )
///
/// Fast path: if the list (at sp-2) is a sole-owned heap value, mutates
/// in place via `peek_heap_mut_second` — no Arc alloc/dealloc cycle.
/// Slow path: pops, clones, and pushes a new list.
///
/// # Safety
/// Stack must have a Value on top and a Variant (list) below
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_list_push(stack: Stack) -> Stack {
    unsafe {
        // Try the fast path: peek at the list without popping.
        // SAFETY: list.push requires two values on the stack (enforced by
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
            return stack; // List is still at sp-1, untouched
        }

        // Slow path: pop both, clone if shared, push result
        let (stack, value) = pop(stack);
        let (stack, list_val) = pop(stack);
        let variant_arc = match list_val {
            Value::Variant(v) => v,
            _ => panic!("list.push: expected Variant (list), got {:?}", list_val),
        };
        push_to_variant(stack, variant_arc, value)
    }
}

/// COW push helper: append value to variant, mutating in place when sole owner.
unsafe fn push_to_variant(stack: Stack, mut variant_arc: Arc<VariantData>, value: Value) -> Stack {
    unsafe {
        if let Some(data) = Arc::get_mut(&mut variant_arc) {
            // Sole owner — mutate in place (amortized O(1))
            data.fields.push(value);
            push(stack, Value::Variant(variant_arc))
        } else {
            // Shared — clone and append (O(n))
            let mut new_fields = Vec::with_capacity(variant_arc.fields.len() + 1);
            new_fields.extend(variant_arc.fields.iter().cloned());
            new_fields.push(value);
            let new_list = Value::Variant(Arc::new(VariantData::new(
                variant_arc.tag.clone(),
                new_fields,
            )));
            push(stack, new_list)
        }
    }
}

/// Get an element from a list by index
///
/// Stack effect: ( Variant Int -- Value Bool )
///
/// Returns the value at the given index and true, or
/// a placeholder value and false if index is out of bounds.
///
/// # Error Handling
/// - Empty stack: Sets runtime error, returns 0 and false
/// - Type mismatch: Sets runtime error, returns 0 and false
/// - Out of bounds: Returns 0 and false (no error set, this is expected)
///
/// # Safety
/// Stack must have an Int on top and a Variant (list) below
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_list_reverse(stack: Stack) -> Stack {
    unsafe {
        let (stack, list_val) = pop(stack);

        let variant_data = match list_val {
            Value::Variant(v) => v,
            _ => panic!("list.reverse: expected Variant (list), got {:?}", list_val),
        };

        let mut reversed: Vec<Value> = variant_data.fields.to_vec();
        reversed.reverse();

        let new_variant = Value::Variant(Arc::new(VariantData::new(
            variant_data.tag.clone(),
            reversed,
        )));

        push(stack, new_variant)
    }
}
