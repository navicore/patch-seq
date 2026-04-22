//! Indexed list access: `list_get` (returns success flag) and `list_set`
//! (functional update returning a new list + success flag).

use super::combinators::stack_depth;
use crate::error::set_runtime_error;
use crate::stack::{Stack, heap_value_mut, pop, push};
use crate::value::{Value, VariantData};
use std::sync::Arc;

/// # Safety
/// Stack must have the expected values on top for this operation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_list_get(stack: Stack) -> Stack {
    unsafe {
        // Check stack depth before any pops to avoid partial consumption
        if stack_depth(stack) < 2 {
            set_runtime_error("list.get: stack underflow (need 2 values)");
            return stack;
        }
        let (stack, index_val) = pop(stack);
        let (stack, list_val) = pop(stack);

        let index = match index_val {
            Value::Int(i) => i,
            _ => {
                set_runtime_error(format!(
                    "list.get: expected Int (index), got {:?}",
                    index_val
                ));
                let stack = push(stack, Value::Int(0));
                return push(stack, Value::Bool(false));
            }
        };

        let variant_data = match list_val {
            Value::Variant(v) => v,
            _ => {
                set_runtime_error(format!(
                    "list.get: expected Variant (list), got {:?}",
                    list_val
                ));
                let stack = push(stack, Value::Int(0));
                return push(stack, Value::Bool(false));
            }
        };

        if index < 0 || index as usize >= variant_data.fields.len() {
            // Out of bounds - return false
            let stack = push(stack, Value::Int(0)); // placeholder
            push(stack, Value::Bool(false))
        } else {
            let value = variant_data.fields[index as usize].clone();
            let stack = push(stack, value);
            push(stack, Value::Bool(true))
        }
    }
}

/// Set an element in a list by index with COW optimization.
///
/// Stack effect: ( Variant Int Value -- Variant Bool )
///
/// Fast path: if the list (at sp-3) is sole-owned and the index (at sp-2)
/// is a valid tagged int, peeks at both without popping, then pops value
/// and index and mutates the list in place.
/// Slow path: pops all three, clones if shared, pushes new list.
///
/// Returns the list with the value at the given index replaced, and true.
/// If index is out of bounds, returns the original list and false.
///
/// # Error Handling
/// - Empty stack: Sets runtime error, returns unchanged stack
/// - Type mismatch: Sets runtime error, returns original list and false
/// - Out of bounds: Returns original list and false (no error set, this is expected)
///
/// # Safety
/// Stack must have Value on top, Int below, and Variant (list) below that
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_list_set(stack: Stack) -> Stack {
    unsafe {
        // Check stack depth before any pops to avoid partial consumption
        if stack_depth(stack) < 3 {
            set_runtime_error("list.set: stack underflow (need 3 values)");
            return stack;
        }

        // Fast path: peek at the list at sp-3 without popping.
        // SAFETY: stack depth >= 3 verified above, so stack.sub(3) is valid.
        // The index at sp-2 must be an Int for the fast path; read it inline
        // to avoid popping/pushing back on type mismatch.
        if let Some(Value::Variant(variant_arc)) = heap_value_mut(stack.sub(3))
            && let Some(data) = Arc::get_mut(variant_arc)
        {
            // Peek at the index at sp-2 without popping — it's an Int (inline),
            // so we can read it directly from the tagged value.
            let index_sv = *stack.sub(2);
            if crate::tagged_stack::is_tagged_int(index_sv) {
                let index = crate::tagged_stack::untag_int(index_sv);
                if index >= 0 && (index as usize) < data.fields.len() {
                    // Sole owner, valid index — pop value and index, mutate in place.
                    // Safety: two pops move sp by 2; the list at the
                    // original sp-3 (now sp-1) is not invalidated.
                    let (stack, value) = pop(stack);
                    let (stack, _index) = pop(stack);
                    data.fields[index as usize] = value;
                    return push(stack, Value::Bool(true));
                }
                // Out of bounds — pop value and index, leave list at sp-1
                let (stack, _value) = pop(stack);
                let (stack, _index) = pop(stack);
                return push(stack, Value::Bool(false));
            }
        }

        // Slow path: pop all three, clone if shared, push result
        let (stack, value) = pop(stack);
        let (stack, index_val) = pop(stack);
        let (stack, list_val) = pop(stack);

        let index = match index_val {
            Value::Int(i) => i,
            _ => {
                set_runtime_error(format!(
                    "list.set: expected Int (index), got {:?}",
                    index_val
                ));
                let stack = push(stack, list_val);
                return push(stack, Value::Bool(false));
            }
        };

        let mut arc = match list_val {
            Value::Variant(v) => v,
            other => {
                set_runtime_error(format!(
                    "list.set: expected Variant (list), got {:?}",
                    other
                ));
                let stack = push(stack, other);
                return push(stack, Value::Bool(false));
            }
        };

        if index < 0 || index as usize >= arc.fields.len() {
            let stack = push(stack, Value::Variant(arc));
            push(stack, Value::Bool(false))
        } else if let Some(data) = Arc::get_mut(&mut arc) {
            data.fields[index as usize] = value;
            let stack = push(stack, Value::Variant(arc));
            push(stack, Value::Bool(true))
        } else {
            let mut new_fields: Vec<Value> = arc.fields.to_vec();
            new_fields[index as usize] = value;
            let new_list = Value::Variant(Arc::new(VariantData::new(arc.tag.clone(), new_fields)));
            let stack = push(stack, new_list);
            push(stack, Value::Bool(true))
        }
    }
}
