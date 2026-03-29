//! Map operations for Seq
//!
//! Dictionary/hash map operations with O(1) lookup.
//! Maps use hashable keys (Int, String, Bool) and can store any Value.
//!
//! # Examples
//!
//! ```seq
//! # Create empty map and add entries
//! make-map "name" "Alice" map-set "age" 30 map-set
//!
//! # Get value by key
//! my-map "name" map-get  # -> "Alice"
//!
//! # Check if key exists
//! my-map "email" map-has?  # -> 0 (false)
//!
//! # Get keys/values as lists
//! my-map map-keys    # -> ["name", "age"]
//! my-map map-values  # -> ["Alice", 30]
//! ```
//!
//! # Error Handling
//!
//! - `map-get` returns (value Bool) - false if key not found (errors are values, not crashes)
//! - Type errors (invalid key types, non-Map values) still panic (internal bugs)
//!
//! # Performance Notes
//!
//! - Operations use functional style: `map-set` and `map-remove` return new maps
//! - Each mutation clones the underlying HashMap (O(n) for n entries)
//! - For small maps (<100 entries), this is typically fast enough
//! - Key/value iteration order is not guaranteed (HashMap iteration order)

use crate::seqstring::global_string;
use crate::stack::{Stack, drop_stack_value, heap_value_mut, pop, pop_sv, push};
use crate::value::{MapKey, Value, VariantData};
use std::sync::Arc;

/// Create an empty map
///
/// Stack effect: ( -- Map )
///
/// # Safety
/// Stack can be any valid stack pointer (including null for empty stack)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_map(stack: Stack) -> Stack {
    unsafe { push(stack, Value::Map(Box::default())) }
}

/// Get a value from the map by key
///
/// Stack effect: ( Map key -- value Bool )
///
/// Returns (value true) if found, or (0 false) if not found.
/// Errors are values, not crashes.
/// Panics only for internal bugs (invalid key type, non-Map value).
///
/// # Safety
/// Stack must have a hashable key on top and a Map below
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_map_get(stack: Stack) -> Stack {
    unsafe {
        // Pop key
        let (stack, key_val) = pop(stack);
        let key = MapKey::from_value(&key_val).unwrap_or_else(|| {
            panic!(
                "map-get: key must be Int, String, or Bool, got {:?}",
                key_val
            )
        });

        // Pop map
        let (stack, map_val) = pop(stack);
        let map = match map_val {
            Value::Map(m) => m,
            _ => panic!("map-get: expected Map, got {:?}", map_val),
        };

        // Look up value - return success flag instead of panicking
        match map.get(&key) {
            Some(value) => {
                let stack = push(stack, value.clone());
                push(stack, Value::Bool(true))
            }
            None => {
                let stack = push(stack, Value::Int(0)); // placeholder value
                push(stack, Value::Bool(false)) // not found
            }
        }
    }
}

/// Set a key-value pair in the map with COW optimization.
///
/// Stack effect: ( Map key value -- Map )
///
/// Fast path: if the map (at sp-3) is sole-owned, pops key and value,
/// inserts directly into the map in place — no Box alloc/dealloc cycle.
/// Slow path: pops all three, clones the map, inserts, pushes new map.
///
/// # Safety
/// Stack must have value on top, key below, and Map at third position
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_map_set(stack: Stack) -> Stack {
    unsafe {
        // Fast path: peek at the map at sp-3 without popping.
        // SAFETY: map.set requires three values on the stack (enforced by
        // the type checker), so stack.sub(3) is valid.
        if let Some(Value::Map(map)) = heap_value_mut(stack.sub(3)) {
            // Sole owner — pop key and value, mutate map in place.
            let (stack, value) = pop(stack);
            let (stack, key_val) = pop(stack);
            let key = MapKey::from_value(&key_val).unwrap_or_else(|| {
                panic!(
                    "map-set: key must be Int, String, or Bool, got {:?}",
                    key_val
                )
            });
            // Safety: `pop` only touches sp-1 per call; the map at
            // the original sp-3 (now sp-1) is not invalidated.
            map.insert(key, value);
            return stack; // Map is still at sp-1, mutated in place
        }

        // Slow path: pop all three, clone map, insert, push
        let (stack, value) = pop(stack);
        let (stack, key_val) = pop(stack);
        let key = MapKey::from_value(&key_val).unwrap_or_else(|| {
            panic!(
                "map-set: key must be Int, String, or Bool, got {:?}",
                key_val
            )
        });
        let (stack, map_val) = pop(stack);
        let mut map = match map_val {
            Value::Map(m) => *m,
            _ => panic!("map-set: expected Map, got {:?}", map_val),
        };
        map.insert(key, value);
        push(stack, Value::Map(Box::new(map)))
    }
}

