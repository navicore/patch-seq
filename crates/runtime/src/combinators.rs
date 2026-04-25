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
//! - `if`   — branch on a Bool, invoking one of two quotations

use crate::quotations::invoke_callable;
use crate::stack::{Stack, pop, push};
use crate::value::Value;

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
    // invoke_callable's safety is documented in quotations.rs.
    unsafe {
        let (stack, quot) = pop(stack); // pop quotation
        let (stack, x) = pop(stack); // pop preserved value
        let stack = invoke_callable(stack, &quot); // run quotation on remaining stack
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
        let stack = invoke_callable(stack, &quot); // run quotation (consumes the copy)
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
        let stack = invoke_callable(stack, &quot1); // run first quotation
        let stack = push(stack, x); // push original for quot2
        invoke_callable(stack, &quot2) // run second quotation
    }
}

/// `if`: Branch on a Bool, invoking one of two quotations.
///
/// Stack effect: ( ..a Bool [..a -- ..b] [..a -- ..b] -- ..b )
///   The two quotations must have identical effects (the typechecker
///   enforces this); whichever runs leaves the stack in the same shape.
///
/// Layout at entry (top → bottom): else-quot, then-quot, cond.
///
/// # Safety
/// - Stack must have at least 3 values (else-quot on top, then-quot below,
///   Bool below that).
/// - The top two values must be Quotations or Closures.
/// - The third value must be a Bool.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_if(stack: Stack) -> Stack {
    // SAFETY: Caller guarantees the stack layout above. invoke_callable's
    // safety contract is documented in quotations.rs.
    unsafe {
        let (stack, else_quot) = pop(stack);
        let (stack, then_quot) = pop(stack);
        let (stack, cond) = pop(stack);
        match cond {
            Value::Bool(true) => invoke_callable(stack, &then_quot),
            Value::Bool(false) => invoke_callable(stack, &else_quot),
            other => panic!("if: expected Bool condition, got {:?}", other),
        }
    }
}

// Public re-exports with short names for internal use
pub use patch_seq_bi as bi;
pub use patch_seq_dip as dip;
pub use patch_seq_if as if_combinator;
pub use patch_seq_keep as keep;
