//! Weave creation: `patch_seq_weave` spawns a coroutine from a quotation or
//! closure and returns a handle the caller uses with `strand.resume`.

use crate::stack::{Stack, pop, push};
use crate::tagged_stack::StackValue;
use crate::value::{Value, WeaveChannelData, WeaveMessage};
use may::sync::mpmc;
use std::sync::Arc;

use super::strand_lifecycle::cleanup_strand;

/// Create a woven strand from a quotation
///
/// Stack effect: ( Quotation -- WeaveHandle )
///
/// Creates a weave from the quotation. The weave is initially suspended,
/// waiting to be resumed with the first value. The quotation will receive
/// a WeaveCtx on its stack that it must pass to yield operations.
///
/// Returns a WeaveHandle that the caller uses with strand.resume.
///
/// # Error Handling
///
/// This function never panics (panicking in extern "C" is UB). On fatal error
/// (null stack, null function pointer, type mismatch), it prints an error
/// and aborts the process.
///
/// # Safety
/// Stack must have a Quotation on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_weave(stack: Stack) -> Stack {
    // Note: We can't use assert! here (it panics). Use abort() for fatal errors.
    if stack.is_null() {
        eprintln!("strand.weave: stack is null (fatal programming error)");
        std::process::abort();
    }

    // Create the two internal channels - NO registry, just Arc values
    // Uses WeaveMessage for type-safe control flow (no sentinel values)
    let (yield_sender, yield_receiver) = mpmc::channel();
    let yield_chan = Arc::new(WeaveChannelData {
        sender: yield_sender,
        receiver: yield_receiver,
    });

    let (resume_sender, resume_receiver) = mpmc::channel();
    let resume_chan = Arc::new(WeaveChannelData {
        sender: resume_sender,
        receiver: resume_receiver,
    });

    // Pop the quotation from stack
    let (stack, quot_value) = unsafe { pop(stack) };

    // Clone channels for the spawned strand's WeaveCtx
    let weave_ctx_yield = Arc::clone(&yield_chan);
    let weave_ctx_resume = Arc::clone(&resume_chan);

    // Clone for the WeaveHandle returned to caller
    let handle_yield = Arc::clone(&yield_chan);
    let handle_resume = Arc::clone(&resume_chan);

    match quot_value {
        Value::Quotation { wrapper, .. } => {
            if wrapper == 0 {
                eprintln!(
                    "strand.weave: quotation wrapper function pointer is null (compiler bug)"
                );
                std::process::abort();
            }

            use crate::scheduler::ACTIVE_STRANDS;
            use may::coroutine;
            use std::sync::atomic::Ordering;

            let fn_ptr: extern "C" fn(Stack) -> Stack = unsafe { std::mem::transmute(wrapper) };

            // Clone the stack for the child
            let (child_stack, child_base) = unsafe { crate::stack::clone_stack_with_base(stack) };

            // Convert pointers to usize (which is Send)
            let stack_addr = child_stack as usize;
            let base_addr = child_base as usize;

            // NOTE: We do NOT increment ACTIVE_STRANDS here!
            // The weave is "dormant" until first resume. This allows the scheduler
            // to exit cleanly if a weave is created but never resumed (fixes #287).
            // ACTIVE_STRANDS is incremented only after receiving the first resume.

            unsafe {
                coroutine::spawn(move || {
                    let child_stack = stack_addr as *mut StackValue;
                    let child_base = base_addr as *mut StackValue;

                    if !child_base.is_null() {
                        crate::stack::patch_seq_set_stack_base(child_base);
                    }

                    // Wait for first resume value before executing
                    // The weave is dormant at this point - not counted in ACTIVE_STRANDS
                    let first_msg = match weave_ctx_resume.receiver.recv() {
                        Ok(msg) => msg,
                        Err(_) => {
                            // Channel closed before we were resumed - just exit
                            // Don't call cleanup_strand since we never activated
                            return;
                        }
                    };

                    // Check for cancellation before starting
                    let first_value = match first_msg {
                        WeaveMessage::Cancel => {
                            // Weave was cancelled before it started - clean exit
                            // Don't call cleanup_strand since we never activated
                            crate::arena::arena_reset();
                            return;
                        }
                        WeaveMessage::Value(v) => v,
                        WeaveMessage::Done => {
                            // Shouldn't happen - Done is sent on yield_chan
                            // Don't call cleanup_strand since we never activated
                            return;
                        }
                    };

                    // NOW we're activated - increment ACTIVE_STRANDS
                    // From this point on, we must call cleanup_strand on exit
                    ACTIVE_STRANDS.fetch_add(1, Ordering::Release);

                    // Push WeaveCtx onto stack (yield_chan, resume_chan as a pair)
                    let weave_ctx = Value::WeaveCtx {
                        yield_chan: weave_ctx_yield.clone(),
                        resume_chan: weave_ctx_resume.clone(),
                    };
                    let stack_with_ctx = push(child_stack, weave_ctx);

                    // Push the first resume value
                    let stack_with_value = push(stack_with_ctx, first_value);

                    // Execute the quotation - it receives (WeaveCtx, resume_value)
                    let final_stack = fn_ptr(stack_with_value);

                    // Quotation returned - pop WeaveCtx and signal completion
                    let (_, ctx_value) = pop(final_stack);
                    if let Value::WeaveCtx { yield_chan, .. } = ctx_value {
                        let _ = yield_chan.sender.send(WeaveMessage::Done);
                    }

                    crate::arena::arena_reset();
                    cleanup_strand();
                });
            }
        }
        Value::Closure { fn_ptr, env } => {
            if fn_ptr == 0 {
                eprintln!("strand.weave: closure function pointer is null (compiler bug)");
                std::process::abort();
            }

            use crate::scheduler::ACTIVE_STRANDS;
            use may::coroutine;
            use std::sync::atomic::Ordering;

            let fn_ref: extern "C" fn(Stack, *const Value, usize) -> Stack =
                unsafe { std::mem::transmute(fn_ptr) };
            let env_clone: Vec<Value> = env.iter().cloned().collect();

            let child_base = crate::stack::alloc_stack();
            let base_addr = child_base as usize;

            // NOTE: We do NOT increment ACTIVE_STRANDS here!
            // The weave is "dormant" until first resume. This allows the scheduler
            // to exit cleanly if a weave is created but never resumed (fixes #287).
            // ACTIVE_STRANDS is incremented only after receiving the first resume.

            unsafe {
                coroutine::spawn(move || {
                    let child_base = base_addr as *mut StackValue;
                    crate::stack::patch_seq_set_stack_base(child_base);

                    // Wait for first resume value
                    // The weave is dormant at this point - not counted in ACTIVE_STRANDS
                    let first_msg = match weave_ctx_resume.receiver.recv() {
                        Ok(msg) => msg,
                        Err(_) => {
                            // Channel closed before we were resumed - just exit
                            // Don't call cleanup_strand since we never activated
                            return;
                        }
                    };

                    // Check for cancellation before starting
                    let first_value = match first_msg {
                        WeaveMessage::Cancel => {
                            // Weave was cancelled before it started - clean exit
                            // Don't call cleanup_strand since we never activated
                            crate::arena::arena_reset();
                            return;
                        }
                        WeaveMessage::Value(v) => v,
                        WeaveMessage::Done => {
                            // Shouldn't happen - Done is sent on yield_chan
                            // Don't call cleanup_strand since we never activated
                            return;
                        }
                    };

                    // NOW we're activated - increment ACTIVE_STRANDS
                    // From this point on, we must call cleanup_strand on exit
                    ACTIVE_STRANDS.fetch_add(1, Ordering::Release);

                    // Push WeaveCtx onto stack
                    let weave_ctx = Value::WeaveCtx {
                        yield_chan: weave_ctx_yield.clone(),
                        resume_chan: weave_ctx_resume.clone(),
                    };
                    let stack_with_ctx = push(child_base, weave_ctx);
                    let stack_with_value = push(stack_with_ctx, first_value);

                    // Execute the closure
                    let final_stack = fn_ref(stack_with_value, env_clone.as_ptr(), env_clone.len());

                    // Signal completion
                    let (_, ctx_value) = pop(final_stack);
                    if let Value::WeaveCtx { yield_chan, .. } = ctx_value {
                        let _ = yield_chan.sender.send(WeaveMessage::Done);
                    }

                    crate::arena::arena_reset();
                    cleanup_strand();
                });
            }
        }
        _ => {
            eprintln!(
                "strand.weave: expected Quotation or Closure, got {:?} (compiler bug or memory corruption)",
                quot_value
            );
            std::process::abort();
        }
    }

    // Return WeaveHandle (contains both channels for resume to use)
    let handle = Value::WeaveCtx {
        yield_chan: handle_yield,
        resume_chan: handle_resume,
    };
    unsafe { push(stack, handle) }
}
