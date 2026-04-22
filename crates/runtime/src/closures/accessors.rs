//! Type-specific closure environment readers.
//!
//! These exist because the generic `env_get` returns a `Value` by value, which
//! causes FFI ABI trouble on some platforms for large enum variants. The
//! specialized readers either return a primitive (`i64`/`f64`) or push the
//! value directly onto the Seq stack, avoiding the by-value enum return.

use crate::stack::{Stack, push};
use crate::value::Value;

/// Get an Int value from the closure environment
///
/// This is a type-specific helper that avoids passing large Value enums through LLVM IR.
/// Returns primitive i64 instead of Value to avoid FFI issues with by-value enum passing.
///
/// # Safety
/// - env_data must be a valid pointer to an array of Values
/// - env_len must match the actual array length
/// - index must be in bounds [0, env_len)
/// - The value at index must be Value::Int
///
/// # FFI Notes
/// This function is ONLY called from LLVM-generated code, not from external C code.
/// The signature is safe for LLVM IR but would be undefined behavior if called from C
/// with incorrect assumptions about type layout.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_env_get_int(
    env_data: *const Value,
    env_len: usize,
    index: i32,
) -> i64 {
    if env_data.is_null() {
        panic!("env_get_int: null environment pointer");
    }

    if index < 0 {
        panic!("env_get_int: index cannot be negative: {}", index);
    }

    let idx = index as usize;

    if idx >= env_len {
        panic!(
            "env_get_int: index {} out of bounds for environment of size {}",
            index, env_len
        );
    }

    // Access the value at the index
    let value = unsafe { &*env_data.add(idx) };

    match value {
        Value::Int(n) => *n,
        _ => panic!(
            "env_get_int: expected Int at index {}, got {:?}",
            index, value
        ),
    }
}

/// Get a String value from the environment at the given index
///
/// # Safety
/// - env_data must be a valid pointer to an array of Values
/// - env_len must be the actual length of that array
/// - index must be within bounds
/// - The value at index must be a String
///
/// This function returns a SeqString by-value.
/// This is safe for FFI because it's only called from LLVM-generated code, not actual C code.
#[allow(improper_ctypes_definitions)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_env_get_string(
    env_data: *const Value,
    env_len: usize,
    index: i32,
) -> crate::seqstring::SeqString {
    if env_data.is_null() {
        panic!("env_get_string: null environment pointer");
    }

    if index < 0 {
        panic!("env_get_string: index cannot be negative: {}", index);
    }

    let idx = index as usize;

    if idx >= env_len {
        panic!(
            "env_get_string: index {} out of bounds for environment of size {}",
            index, env_len
        );
    }

    // Access the value at the index
    let value = unsafe { &*env_data.add(idx) };

    match value {
        Value::String(s) => s.clone(),
        _ => panic!(
            "env_get_string: expected String at index {}, got {:?}",
            index, value
        ),
    }
}

/// Push a String from the closure environment directly onto the stack
///
/// This combines getting and pushing in one operation to avoid returning
/// SeqString by value through FFI, which has calling convention issues on Linux.
///
/// # Safety
/// - Stack pointer must be valid
/// - env_data must be a valid pointer to an array of Values
/// - env_len must match the actual array length
/// - index must be in bounds [0, env_len)
/// - The value at index must be Value::String
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_env_push_string(
    stack: Stack,
    env_data: *const Value,
    env_len: usize,
    index: i32,
) -> Stack {
    if env_data.is_null() {
        panic!("env_push_string: null environment pointer");
    }

    if index < 0 {
        panic!("env_push_string: index cannot be negative: {}", index);
    }

    let idx = index as usize;

    if idx >= env_len {
        panic!(
            "env_push_string: index {} out of bounds for environment of size {}",
            index, env_len
        );
    }

    // Access the value at the index
    let value = unsafe { &*env_data.add(idx) };

    match value {
        Value::String(s) => unsafe { push(stack, Value::String(s.clone())) },
        _ => panic!(
            "env_push_string: expected String at index {}, got {:?}",
            index, value
        ),
    }
}