/// Check if a key exists in the map
///
/// Stack effect: ( Map key -- Int )
///
/// Returns 1 if the key exists, 0 otherwise.
/// Panics if the key type is not hashable.
///
/// # Safety
/// Stack must have a hashable key on top and a Map below
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_map_has(stack: Stack) -> Stack {
    unsafe {
        // Pop key
        let (stack, key_val) = pop(stack);
        let key = MapKey::from_value(&key_val).unwrap_or_else(|| {
            panic!(
                "map-has?: key must be Int, String, or Bool, got {:?}",
                key_val
            )
        });

        // Pop map
        let (stack, map_val) = pop(stack);
        let map = match map_val {
            Value::Map(m) => m,
            _ => panic!("map-has?: expected Map, got {:?}", map_val),
        };

        let has_key = map.contains_key(&key);
        push(stack, Value::Bool(has_key))
    }
}

/// Remove a key from the map with COW optimization.
///
/// Stack effect: ( Map key -- Map )
///
/// Fast path: if the map (at sp-2) is sole-owned, pops key and
/// removes directly from the map in place.
/// Slow path: pops both, clones, removes, pushes new map.
///
/// # Safety
/// Stack must have a hashable key on top and a Map below
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_map_remove(stack: Stack) -> Stack {
    unsafe {
        // Fast path: peek at the map at sp-2 without popping.
        // SAFETY: map.remove requires two values on the stack (enforced by
        // the type checker), so stack.sub(2) is valid.
        if let Some(Value::Map(map)) = heap_value_mut(stack.sub(2)) {
            let (stack, key_val) = pop(stack);
            let key = MapKey::from_value(&key_val).unwrap_or_else(|| {
                panic!(
                    "map-remove: key must be Int, String, or Bool, got {:?}",
                    key_val
                )
            });
            // Safety: pop only touches sp-1; the map at the original
            // sp-2 (now sp-1) is not invalidated.
            map.remove(&key);
            return stack; // Map is still at sp-1, mutated in place
        }

        // Slow path: pop both, clone map, remove, push
        let (stack, key_val) = pop(stack);
        let key = MapKey::from_value(&key_val).unwrap_or_else(|| {
            panic!(
                "map-remove: key must be Int, String, or Bool, got {:?}",
                key_val
            )
        });
        let (stack, map_val) = pop(stack);
        let mut map = match map_val {
            Value::Map(m) => *m,
            _ => panic!("map-remove: expected Map, got {:?}", map_val),
        };
        map.remove(&key);
        push(stack, Value::Map(Box::new(map)))
    }
}

/// Get all keys from the map as a list
///
/// Stack effect: ( Map -- Variant )
///
/// Returns a Variant containing all keys in the map.
/// Note: Order is not guaranteed (HashMap iteration order).
///
/// # Safety
/// Stack must have a Map on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_map_keys(stack: Stack) -> Stack {
    unsafe {
        let (stack, map_val) = pop(stack);
        let map = match map_val {
            Value::Map(m) => m,
            _ => panic!("map-keys: expected Map, got {:?}", map_val),
        };

        let keys: Vec<Value> = map.keys().map(|k| k.to_value()).collect();
        let variant = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            keys,
        )));
        push(stack, variant)
    }
}

/// Get all values from the map as a list
///
/// Stack effect: ( Map -- Variant )
///
/// Returns a Variant containing all values in the map.
/// Note: Order is not guaranteed (HashMap iteration order).
///
/// # Safety
/// Stack must have a Map on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_map_values(stack: Stack) -> Stack {
    unsafe {
        let (stack, map_val) = pop(stack);
        let map = match map_val {
            Value::Map(m) => m,
            _ => panic!("map-values: expected Map, got {:?}", map_val),
        };

        let values: Vec<Value> = map.values().cloned().collect();
        let variant = Value::Variant(Arc::new(VariantData::new(
            global_string("List".to_string()),
            values,
        )));
        push(stack, variant)
    }
}

