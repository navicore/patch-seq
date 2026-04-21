//! Register-Based Specialization for Seq Compiler
//!
//! This module generates optimized register-based LLVM IR for words that operate
//! purely on primitive types (Int, Float, Bool), eliminating the tagged pointer
//! overhead at function boundaries.
//!
//! ## Performance
//!
//! Specialization achieves **8-11x speedup** for numeric-intensive code:
//! - `fib(35)` benchmark: 124ms (stack) → 15ms (specialized)
//! - Recursive calls use `musttail` for guaranteed tail call optimization
//! - 1M recursive calls execute without stack overflow
//!
//! ## How It Works
//!
//! The standard Seq calling convention passes a pointer to a heap-allocated stack
//! of 8-byte tagged pointers. This is flexible but expensive for primitive
//! operations due to tagging/untagging and heap boxing.
//!
//! Specialization detects words that only use primitives and generates a parallel
//! "fast path" that passes values directly in CPU registers:
//!
//! ```llvm
//! ; Fast path - values in registers, no memory access
//! define i64 @seq_fib_i64(i64 %n) {
//!   %cmp = icmp slt i64 %n, 2
//!   br i1 %cmp, label %base, label %recurse
//! base:
//!   ret i64 %n
//! recurse:
//!   %n1 = sub i64 %n, 1
//!   %r1 = musttail call i64 @seq_fib_i64(i64 %n1)
//!   ; ...
//! }
//!
//! ; Fallback - always generated for polymorphic call sites
//! define tailcc ptr @seq_fib(ptr %stack) { ... }
//! ```
//!
//! ## Call Site Dispatch
//!
//! At call sites, the compiler checks if:
//! 1. A specialized version exists for the called word
//! 2. The virtual stack contains values matching the expected types
//!
//! If both conditions are met, it emits a direct register-based call.
//! Otherwise, it falls back to the stack-based version.
//!
//! ## Eligibility
//!
//! A word is specializable if:
//! - Its declared effect has only Int/Float/Bool in inputs/outputs
//! - Its body has no quotations, strings, or symbols (which need heap allocation)
//! - All calls are to inline ops, other specializable words, or recursive self-calls
//! - It has exactly one output (multiple outputs require struct returns - future work)
//!
//! ## Supported Operations (65 total)
//!
//! - **Integer arithmetic**: i.+, i.-, i.*, i./, i.% (with division-by-zero checks)
//! - **Float arithmetic**: f.+, f.-, f.*, f./
//! - **Comparisons**: i.<, i.>, i.<=, i.>=, i.=, i.<>, f.<, f.>, etc.
//! - **Bitwise**: band, bor, bxor, bnot, shl, shr (with bounds checking)
//! - **Bit counting**: popcount, clz, ctz (using LLVM intrinsics)
//! - **Boolean**: and, or, not
//! - **Type conversions**: int->float, float->int
//! - **Stack ops**: dup, drop, swap, over, rot, nip, tuck, pick, roll
//!
//! ## Implementation Notes
//!
//! ### RegisterContext
//! Tracks SSA variable names instead of emitting stack operations. Stack shuffles
//! like `swap` and `rot` become free context manipulations.
//!
//! ### Safe Division
//! Division and modulo emit branch-based zero checks with phi nodes, returning
//! both the result and a success flag to maintain Seq's safe division semantics.
//!
//! ### Safe Shifts
//! Shift operations check for out-of-bounds shift amounts (negative or >= 64)
//! and return 0 for invalid shifts, matching Seq's defined behavior.
//!
//! ### Tail Call Optimization
//! Recursive calls use `musttail` to guarantee TCO. This is critical for
//! recursive algorithms that would otherwise overflow the call stack.
//!
//! ## Module Layout
//!
//! - `types`          — `RegisterType`, `SpecSignature`
//! - `context`        — `RegisterContext` SSA stack
//! - `eligibility`    — `can_specialize` and the op allowlist
//! - `codegen_word`   — function prologue, statement dispatch, `if`, return
//! - `codegen_ops`    — per-word-call lowering (arith, compare, stack, …)
//! - `codegen_safe_math` — safe division and shift with phi nodes
//! - `codegen_calls`  — recursive and cross-word specialized calls
//!
//! ## Future Work
//!
//! - **Multiple outputs**: Words returning multiple values could use LLVM struct
//!   returns `{ i64, i64 }`, but this requires changing how callers unpack results.

mod codegen_calls;
mod codegen_ops;
mod codegen_safe_math;
mod codegen_word;
mod context;
mod eligibility;
mod types;

use super::CodeGen;

pub use types::{RegisterType, SpecSignature};
