//! Closure environment construction and generic access.
//!
//! - `create_env` allocates a fixed-size `Vec<Value>` returned as a boxed slice
//!   pointer for LLVM codegen to populate.
//! - `env_set` fills a slot; each slot is written exactly once before
//!   `make_closure` takes ownership.
//! - `env_get` is the generic read path; type-specific readers live in
//!   `super::accessors`.

use crate::value::Value;

use super::MAX_CAPTURES;

/// Create a closure environment (array of captured values)
///
/// Called from generated LLVM code to allocate space for captured values.
/// Returns a raw pointer to a boxed slice that will be filled with values.
///
/// # Safety
/// - Caller must populate the environment with `env_set` before using
/// - Caller must eventually pass ownership to a Closure value (via `make_closure`)
// Allow improper_ctypes_definitions: Called from LLVM IR (not C), both sides understand layout
#[allow(improper_ctypes_definitions)]
#[unsafe(no_mangle)]
pub extern "C" fn patch_seq_create_env(size: i32) -> *mut [Value] {
    if size < 0 {
        panic!("create_env: size cannot be negative: {}", size);
    }

    let size_usize = size as usize;
    if size_usize > MAX_CAPTURES {
        panic!(
            "create_env: size {} exceeds MAX_CAPTURES ({})",
            size_usize, MAX_CAPTURES
        );
    }

    let mut vec: Vec<Value> = Vec::with_capacity(size_usize);

    // Fill with placeholder values (will be replaced by env_set)
    for _ in 0..size {
        vec.push(Value::Int(0));
    }

    Box::into_raw(vec.into_boxed_slice())
}

/// Set a value in the closure environment
///
/// Called from generated LLVM code to populate captured values.
///
/// # Safety
/// - env must be a valid pointer from `create_env`
/// - index must be in bounds [0, size)
/// - env must not have been passed to `make_closure` yet
// Allow improper_ctypes_definitions: Called from LLVM IR (not C), both sides understand layout
#[allow(improper_ctypes_definitions)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_env_set(env: *mut [Value], index: i32, value: Value) {
    if env.is_null() {
        panic!("env_set: null environment pointer");
    }

    if index < 0 {
        panic!("env_set: index cannot be negative: {}", index);
    }

    let env_slice = unsafe { &mut *env };
    let idx = index as usize;

    if idx >= env_slice.len() {
        panic!(
            "env_set: index {} out of bounds for environment of size {}",
            index,
            env_slice.len()
        );
    }

    env_slice[idx] = value;
}

/// Get a value from the closure environment
///
/// Called from generated closure function code to access captured values.
/// Takes environment as separate data pointer and length (since LLVM can't handle fat pointers).
///
/// # Safety
/// - env_data must be a valid pointer to an array of Values
/// - env_len must match the actual array length
/// - index must be in bounds [0, env_len)
// Allow improper_ctypes_definitions: Called from LLVM IR (not C), both sides understand layout
#[allow(improper_ctypes_definitions)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_env_get(
    env_data: *const Value,
    env_len: usize,
    index: i32,
) -> Value {
    if env_data.is_null() {
        panic!("env_get: null environment pointer");
    }

    if index < 0 {
        panic!("env_get: index cannot be negative: {}", index);
    }

    let idx = index as usize;

    if idx >= env_len {
        panic!(
            "env_get: index {} out of bounds for environment of size {}",
            index, env_len
        );
    }

    // Clone the value from the environment
    unsafe { (*env_data.add(idx)).clone() }
}