/// Get the number of entries in the map
///
/// Stack effect: ( Map -- Int )
///
/// # Safety
/// Stack must have a Map on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_map_size(stack: Stack) -> Stack {
    unsafe {
        let (stack, map_val) = pop(stack);
        let map = match map_val {
            Value::Map(m) => m,
            _ => panic!("map-size: expected Map, got {:?}", map_val),
        };

        push(stack, Value::Int(map.len() as i64))
    }
}

/// Check if the map is empty
///
/// Stack effect: ( Map -- Int )
///
/// Returns 1 if the map has no entries, 0 otherwise.
///
/// # Safety
/// Stack must have a Map on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_map_empty(stack: Stack) -> Stack {
    unsafe {
        let (stack, map_val) = pop(stack);
        let map = match map_val {
            Value::Map(m) => m,
            _ => panic!("map-empty?: expected Map, got {:?}", map_val),
        };

        let is_empty = map.is_empty();
        push(stack, Value::Bool(is_empty))
    }
}

/// Iterate over all key-value pairs in a map, calling a quotation for each.
///
/// Stack effect: ( Map Quotation -- )
///   where Quotation : ( key value -- )
///
/// The quotation receives each key and value on a fresh stack.
/// Iteration order is not guaranteed.
///
/// # Safety
/// Stack must have a Quotation/Closure on top and a Map below
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_map_each(stack: Stack) -> Stack {
    unsafe {
        // Pop quotation
        let (stack, callable) = pop(stack);
        match &callable {
            Value::Quotation { .. } | Value::Closure { .. } => {}
            _ => panic!(
                "map.each: expected Quotation or Closure, got {:?}",
                callable
            ),
        }

        // Pop map
        let (stack, map_val) = pop(stack);
        let map = match &map_val {
            Value::Map(m) => m,
            _ => panic!("map.each: expected Map, got {:?}", map_val),
        };

        // Call quotation for each key-value pair
        for (key, value) in map.iter() {
            let temp_base = crate::stack::alloc_stack();
            let temp_stack = push(temp_base, key.to_value());
            let temp_stack = push(temp_stack, value.clone());
            let temp_stack = call_callable(temp_stack, &callable);
            // Drain any leftover values
            drain_to_base(temp_stack, temp_base);
        }

        stack
    }
}

/// Fold over all key-value pairs in a map with an accumulator.
///
/// Stack effect: ( Map init Quotation -- result )
///   where Quotation : ( acc key value -- acc' )
///
/// The quotation receives the accumulator, key, and value, and must
/// return the new accumulator. Iteration order is not guaranteed.
///
/// # Safety
/// Stack must have Quotation on top, init below, and Map below that
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_map_fold(stack: Stack) -> Stack {
    unsafe {
        // Pop quotation
        let (stack, callable) = pop(stack);
        match &callable {
            Value::Quotation { .. } | Value::Closure { .. } => {}
            _ => panic!(
                "map.fold: expected Quotation or Closure, got {:?}",
                callable
            ),
        }

        // Pop initial accumulator
        let (stack, mut acc) = pop(stack);

        // Pop map
        let (stack, map_val) = pop(stack);
        let map = match &map_val {
            Value::Map(m) => m,
            _ => panic!("map.fold: expected Map, got {:?}", map_val),
        };

        // Fold over each key-value pair
        for (key, value) in map.iter() {
            let temp_base = crate::stack::alloc_stack();
            let temp_stack = push(temp_base, acc);
            let temp_stack = push(temp_stack, key.to_value());
            let temp_stack = push(temp_stack, value.clone());
            let temp_stack = call_callable(temp_stack, &callable);
            // Pop new accumulator
            if temp_stack <= temp_base {
                panic!("map.fold: quotation consumed accumulator without producing result");
            }
            let (remaining, new_acc) = pop(temp_stack);
            acc = new_acc;
            // Drain any extra values left by the quotation
            if remaining > temp_base {
                drain_to_base(remaining, temp_base);
            }
        }

        push(stack, acc)
    }
}

/// Helper to call a quotation or closure with the current stack.
#[inline]
unsafe fn call_callable(stack: Stack, callable: &Value) -> Stack {
    unsafe {
        match callable {
            Value::Quotation { wrapper, .. } => {
                let fn_ref: unsafe extern "C" fn(Stack) -> Stack = std::mem::transmute(*wrapper);
                fn_ref(stack)
            }
            Value::Closure { fn_ptr, env } => {
                let fn_ref: unsafe extern "C" fn(Stack, *const Value, usize) -> Stack =
                    std::mem::transmute(*fn_ptr);
                fn_ref(stack, env.as_ptr(), env.len())
            }
            _ => unreachable!(),
        }
    }
}

