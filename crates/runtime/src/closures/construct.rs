//! Closure value construction: `make_closure` (wraps a pre-built env) and
//! `push_closure` (pops captures off the stack, builds the env, pushes the
//! resulting closure).

use crate::stack::{Stack, pop, push};
use crate::value::Value;
use std::sync::Arc;

/// Create a closure value from a function pointer and environment
///
/// Takes ownership of the environment (converts raw pointer to Arc).
/// Arc enables TCO: no cleanup needed after tail calls.
///
/// # Safety
/// - fn_ptr must be a valid function pointer (will be transmuted when called)
/// - env must be a valid pointer from `create_env`, fully populated via `env_set`
/// - env ownership is transferred to the Closure value
// Allow improper_ctypes_definitions: Called from LLVM IR (not C), both sides understand layout
#[allow(improper_ctypes_definitions)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_closure(fn_ptr: u64, env: *mut [Value]) -> Value {
    if fn_ptr == 0 {
        panic!("make_closure: null function pointer");
    }

    if env.is_null() {
        panic!("make_closure: null environment pointer");
    }

    // Take ownership of the environment and convert to Arc for TCO support
    let env_box = unsafe { Box::from_raw(env) };
    let env_arc: Arc<[Value]> = Arc::from(env_box);

    Value::Closure {
        fn_ptr: fn_ptr as usize,
        env: env_arc,
    }
}

/// Create closure from function pointer and stack values (all-in-one helper)
///
/// Pops `capture_count` values from stack and creates a closure environment
/// indexed bottom-to-top: env[0] is the caller's deepest capture,
/// env[N-1] is the caller's shallowest (the value that was on top just
/// before this call). This matches the typechecker's capture-type vector
/// and preserves the caller's visual stack order inside the closure body.
///
/// # Safety
/// - fn_ptr must be a valid function pointer
/// - stack must have at least `capture_count` values
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_push_closure(
    mut stack: Stack,
    fn_ptr: u64,
    capture_count: i32,
) -> Stack {
    if fn_ptr == 0 {
        panic!("push_closure: null function pointer");
    }

    if capture_count < 0 {
        panic!(
            "push_closure: capture_count cannot be negative: {}",
            capture_count
        );
    }

    let count = capture_count as usize;

    // Pop values from stack top-down, then reverse so env is bottom-to-top.
    // Index 0 corresponds to the deepest caller capture; the codegen pushes
    // env[0..N-1] in order at closure entry, leaving the caller's shallowest
    // capture on top of the body's stack — matching the caller's visual order.
    let mut captures: Vec<Value> = Vec::with_capacity(count);
    for _ in 0..count {
        let (new_stack, value) = unsafe { pop(stack) };
        captures.push(value);
        stack = new_stack;
    }
    captures.reverse();

    // Create closure value with Arc for TCO support
    let closure = Value::Closure {
        fn_ptr: fn_ptr as usize,
        env: Arc::from(captures.into_boxed_slice()),
    };

    // Push onto stack
    unsafe { push(stack, closure) }
}
