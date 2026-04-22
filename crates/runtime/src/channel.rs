//! Channel operations for CSP-style concurrency
//!
//! Channels are the primary communication mechanism between strands.
//! They use May's MPMC channels with cooperative blocking.
//!
//! ## Zero-Mutex Design
//!
//! Channels are passed directly as `Value::Channel` on the stack. There is NO
//! global registry and NO mutex contention. Send/receive operations work directly
//! on the channel handles with zero locking overhead.
//!
//! ## Non-Blocking Guarantee
//!
//! All channel operations (`send`, `receive`) cooperatively block using May's scheduler.
//! They NEVER block OS threads - May handles scheduling other strands while waiting.
//!
//! ## Multi-Consumer Support
//!
//! Channels support multiple producers AND multiple consumers (MPMC). Multiple strands
//! can receive from the same channel concurrently - each message is delivered to exactly
//! one receiver (work-stealing semantics).
//!
//! ## Stack Effects
//!
//! - `chan.make`: ( -- Channel ) - creates a new channel
//! - `chan.send`: ( value Channel -- Bool ) - sends value, returns success
//! - `chan.receive`: ( Channel -- value Bool ) - receives value and success flag
//!
//! ## Error Handling
//!
//! All operations return success flags - errors are values, not crashes:
//!
//! - `chan.send`: ( value Channel -- Bool ) - returns true on success, false on closed
//! - `chan.receive`: ( Channel -- value Bool ) - returns value and success flag

use crate::stack::{Stack, pop, push};
use crate::value::{ChannelData, Value};
use may::sync::mpmc;
use std::sync::Arc;

#[cfg(feature = "diagnostics")]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "diagnostics")]
pub static TOTAL_MESSAGES_SENT: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "diagnostics")]
pub static TOTAL_MESSAGES_RECEIVED: AtomicU64 = AtomicU64::new(0);

/// Create a new channel
///
/// Stack effect: ( -- Channel )
///
/// Returns a Channel value that can be used with send/receive operations.
/// The channel can be duplicated (dup) to share between strands.
///
/// # Safety
/// Always safe to call
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_channel(stack: Stack) -> Stack {
    // Create an unbounded MPMC channel
    // May's mpmc::channel() creates coroutine-aware channels with multi-producer, multi-consumer
    // The recv() operation cooperatively blocks (yields) instead of blocking the OS thread
    let (sender, receiver) = mpmc::channel();

    // Wrap in Arc<ChannelData> and push directly - NO registry, NO mutex
    let channel = Arc::new(ChannelData { sender, receiver });

    unsafe { push(stack, Value::Channel(channel)) }
}

/// Close a channel (drop it from the stack)
///
/// Stack effect: ( Channel -- )
///
/// Simply drops the channel. When all references are dropped, the channel is closed.
/// This is provided for API compatibility but is equivalent to `drop`.
///
/// # Safety
/// Stack must have a Channel on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_close_channel(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "close_channel: stack is empty");

    // Pop and drop the channel
    let (rest, channel_value) = unsafe { pop(stack) };
    match channel_value {
        Value::Channel(_) => {} // Drop occurs here
        _ => panic!(
            "close_channel: expected Channel on stack, got {:?}",
            channel_value
        ),
    }

    rest
}

/// Send a value through a channel
///
/// Stack effect: ( value Channel -- Bool )
///
/// Returns true on success, false on failure (closed channel).
/// Errors are values, not crashes.
///
/// # Safety
/// Stack must have a Channel on top and a value below it
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_chan_send(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "chan.send: stack is empty");

    // Pop channel
    let (stack, channel_value) = unsafe { pop(stack) };
    let channel = match channel_value {
        Value::Channel(ch) => ch,
        _ => {
            // Wrong type - consume value and return failure
            if !stack.is_null() {
                let (rest, _value) = unsafe { pop(stack) };
                return unsafe { push(rest, Value::Bool(false)) };
            }
            return unsafe { push(stack, Value::Bool(false)) };
        }
    };

    if stack.is_null() {
        // No value to send - return failure
        return unsafe { push(stack, Value::Bool(false)) };
    }

    // Pop value to send
    let (rest, value) = unsafe { pop(stack) };

    // Clone the value before sending
    let global_value = value.clone();

    // Send the value
    match channel.sender.send(global_value) {
        Ok(()) => {
            #[cfg(feature = "diagnostics")]
            TOTAL_MESSAGES_SENT.fetch_add(1, Ordering::Relaxed);
            unsafe { push(rest, Value::Bool(true)) }
        }
        Err(_) => unsafe { push(rest, Value::Bool(false)) },
    }
}

/// Receive a value from a channel
///
/// Stack effect: ( Channel -- value Bool )
///
/// Returns (value, true) on success, (0, false) on failure (closed channel).
/// Errors are values, not crashes.
///
/// ## Multi-Consumer Support
///
/// Multiple strands can receive from the same channel concurrently (MPMC).
/// Each message is delivered to exactly one receiver (work-stealing semantics).
///
/// # Safety
/// Stack must have a Channel on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_chan_receive(stack: Stack) -> Stack {
    assert!(!stack.is_null(), "chan.receive: stack is empty");

    // Pop channel
    let (rest, channel_value) = unsafe { pop(stack) };
    let channel = match channel_value {
        Value::Channel(ch) => ch,
        _ => {
            // Wrong type - return failure
            let stack = unsafe { push(rest, Value::Int(0)) };
            return unsafe { push(stack, Value::Bool(false)) };
        }
    };

    // Receive a value
    match channel.receiver.recv() {
        Ok(value) => {
            #[cfg(feature = "diagnostics")]
            TOTAL_MESSAGES_RECEIVED.fetch_add(1, Ordering::Relaxed);
            let stack = unsafe { push(rest, value) };
            unsafe { push(stack, Value::Bool(true)) }
        }
        Err(_) => {
            let stack = unsafe { push(rest, Value::Int(0)) };
            unsafe { push(stack, Value::Bool(false)) }
        }
    }
}

// Public re-exports with short names for internal use
pub use patch_seq_chan_receive as receive;
pub use patch_seq_chan_send as send;
pub use patch_seq_close_channel as close_channel;
pub use patch_seq_make_channel as make_channel;

#[cfg(test)]
mod tests;