/// Drain stack values back to base, properly freeing heap-allocated values.
unsafe fn drain_to_base(mut stack: Stack, base: Stack) {
    unsafe {
        while stack > base {
            let (rest, sv) = pop_sv(stack);
            drop_stack_value(sv);
            stack = rest;
        }
    }
}

// Public re-exports
pub use patch_seq_make_map as make_map;
pub use patch_seq_map_each as map_each;
pub use patch_seq_map_empty as map_empty;
pub use patch_seq_map_fold as map_fold;
pub use patch_seq_map_get as map_get;
pub use patch_seq_map_has as map_has;
pub use patch_seq_map_keys as map_keys;
pub use patch_seq_map_remove as map_remove;
pub use patch_seq_map_set as map_set;
pub use patch_seq_map_size as map_size;
pub use patch_seq_map_values as map_values;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_map() {
        unsafe {
            let stack = crate::stack::alloc_test_stack();
            let stack = make_map(stack);

            let (_stack, result) = pop(stack);
            match result {
                Value::Map(m) => assert!(m.is_empty()),
                _ => panic!("Expected Map"),
            }
        }
    }

    #[test]
    fn test_map_set_and_get() {
        unsafe {
            let stack = crate::stack::alloc_test_stack();
            let stack = make_map(stack);
            let stack = push(stack, Value::String("name".into()));
            let stack = push(stack, Value::String("Alice".into()));
            let stack = map_set(stack);

            // Get the value back
            let stack = push(stack, Value::String("name".into()));
            let stack = map_get(stack);

            // map_get returns (value Bool)
            let (stack, flag) = pop(stack);
            assert_eq!(flag, Value::Bool(true));
            let (_stack, result) = pop(stack);
            match result {
                Value::String(s) => assert_eq!(s.as_str(), "Alice"),
                _ => panic!("Expected String"),
            }
        }
    }

    #[test]
    fn test_map_set_with_int_key() {
        unsafe {
            let stack = crate::stack::alloc_test_stack();
            let stack = make_map(stack);
            let stack = push(stack, Value::Int(42));
            let stack = push(stack, Value::String("answer".into()));
            let stack = map_set(stack);

            let stack = push(stack, Value::Int(42));
            let stack = map_get(stack);

            // map_get returns (value Bool)
            let (stack, flag) = pop(stack);
            assert_eq!(flag, Value::Bool(true));
            let (_stack, result) = pop(stack);
            match result {
                Value::String(s) => assert_eq!(s.as_str(), "answer"),
                _ => panic!("Expected String"),
            }
        }
    }

    #[test]
    fn test_map_has() {
        unsafe {
            let stack = crate::stack::alloc_test_stack();
            let stack = make_map(stack);
            let stack = push(stack, Value::String("key".into()));
            let stack = push(stack, Value::Int(100));
            let stack = map_set(stack);

            // Check existing key (dup map first since map_has consumes it)
            let stack = crate::stack::dup(stack);
            let stack = push(stack, Value::String("key".into()));
            let stack = map_has(stack);
            let (stack, result) = pop(stack);
            assert_eq!(result, Value::Bool(true));

            // Check non-existing key (map is still on stack)
            let stack = push(stack, Value::String("missing".into()));
            let stack = map_has(stack);
            let (_stack, result) = pop(stack);
            assert_eq!(result, Value::Bool(false));
        }
    }

    #[test]
    fn test_map_remove() {
        unsafe {
            let stack = crate::stack::alloc_test_stack();
            let stack = make_map(stack);
            let stack = push(stack, Value::String("a".into()));
            let stack = push(stack, Value::Int(1));
            let stack = map_set(stack);
            let stack = push(stack, Value::String("b".into()));
            let stack = push(stack, Value::Int(2));
            let stack = map_set(stack);

            // Remove "a"
            let stack = push(stack, Value::String("a".into()));
            let stack = map_remove(stack);

            // Check "a" is gone (dup map first since map_has consumes it)
            let stack = crate::stack::dup(stack);
            let stack = push(stack, Value::String("a".into()));
            let stack = map_has(stack);
            let (stack, result) = pop(stack);
            assert_eq!(result, Value::Bool(false));

            // Check "b" is still there (map is still on stack)
            let stack = push(stack, Value::String("b".into()));
            let stack = map_has(stack);
            let (_stack, result) = pop(stack);
            assert_eq!(result, Value::Bool(true));
        }
    }

    #[test]
    fn test_map_size() {
        unsafe {
            let stack = crate::stack::alloc_test_stack();
            let stack = make_map(stack);

            // Empty map
            let stack = map_size(stack);
            let (stack, result) = pop(stack);
            assert_eq!(result, Value::Int(0));

            // Add entries
            let stack = make_map(stack);
            let stack = push(stack, Value::String("a".into()));
            let stack = push(stack, Value::Int(1));
            let stack = map_set(stack);
            let stack = push(stack, Value::String("b".into()));
            let stack = push(stack, Value::Int(2));
            let stack = map_set(stack);

            let stack = map_size(stack);
            let (_stack, result) = pop(stack);
            assert_eq!(result, Value::Int(2));
        }
    }

    #[test]
    fn test_map_empty() {
        unsafe {
            let stack = crate::stack::alloc_test_stack();
            let stack = make_map(stack);

            let stack = map_empty(stack);
            let (stack, result) = pop(stack);
            assert_eq!(result, Value::Bool(true));

            // Non-empty
            let stack = make_map(stack);
            let stack = push(stack, Value::String("key".into()));
            let stack = push(stack, Value::Int(1));
            let stack = map_set(stack);

            let stack = map_empty(stack);
            let (_stack, result) = pop(stack);
            assert_eq!(result, Value::Bool(false));
        }
    }

    #[test]
    fn test_map_keys_and_values() {
        unsafe {
            let stack = crate::stack::alloc_test_stack();
            let stack = make_map(stack);
            let stack = push(stack, Value::String("x".into()));
            let stack = push(stack, Value::Int(10));
            let stack = map_set(stack);
            let stack = push(stack, Value::String("y".into()));
            let stack = push(stack, Value::Int(20));
            let stack = map_set(stack);

            // Get keys
            let stack = crate::stack::dup(stack); // Keep map for values test
            let stack = map_keys(stack);
            let (stack, keys_result) = pop(stack);
            match keys_result {
                Value::Variant(v) => {
                    assert_eq!(v.fields.len(), 2);
                    // Keys are "x" and "y" but order is not guaranteed
                }
                _ => panic!("Expected Variant"),
            }

            // Get values
            let stack = map_values(stack);
            let (_stack, values_result) = pop(stack);
            match values_result {
                Value::Variant(v) => {
                    assert_eq!(v.fields.len(), 2);
                    // Values are 10 and 20 but order is not guaranteed
                }
                _ => panic!("Expected Variant"),
            }
        }
    }

    #[test]
    fn test_map_get_found() {
        unsafe {
            let stack = crate::stack::alloc_test_stack();
            let stack = make_map(stack);
            let stack = push(stack, Value::String("key".into()));
            let stack = push(stack, Value::Int(42));
            let stack = map_set(stack);

            let stack = push(stack, Value::String("key".into()));
            let stack = map_get(stack);

            let (stack, flag) = pop(stack);
            let (_stack, value) = pop(stack);
            assert_eq!(flag, Value::Bool(true));
            assert_eq!(value, Value::Int(42));
        }
    }

    #[test]
    fn test_map_get_not_found() {
        unsafe {
            let stack = crate::stack::alloc_test_stack();
            let stack = make_map(stack);

            let stack = push(stack, Value::String("missing".into()));
            let stack = map_get(stack);

            let (stack, flag) = pop(stack);
            let (_stack, _value) = pop(stack); // placeholder
            assert_eq!(flag, Value::Bool(false));
        }
    }

    #[test]
    fn test_map_with_bool_key() {
        unsafe {
            let stack = crate::stack::alloc_test_stack();
            let stack = make_map(stack);
            let stack = push(stack, Value::Bool(true));
            let stack = push(stack, Value::String("yes".into()));
            let stack = map_set(stack);
            let stack = push(stack, Value::Bool(false));
            let stack = push(stack, Value::String("no".into()));
            let stack = map_set(stack);

            let stack = push(stack, Value::Bool(true));
            let stack = map_get(stack);
            // map_get returns (value Bool)
            let (stack, flag) = pop(stack);
            assert_eq!(flag, Value::Bool(true));
            let (_stack, result) = pop(stack);
            match result {
                Value::String(s) => assert_eq!(s.as_str(), "yes"),
                _ => panic!("Expected String"),
            }
        }
    }

    #[test]
    fn test_map_key_overwrite() {
        // Test that map-set with existing key overwrites the value
        unsafe {
            let stack = crate::stack::alloc_test_stack();
            let stack = make_map(stack);

            // Set initial value
            let stack = push(stack, Value::String("key".into()));
            let stack = push(stack, Value::Int(100));
            let stack = map_set(stack);

            // Overwrite with new value
            let stack = push(stack, Value::String("key".into()));
            let stack = push(stack, Value::Int(200));
            let stack = map_set(stack);

            // Verify size is still 1 (not 2)
            let stack = crate::stack::dup(stack);
            let stack = map_size(stack);
            let (stack, size) = pop(stack);
            assert_eq!(size, Value::Int(1));

            // Verify value was updated
            let stack = push(stack, Value::String("key".into()));
            let stack = map_get(stack);
            // map_get returns (value Bool)
            let (stack, flag) = pop(stack);
            assert_eq!(flag, Value::Bool(true));
            let (_stack, result) = pop(stack);
            assert_eq!(result, Value::Int(200));
        }
    }

    #[test]
    fn test_map_mixed_key_types() {
        // Test that a single map can have different key types
        unsafe {
            let stack = crate::stack::alloc_test_stack();
            let stack = make_map(stack);

            // Add string key
            let stack = push(stack, Value::String("name".into()));
            let stack = push(stack, Value::String("Alice".into()));
            let stack = map_set(stack);

            // Add integer key
            let stack = push(stack, Value::Int(42));
            let stack = push(stack, Value::String("answer".into()));
            let stack = map_set(stack);

            // Add boolean key
            let stack = push(stack, Value::Bool(true));
            let stack = push(stack, Value::String("yes".into()));
            let stack = map_set(stack);

            // Verify size is 3
            let stack = crate::stack::dup(stack);
            let stack = map_size(stack);
            let (stack, size) = pop(stack);
            assert_eq!(size, Value::Int(3));

            // Verify each key retrieves correct value
            // map_get returns (value Bool)
            let stack = crate::stack::dup(stack);
            let stack = push(stack, Value::String("name".into()));
            let stack = map_get(stack);
            let (stack, flag) = pop(stack);
            assert_eq!(flag, Value::Bool(true));
            let (stack, result) = pop(stack);
            match result {
                Value::String(s) => assert_eq!(s.as_str(), "Alice"),
                _ => panic!("Expected String for name key"),
            }

            let stack = crate::stack::dup(stack);
            let stack = push(stack, Value::Int(42));
            let stack = map_get(stack);
            let (stack, flag) = pop(stack);
            assert_eq!(flag, Value::Bool(true));
            let (stack, result) = pop(stack);
            match result {
                Value::String(s) => assert_eq!(s.as_str(), "answer"),
                _ => panic!("Expected String for int key"),
            }

            let stack = push(stack, Value::Bool(true));
            let stack = map_get(stack);
            let (stack, flag) = pop(stack);
            assert_eq!(flag, Value::Bool(true));
            let (_stack, result) = pop(stack);
            match result {
                Value::String(s) => assert_eq!(s.as_str(), "yes"),
                _ => panic!("Expected String for bool key"),
            }
        }
    }

    // =========================================================================
    // map.fold tests
    // =========================================================================

    #[test]
    fn test_map_fold_empty() {
        // Folding an empty map should return the initial accumulator
        unsafe {
            use crate::quotations::push_quotation;

            let stack = crate::stack::alloc_test_stack();

            // Push empty map
            let stack = make_map(stack);

            // Push initial accumulator
            let stack = push(stack, Value::Int(99));

            // Push a dummy quotation (won't be called for empty map)
            unsafe extern "C" fn noop(stack: Stack) -> Stack {
                stack
            }
            let fn_ptr = noop as usize;
            let stack = push_quotation(stack, fn_ptr, fn_ptr);

            let stack = patch_seq_map_fold(stack);

            let (_stack, result) = pop(stack);
            assert_eq!(result, Value::Int(99));
        }
    }
}
