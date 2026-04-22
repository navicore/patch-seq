//! List operations for Seq
//!
//! Higher-order combinators for working with lists (Variants).
//! These provide idiomatic concatenative-style list processing.
//!
//! # Examples
//!
//! ```seq
//! # Map: double each element
//! my-list [ 2 * ] list-map
//!
//! # Filter: keep positive numbers
//! my-list [ 0 > ] list-filter
//!
//! # Fold: sum all elements
//! my-list 0 [ + ] list-fold
//!
//! # Each: print each element
//! my-list [ write_line ] list-each
//! ```

use crate::error::set_runtime_error;
use crate::stack::{
    Stack, drop_stack_value, get_stack_base, heap_value_mut, peek_heap_mut_second, pop, pop_sv,
    push, stack_value_size,
};
use crate::value::{Value, VariantData};
use std::sync::Arc;

/// Check if the stack has at least `n` values
#[inline]
fn stack_depth(stack: Stack) -> usize {
    if stack.is_null() {
        return 0;
    }
    let base = get_stack_base();
    if base.is_null() {
        return 0;
    }
    (stack as usize - base as usize) / stack_value_size()
}

/// Helper to drain any remaining stack values back to the base
///
/// This ensures no memory is leaked if a quotation misbehaves
/// by leaving extra values on the stack.
unsafe fn drain_stack_to_base(mut stack: Stack, base: Stack) {
    unsafe {
        while stack > base {
            let (rest, sv) = pop_sv(stack);
            drop_stack_value(sv);
            stack = rest;
        }
    }
}

/// Helper to call a quotation or closure with a value on the stack.
///
/// Pushes `value` onto the base stack, then invokes the callable.
/// Uses the shared `invoke_callable` from quotations.rs.
unsafe fn call_with_value(base: Stack, value: Value, callable: &Value) -> Stack {
    unsafe {
        let stack = push(base, value);
        crate::quotations::invoke_callable(stack, callable)
    }
}

/// Map a quotation over a list, returning a new list
///
/// Stack effect: ( Variant Quotation -- Variant )
///
/// The quotation should have effect ( elem -- elem' )
/// Each element is transformed by the quotation.
///
/// # Safety
/// Stack must have a Quotation/Closure on top and a Variant below
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_list_map(stack: Stack) -> Stack {
    unsafe {
        // Pop quotation
        let (stack, callable) = pop(stack);

        // Validate callable
        match &callable {
            Value::Quotation { .. } | Value::Closure { .. } => {}
            _ => panic!(
                "list-map: expected Quotation or Closure, got {:?}",
                callable
            ),
        }

        // Pop variant (list)
        let (stack, list_val) = pop(stack);

        let variant_data = match list_val {
            Value::Variant(v) => v,
            _ => panic!("list-map: expected Variant (list), got {:?}", list_val),
        };

        // Map over each element
        let mut results = Vec::with_capacity(variant_data.fields.len());

        for field in variant_data.fields.iter() {
            // Create a fresh temp stack for this call
            let temp_base = crate::stack::alloc_stack();
            let temp_stack = call_with_value(temp_base, field.clone(), &callable);

            // Pop result - quotation should have effect ( elem -- elem' )
            if temp_stack <= temp_base {
                panic!("list-map: quotation consumed element without producing result");
            }
            let (remaining, result) = pop(temp_stack);
            results.push(result);

            // Stack hygiene: drain any extra values left by misbehaving quotation
            if remaining > temp_base {
                drain_stack_to_base(remaining, temp_base);
            }
        }

        // Create new variant with same tag
        let new_variant = Value::Variant(Arc::new(VariantData::new(
            variant_data.tag.clone(),
            results,
        )));

        push(stack, new_variant)
    }
}

/// Filter a list, keeping elements where quotation returns true
///
/// Stack effect: ( Variant Quotation -- Variant )
///
/// The quotation should have effect ( elem -- Bool )
/// Elements are kept if the quotation returns true.
///
/// # Safety
/// Stack must have a Quotation/Closure on top and a Variant below
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_list_filter(stack: Stack) -> Stack {
    unsafe {
        // Pop quotation
        let (stack, callable) = pop(stack);

        // Validate callable
        match &callable {
            Value::Quotation { .. } | Value::Closure { .. } => {}
            _ => panic!(
                "list-filter: expected Quotation or Closure, got {:?}",
                callable
            ),
        }

        // Pop variant (list)
        let (stack, list_val) = pop(stack);

        let variant_data = match list_val {
            Value::Variant(v) => v,
            _ => panic!("list-filter: expected Variant (list), got {:?}", list_val),
        };

        // Filter elements
        let mut results = Vec::new();

        for field in variant_data.fields.iter() {
            // Create a fresh temp stack for this call
            let temp_base = crate::stack::alloc_stack();
            let temp_stack = call_with_value(temp_base, field.clone(), &callable);

            // Pop result - quotation should have effect ( elem -- Bool )
            if temp_stack <= temp_base {
                panic!("list-filter: quotation consumed element without producing result");
            }
            let (remaining, result) = pop(temp_stack);

            let keep = match result {
                Value::Bool(b) => b,
                _ => panic!("list-filter: quotation must return Bool, got {:?}", result),
            };

            if keep {
                results.push(field.clone());
            }

            // Stack hygiene: drain any extra values left by misbehaving quotation
            if remaining > temp_base {
                drain_stack_to_base(remaining, temp_base);
            }
        }

        // Create new variant with same tag
        let new_variant = Value::Variant(Arc::new(VariantData::new(
            variant_data.tag.clone(),
            results,
        )));

        push(stack, new_variant)
    }
}

