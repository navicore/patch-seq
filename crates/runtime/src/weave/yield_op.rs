//! Weave-side yield: `patch_seq_yield` sends a value back through the weave
//! context and awaits the next resume value.

use crate::stack::{Stack, pop, push};
use crate::value::{Value, WeaveMessage};
use std::sync::Arc;

use super::strand_lifecycle::{block_forever, cleanup_strand};

/// Yield a value from within a woven strand
///
/// Stack effect: ( WeaveCtx a -- WeaveCtx a )
///
/// Sends value `a` to the caller and waits for the next resume value.
/// The WeaveCtx must be passed through - it contains the channels.
///
/// # Error Handling
///
/// This function never panics (panicking in extern "C" is UB). On error:
/// - Type mismatch: eprintln + cleanup + block forever
/// - Channel closed: cleanup + block forever
///
/// The coroutine is marked as completed before blocking, so the program
/// can still terminate normally.
///
/// # Safety
/// Stack must have a value on top and WeaveCtx below it
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_yield(stack: Stack) -> Stack {
    // Note: We can't use assert! here (it panics). A null stack is a fatal
    // programming error, but we handle it gracefully to avoid UB.
    if stack.is_null() {
        eprintln!("yield: stack is null (fatal programming error)");
        crate::arena::arena_reset();
        cleanup_strand();
        block_forever();
    }

    // Pop the value to yield
    let (stack, value) = unsafe { pop(stack) };

    // Pop the WeaveCtx
    let (stack, ctx) = unsafe { pop(stack) };

    let (yield_chan, resume_chan) = match &ctx {
        Value::WeaveCtx {
            yield_chan,
            resume_chan,
        } => (Arc::clone(yield_chan), Arc::clone(resume_chan)),
        _ => {
            // Type mismatch - yield called without WeaveCtx on stack
            // This is a programming error but we can't panic (UB)
            eprintln!(
                "yield: expected WeaveCtx on stack, got {:?}. \
                 yield can only be called inside strand.weave with context threaded through.",
                ctx
            );
            crate::arena::arena_reset();
            cleanup_strand();
            block_forever();
        }
    };

    // Wrap value in WeaveMessage for sending
    let msg_to_send = WeaveMessage::Value(value.clone());

    // Send the yielded value
    if yield_chan.sender.send(msg_to_send).is_err() {
        // Channel unexpectedly closed - caller dropped the handle
        // Clean up and block forever (can't panic in extern "C")
        // We're still active here, so call cleanup_strand
        crate::arena::arena_reset();
        cleanup_strand();
        block_forever();
    }

    // IMPORTANT: Become "dormant" before waiting for resume (fixes #287)
    // This allows the scheduler to exit if the program ends while we're waiting.
    // We'll re-activate after receiving the resume value.
    use crate::scheduler::ACTIVE_STRANDS;
    use std::sync::atomic::Ordering;
    ACTIVE_STRANDS.fetch_sub(1, Ordering::AcqRel);

    // Wait for resume value (we're dormant now - not counted as active)
    let resume_msg = match resume_chan.receiver.recv() {
        Ok(msg) => msg,
        Err(_) => {
            // Resume channel closed - caller dropped the handle
            // We're already dormant (decremented above), don't call cleanup_strand
            crate::arena::arena_reset();
            block_forever();
        }
    };

    // Handle the message
    match resume_msg {
        WeaveMessage::Cancel => {
            // Weave was cancelled - signal completion and exit cleanly
            // We're already dormant (decremented above), don't call cleanup_strand
            let _ = yield_chan.sender.send(WeaveMessage::Done);
            crate::arena::arena_reset();
            block_forever();
        }
        WeaveMessage::Value(resume_value) => {
            // Re-activate: we're about to run user code again
            // Use AcqRel for consistency with the decrement above
            ACTIVE_STRANDS.fetch_add(1, Ordering::AcqRel);

            // Push WeaveCtx back, then resume value
            let stack = unsafe { push(stack, ctx) };
            unsafe { push(stack, resume_value) }
        }
        WeaveMessage::Done => {
            // Protocol error - Done should only be sent on yield_chan
            // We're already dormant (decremented above), don't call cleanup_strand
            crate::arena::arena_reset();
            block_forever();
        }
    }
}
