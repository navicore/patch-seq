//! Higher-order list combinators: `list_map`, `list_filter`, `list_fold`,
//! `list_each`. Each invokes a Quotation/Closure on the caller's stack
//! via the shared `invoke_callable` helper from `quotations`.

use crate::stack::{Stack, drop_stack_value, get_stack_base, pop, pop_sv, push, stack_value_size};
use crate::value::{Value, VariantData};
use std::sync::Arc;

/// Check if the stack has at least `n` values
#[inline]
pub(super) fn stack_depth(stack: Stack) -> usize {
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
