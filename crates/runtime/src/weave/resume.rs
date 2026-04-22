//! Caller-side weave operations: `patch_seq_resume` and `patch_seq_weave_cancel`.
//!
//! Both operate on a `WeaveHandle` (a `Value::WeaveCtx`) received from
//! `strand.weave`, sending on the shared `resume_chan` to drive the woven
//! coroutine forward or tear it down.

use crate::stack::{Stack, pop, push};
use crate::value::{Value, WeaveMessage};
use std::sync::Arc;

/// Resume a woven strand with a value
///
/// Stack effect: ( WeaveHandle a -- WeaveHandle a Bool )
///
/// Sends value `a` to the weave and waits for it to yield.
/// Returns (handle, yielded_value, has_more).
/// - has_more = true: weave yielded a value
/// - has_more = false: weave completed
///
/// # Error Handling
///
/// This function never panics (panicking in extern "C" is UB). On fatal error
/// (null stack, type mismatch), it prints an error and aborts the process.
///
/// # Safety
/// Stack must have a value on top and WeaveHandle below it
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_resume(stack: Stack) -> Stack {
    // Note: We can't use assert! here (it panics). Use abort() for fatal errors.
    if stack.is_null() {
        eprintln!("strand.resume: stack is null (fatal programming error)");
        std::process::abort();
    }

    // Pop the value to send
    let (stack, value) = unsafe { pop(stack) };

    // Pop the WeaveHandle
    let (stack, handle) = unsafe { pop(stack) };

    let (yield_chan, resume_chan) = match &handle {
        Value::WeaveCtx {
            yield_chan,
            resume_chan,
        } => (Arc::clone(yield_chan), Arc::clone(resume_chan)),
        _ => {
            eprintln!("strand.resume: expected WeaveHandle, got {:?}", handle);
            std::process::abort();
        }
    };

    // Wrap value in WeaveMessage for sending
    let msg_to_send = WeaveMessage::Value(value.clone());

    // Send resume value to the weave
    if resume_chan.sender.send(msg_to_send).is_err() {
        // Channel closed - weave is done
        let stack = unsafe { push(stack, handle) };
        let stack = unsafe { push(stack, Value::Int(0)) };
        return unsafe { push(stack, Value::Bool(false)) };
    }

    // Wait for yielded value
    match yield_chan.receiver.recv() {
        Ok(msg) => match msg {
            WeaveMessage::Done => {
                // Weave completed
                let stack = unsafe { push(stack, handle) };
                let stack = unsafe { push(stack, Value::Int(0)) };
                unsafe { push(stack, Value::Bool(false)) }
            }
            WeaveMessage::Value(yielded) => {
                // Normal yield
                let stack = unsafe { push(stack, handle) };
                let stack = unsafe { push(stack, yielded) };
                unsafe { push(stack, Value::Bool(true)) }
            }
            WeaveMessage::Cancel => {
                // Shouldn't happen - Cancel is sent on resume_chan
                let stack = unsafe { push(stack, handle) };
                let stack = unsafe { push(stack, Value::Int(0)) };
                unsafe { push(stack, Value::Bool(false)) }
            }
        },
        Err(_) => {
            // Channel closed unexpectedly
            let stack = unsafe { push(stack, handle) };
            let stack = unsafe { push(stack, Value::Int(0)) };
            unsafe { push(stack, Value::Bool(false)) }
        }
    }
}

/// Cancel a weave, releasing its resources
///
/// Stack effect: ( WeaveHandle -- )
///
/// Sends a cancellation signal to the weave, causing it to exit cleanly.
/// This is necessary to avoid resource leaks when abandoning a weave
/// before it completes naturally.
///
/// If the weave is:
/// - Waiting for first resume: exits immediately
/// - Waiting inside yield: receives cancel signal and can exit
/// - Already completed: no effect (signal is ignored)
///
/// # Error Handling
///
/// This function never panics (panicking in extern "C" is UB). On fatal error
/// (null stack, type mismatch), it prints an error and aborts the process.
///
/// # Safety
/// Stack must have a WeaveHandle (WeaveCtx) on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_weave_cancel(stack: Stack) -> Stack {
    // Note: We can't use assert! here (it panics). Use abort() for fatal errors.
    if stack.is_null() {
        eprintln!("strand.weave-cancel: stack is null (fatal programming error)");
        std::process::abort();
    }

    // Pop the WeaveHandle
    let (stack, handle) = unsafe { pop(stack) };

    // Extract the resume channel to send cancel signal
    match handle {
        Value::WeaveCtx { resume_chan, .. } => {
            // Send cancel signal - if this fails, weave is already done (fine)
            let _ = resume_chan.sender.send(WeaveMessage::Cancel);
        }
        _ => {
            eprintln!(
                "strand.weave-cancel: expected WeaveHandle, got {:?}",
                handle
            );
            std::process::abort();
        }
    }

    // Handle is consumed (dropped), stack returned without it
    stack
}