/// Fold a list with an accumulator and quotation
///
/// Stack effect: ( Variant init Quotation -- result )
///
/// The quotation should have effect ( acc elem -- acc' )
/// Starts with init as accumulator, folds left through the list.
///
/// # Safety
/// Stack must have Quotation on top, init below, and Variant below that
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_list_fold(stack: Stack) -> Stack {
    unsafe {
        // Pop quotation
        let (stack, callable) = pop(stack);

        // Validate callable
        match &callable {
            Value::Quotation { .. } | Value::Closure { .. } => {}
            _ => panic!(
                "list-fold: expected Quotation or Closure, got {:?}",
                callable
            ),
        }

        // Pop initial accumulator
        let (stack, init) = pop(stack);

        // Pop variant (list)
        let (stack, list_val) = pop(stack);

        let variant_data = match list_val {
            Value::Variant(v) => v,
            _ => panic!("list-fold: expected Variant (list), got {:?}", list_val),
        };

        // Fold over elements
        let mut acc = init;

        for field in variant_data.fields.iter() {
            // Create a fresh temp stack and push acc, then element, then call quotation
            let temp_base = crate::stack::alloc_stack();
            let temp_stack = push(temp_base, acc);
            let temp_stack = push(temp_stack, field.clone());

            let temp_stack = match &callable {
                Value::Quotation { wrapper, .. } => {
                    let fn_ref: unsafe extern "C" fn(Stack) -> Stack =
                        std::mem::transmute(*wrapper);
                    fn_ref(temp_stack)
                }
                Value::Closure { fn_ptr, env } => {
                    let fn_ref: unsafe extern "C" fn(Stack, *const Value, usize) -> Stack =
                        std::mem::transmute(*fn_ptr);
                    fn_ref(temp_stack, env.as_ptr(), env.len())
                }
                _ => unreachable!(),
            };

            // Pop new accumulator - quotation should have effect ( acc elem -- acc' )
            if temp_stack <= temp_base {
                panic!("list-fold: quotation consumed inputs without producing result");
            }
            let (remaining, new_acc) = pop(temp_stack);
            acc = new_acc;

            // Stack hygiene: drain any extra values left by misbehaving quotation
            if remaining > temp_base {
                drain_stack_to_base(remaining, temp_base);
            }
        }

        push(stack, acc)
    }
}

/// Apply a quotation to each element of a list (for side effects)
///
/// Stack effect: ( Variant Quotation -- )
///
/// The quotation should have effect ( elem -- )
/// Each element is passed to the quotation; results are discarded.
///
/// # Safety
/// Stack must have a Quotation/Closure on top and a Variant below
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_list_each(stack: Stack) -> Stack {
    unsafe {
        // Pop quotation
        let (stack, callable) = pop(stack);

        // Validate callable
        match &callable {
            Value::Quotation { .. } | Value::Closure { .. } => {}
            _ => panic!(
                "list-each: expected Quotation or Closure, got {:?}",
                callable
            ),
        }

        // Pop variant (list)
        let (stack, list_val) = pop(stack);

        let variant_data = match list_val {
            Value::Variant(v) => v,
            _ => panic!("list-each: expected Variant (list), got {:?}", list_val),
        };

        // Call quotation for each element (for side effects)
        for field in variant_data.fields.iter() {
            let temp_base = crate::stack::alloc_stack();
            let temp_stack = call_with_value(temp_base, field.clone(), &callable);
            // Stack hygiene: drain any values left by quotation (effect should be ( elem -- ))
            if temp_stack > temp_base {
                drain_stack_to_base(temp_stack, temp_base);
            }
        }

        stack
    }
}

/// Get the length of a list
///
/// Stack effect: ( Variant -- Int )
///
/// Returns the number of elements in the list.
/// This is an alias for variant-field-count, provided for semantic clarity.
///
/// # Safety
/// Stack must have a Variant on top
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

/// Reverse a list.
///
/// Stack effect: ( Variant -- Variant )
///
/// Returns a new list with elements in reverse order.
///
/// # Safety
/// Stack must have a Variant (list) on top
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

// Public re-exports
pub use patch_seq_list_each as list_each;
pub use patch_seq_list_empty as list_empty;
pub use patch_seq_list_filter as list_filter;
pub use patch_seq_list_fold as list_fold;
pub use patch_seq_list_get as list_get;
pub use patch_seq_list_length as list_length;
pub use patch_seq_list_make as list_make;
pub use patch_seq_list_map as list_map;
pub use patch_seq_list_push as list_push;
pub use patch_seq_list_reverse as list_reverse;
pub use patch_seq_list_set as list_set;

#[cfg(test)]
mod tests;