/// Push any value from the closure environment onto the stack.
///
/// This is the generic capture-push function for types that don't have
/// specialized getters (Variant, Map, Union, Symbol, Channel). It clones
/// the Value from the env and pushes it directly, avoiding passing Value
/// by value through the FFI boundary (which crashes on Linux for some types).
///
/// # Safety
/// - `stack` must be a valid stack pointer
/// - `env_data` must be a valid pointer to a Value array
/// - `env_len` must match the actual array length
/// - `index` must be in bounds [0, env_len)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_env_push_value(
    stack: Stack,
    env_data: *const Value,
    env_len: usize,
    index: i32,
) -> Stack {
    if env_data.is_null() {
        panic!("env_push_value: null environment pointer");
    }

    if index < 0 {
        panic!("env_push_value: index cannot be negative: {}", index);
    }

    let idx = index as usize;

    if idx >= env_len {
        panic!(
            "env_push_value: index {} out of bounds for environment of size {}",
            index, env_len
        );
    }

    // Clone the value from the environment and push onto the stack.
    // This works for any Value variant (Variant, Map, Symbol, Channel, etc.)
    // The clone is O(1) for Arc-wrapped types (Variant, Map) — just a refcount bump.
    //
    // Primitive types (Int, Bool, Float) should use their specialized getters
    // (env_get_int, etc.) for efficiency. This generic path is for types that
    // don't have specialized LLVM IR representations.
    let value = unsafe { (*env_data.add(idx)).clone() };
    debug_assert!(
        !matches!(value, Value::Int(_) | Value::Bool(_) | Value::Float(_)),
        "env_push_value called for primitive type {:?} — use the specialized getter",
        value
    );
    unsafe { push(stack, value) }
}

/// Get a Bool value from the closure environment
///
/// Returns i64 (0 for false, 1 for true) to match LLVM IR representation.
/// Bools are stored as i64 in the generated code for simplicity.
///
/// # Safety
/// - env_data must be a valid pointer to an array of Values
/// - env_len must match the actual array length
/// - index must be in bounds [0, env_len)
/// - The value at index must be Value::Bool
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_env_get_bool(
    env_data: *const Value,
    env_len: usize,
    index: i32,
) -> i64 {
    if env_data.is_null() {
        panic!("env_get_bool: null environment pointer");
    }

    if index < 0 {
        panic!("env_get_bool: index cannot be negative: {}", index);
    }

    let idx = index as usize;

    if idx >= env_len {
        panic!(
            "env_get_bool: index {} out of bounds for environment of size {}",
            index, env_len
        );
    }

    let value = unsafe { &*env_data.add(idx) };

    match value {
        Value::Bool(b) => {
            if *b {
                1
            } else {
                0
            }
        }
        _ => panic!(
            "env_get_bool: expected Bool at index {}, got {:?}",
            index, value
        ),
    }
}

/// Get a Float value from the closure environment
///
/// Returns f64 directly for efficient LLVM IR integration.
///
/// # Safety
/// - env_data must be a valid pointer to an array of Values
/// - env_len must match the actual array length
/// - index must be in bounds [0, env_len)
/// - The value at index must be Value::Float
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_env_get_float(
    env_data: *const Value,
    env_len: usize,
    index: i32,
) -> f64 {
    if env_data.is_null() {
        panic!("env_get_float: null environment pointer");
    }

    if index < 0 {
        panic!("env_get_float: index cannot be negative: {}", index);
    }

    let idx = index as usize;

    if idx >= env_len {
        panic!(
            "env_get_float: index {} out of bounds for environment of size {}",
            index, env_len
        );
    }

    let value = unsafe { &*env_data.add(idx) };

    match value {
        Value::Float(f) => *f,
        _ => panic!(
            "env_get_float: expected Float at index {}, got {:?}",
            index, value
        ),
    }
}

/// Get a Quotation impl_ function pointer from the closure environment
///
/// Returns i64 (the impl_ function pointer as usize) for LLVM IR.
/// Returns the tailcc impl_ pointer for TCO when called from compiled code.
/// Quotations are stateless, so only the function pointer is needed.
///
/// # Safety
/// - env_data must be a valid pointer to an array of Values
/// - env_len must match the actual array length
/// - index must be in bounds [0, env_len)
/// - The value at index must be Value::Quotation
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_env_get_quotation(
    env_data: *const Value,
    env_len: usize,
    index: i32,
) -> i64 {
    if env_data.is_null() {
        panic!("env_get_quotation: null environment pointer");
    }

    if index < 0 {
        panic!("env_get_quotation: index cannot be negative: {}", index);
    }

    let idx = index as usize;

    if idx >= env_len {
        panic!(
            "env_get_quotation: index {} out of bounds for environment of size {}",
            index, env_len
        );
    }

    let value = unsafe { &*env_data.add(idx) };

    match value {
        Value::Quotation { impl_, .. } => *impl_ as i64,
        _ => panic!(
            "env_get_quotation: expected Quotation at index {}, got {:?}",
            index, value
        ),
    }
}
