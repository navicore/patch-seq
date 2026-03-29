//! Dataflow combinators for Seq
//!
//! Higher-order words that manage value flow on the stack,
//! reducing the need for explicit stack shuffling (swap/rot/pick)
//! or auxiliary stack usage (>aux / aux>).
//!
//! These follow the concatenative tradition from Factor/Joy:
//! - `dip`  — hide top value, run quotation, restore value
//! - `keep` — run quotation on top value, but preserve the original
//! - `bi`   — apply two quotations to the same value

use crate::stack::{Stack, pop, push};
use crate::value::Value;

/// Call a quotation or closure with the given stack.
///
/// This is the shared calling convention used by all combinators.
/// Handles both Quotation (bare function pointer) and Closure
/// (function pointer + captured environment).
///
/// # Safety
/// - Stack must be valid
/// - The callable must be a Quotation or Closure value
#[inline]
unsafe fn invoke(stack: Stack, callable: &Value) -> Stack {
    // SAFETY: Function pointers were created by the compiler's codegen.
    // Quotation wrappers use C calling convention: fn(Stack) -> Stack.
    // Closure functions use: fn(Stack, *const Value, usize) -> Stack.
    // We validate non-null before transmute.
    unsafe {
        match callable {
            Value::Quotation { wrapper, .. } => {
                if *wrapper == 0 {
                    panic!("combinator: quotation function pointer is null");
                }
                let fn_ref: unsafe extern "C" fn(Stack) -> Stack = std::mem::transmute(*wrapper);
                fn_ref(stack)
            }
            Value::Closure { fn_ptr, env } => {
                if *fn_ptr == 0 {
                    panic!("combinator: closure function pointer is null");
                }
                let fn_ref: unsafe extern "C" fn(Stack, *const Value, usize) -> Stack =
                    std::mem::transmute(*fn_ptr);
                fn_ref(stack, env.as_ptr(), env.len())
            }
            _ => panic!(
                "combinator: expected Quotation or Closure, got {:?}",
                callable
            ),
        }
    }
}

/// `dip`: Hide top value, run quotation on the rest, restore value.
///
/// Stack effect: ( ..a x quot -- ..b x )
///   where quot : ( ..a -- ..b )
///
/// Equivalent to: `swap >aux call aux>`
///
/// # Safety
/// - Stack must have at least 2 values (quotation on top, preserved value below)
/// - Top of stack must be a Quotation or Closure
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_dip(stack: Stack) -> Stack {
    // SAFETY: Caller guarantees stack has quotation on top and a value below.
    // invoke's safety is documented above.
    unsafe {
        let (stack, quot) = pop(stack); // pop quotation
        let (stack, x) = pop(stack); // pop preserved value
        let stack = invoke(stack, &quot); // run quotation on remaining stack
        push(stack, x) // restore preserved value
    }
}

/// `keep`: Run quotation on top value, but preserve the original.
///
/// Stack effect: ( ..a x quot -- ..b x )
///   where quot : ( ..a x -- ..b )
///
/// Like `dip`, but the quotation also receives the preserved value.
/// Equivalent to: `over >aux call aux>`
///
/// # Safety
/// - Stack must have at least 2 values (quotation on top, value below)
/// - Top of stack must be a Quotation or Closure
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_keep(stack: Stack) -> Stack {
    // SAFETY: Caller guarantees stack has quotation on top and a value below.
    // x is cloned so both the quotation and the restore get valid values.
    unsafe {
        let (stack, quot) = pop(stack); // pop quotation
        let (stack, x) = pop(stack); // pop value to preserve
        let stack = push(stack, x.clone()); // push copy for quotation to consume
        let stack = invoke(stack, &quot); // run quotation (consumes the copy)
        push(stack, x) // restore original value
    }
}

/// `bi`: Apply two quotations to the same value.
///
/// Stack effect: ( ..a x quot1 quot2 -- ..c )
///   where quot1 : ( ..a x -- ..b )
///         quot2 : ( ..b x -- ..c )
///
/// Equivalent to: `>aux keep aux> call`
///
/// # Safety
/// - Stack must have at least 3 values (quot2 on top, quot1 below, value below that)
/// - Top two stack values must be Quotations or Closures
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_bi(stack: Stack) -> Stack {
    // SAFETY: Caller guarantees stack layout. x is cloned so both
    // quotations receive a valid copy.
    unsafe {
        let (stack, quot2) = pop(stack); // pop second quotation
        let (stack, quot1) = pop(stack); // pop first quotation
        let (stack, x) = pop(stack); // pop value
        let stack = push(stack, x.clone()); // push copy for quot1
        let stack = invoke(stack, &quot1); // run first quotation
        let stack = push(stack, x); // push original for quot2
        invoke(stack, &quot2) // run second quotation
    }
}

// Public re-exports with short names for internal use
pub use patch_seq_bi as bi;
pub use patch_seq_dip as dip;
pub use patch_seq_keep as keep;
