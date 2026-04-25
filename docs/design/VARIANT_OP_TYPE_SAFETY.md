# Variant-op Type Safety + Locatable Runtime Panics

Status: design · 2026-04-25

## Intent

Today the compiler accepts `"alpha beta gamma" 0 variant.field-at` without
complaint. The program builds, then panics in `runtime/src/variant_ops/access.rs`
with no source location, leaving the user nothing to grep for. Two distinct
holes:

1. **Type checker can't distinguish "any value" from "a variant."** All
   `variant.*` builtins are typed `(a V Int -- a T)` where `V` is a free
   type variable, so `String` unifies with `V` happily.
2. **Runtime panics carry no `.seq` source location.** Even when a panic
   *is* the right answer (e.g. div-by-zero in older code paths, future
   FFI failures), the user sees Rust file/line, not Seq file/line.

Goal: catch shape-1 errors at compile time, and make any panic that
*does* slip through self-locating in the source file. Both must respect
"the fast path stays fast" (ROADMAP.md).

## Constraints

- **No measurable runtime overhead on the hot path.** This is
  load-bearing. Any solution that adds a per-builtin-call cost must be
  zero or near-zero (e.g. one `store i64` to a thread-local, only at
  fallible builtins) — and only if no zero-cost path exists.
- **No new user-visible syntax.** Users don't push spans onto the stack.
- **Don't weaken the type system or break existing union/match flow.**
  `Type::Union("Message")` keeps working; `match` exhaustiveness keeps
  working; auto-generated `Make-X` constructors keep working.
- **No generics.** (`feedback_no_generics.md` is firm: lint-based safety
  preferred, but here a *kind* constraint on one type parameter is the
  minimum that closes the hole — it isn't generics in the user-facing
  sense.)
- **Out of scope:** rejecting all runtime panics (some FFI calls and OOM
  always can); coloured output; structured panic protocol.

## Approach

### Part A — compile-time check (primary fix)

Two viable shapes; pick one:

1. **Introduce `Type::Variant`** as an anonymous "this is a tagged
   variant value" type. `variant.make-N` and `Make-X` constructors
   return `Variant`. `Type::Union(name)` unifies with `Variant`
   (Union-is-a-Variant, one direction). All `variant.*` ops change
   their signature from `(a V Int -- a T)` to `(a Variant Int -- a T)`.
   Cost: one new `Type` enum arm + one unification rule + signature
   updates in `builtins/adt.rs`.
2. **Kinded type variables.** `Type::Var("V")` gains an optional kind
   tag; `variant.*` signatures use `Type::VarKinded("V", Kind::Variant)`.
   Unification rejects `String` against a `Variant`-kinded var. More
   general (could later constrain `Numeric`, `Hashable`, etc.) but more
   surface area now.

Recommend (1): smaller blast radius, matches how `Type::Union` already
works, no new generics machinery. The codegen path doesn't change at
all — `Variant`/`Union(_)` are already represented identically at
runtime (`Value::Variant`).

The `T` output (field type) stays a free type variable — that's
correct, since fields are heterogeneous and only `match` can refine
them safely. This intentionally leaves *that* dynamism in place;
we're tightening only the input shape, not the output.

### Part B — locatable panics (secondary, only if zero-cost)

Three options, ranked by overhead:

1. **LLVM `!dbg` metadata + DWARF.** Emit source locations on every
   codegen instruction. When Rust runtime panics, its backtrace already
   resolves return addresses; with debug info attached to Seq-generated
   IR, the Seq-frame in the backtrace resolves to `.seq:line:col`.
   **Zero runtime cost.** Cost is at panic time and at compile time
   (slightly larger object files; gated behind a `--debug` flag if
   needed). This is the right answer if it works — needs a spike to
   confirm `addr2line` lights up Seq frames cleanly.
2. **Thread-local current-span, set only at fallible-builtin calls.**
   Reuse the pattern from `done/ASSERT_FAILURE_DETAILS.md` (Phase 2):
   compiler emits `patch_seq_set_current_span(line)` immediately
   before each call to a builtin in the existing fallible list (the
   one already maintained for the error-flag lint). One `store i64`
   per fallible call, zero overhead on infallible ops (arithmetic,
   stack shuffling, etc.). Acceptable if option 1 doesn't pan out.
3. **Unconditional span-threading through every call.** Rejected —
   measurable overhead on the hot path.

Part B is gated on Part A landing first. If Part A closes the
`variant.field-at` class entirely, Part B's urgency drops — but it
still pays off for legitimate runtime failures (FFI, OOM, future
fallible ops).

## Domain events

- **User compiles a Seq program with a variant.* misuse** → typechecker
  fails unification of input against `Type::Variant` → emits a
  `WordCall`-located error pointing at `variant.field-at` with
  expected/actual types → no executable produced → CI gates fire.
- **User compiles a program that calls a fallible builtin** → codegen
  emits a span-update or attaches `!dbg` metadata → no behavioural
  change for the user.
- **A runtime panic does fire** (FFI, OOM, etc.) → backtrace contains
  `.seq:line` → user can locate the call site without binary-searching
  the source.

## Checkpoints

1. The reproducing program (`"alpha beta gamma" 0 variant.field-at`)
   fails to compile with a type error pointing at the
   `variant.field-at` call's line and column. Existing `union`/`match`
   programs still type-check.
2. The full integration test suite passes (`just ci`). All `variant.*`
   call sites in stdlib (`json.seq`, generated `Make-X` /
   `X-fieldname` words, etc.) continue to type-check.
3. New negative tests in `crates/compiler/src/typechecker/tests.rs`
   covering: `String → variant.field-at`, `Int → variant.tag`, plus
   one per remaining `variant.*` op.
4. (Part B) Spike: a hand-crafted always-panicking `.seq` program
   produces a backtrace whose top Seq frame resolves to the correct
   `.seq` file and line under option 1 (DWARF) or option 2 (TLS span),
   *without* a measurable regression in the existing benchmarks
   (`just bench` if it exists, otherwise a quick wall-clock on
   `examples/projects/sss.seq`).
5. No new permanent unsafe code; no new `#[allow]` attributes.
