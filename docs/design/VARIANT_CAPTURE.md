# Variant Capture in Closures

## Intent

Enable closures to capture `Variant` values (lists, maps, unions) from
the caller's stack. This is the actual blocker for rewriting the 7
remaining manual-recursive loops in sss.seq.

## What We Learned (Feasibility Investigation)

**The typechecker auto-capture already works for Variants.** When a
fold body needs more inputs than the expected signature provides, the
row variable at the base of the seeded stack expands during inference.
`extract_concrete_types` counts ALL types (including type variables),
and the capture detection correctly fires. A test program that places
a list on the stack above a `list.fold` quotation compiles — the
closure is created with the list in its environment.

**The runtime crash is at codegen, not type-checking.** The generated
closure function calls `patch_seq_env_get_int` on a Variant value.
This is because the captured type resolved to `Int` during unification
— the body's accumulator operations (like `i.+`) constrained the type
variable to `Int`, even though the actual captured value is a Variant.

## The Real Problem: Type Resolution of Captures

The capture type is determined by `calculate_captures`, which slices
the bottom types from the body's inferred input stack. These types
are the *body's view* of the captured values — shaped by unification
with the body's operations, not by what's actually on the caller's
stack.

Example: `list.fold` expected input is `( ..b Acc T )`. The body
needs `( ..b CapturedList Acc T )`. After inference, `CapturedList`
is a type variable that the body constrains to match `list.length`'s
input — but the body doesn't call `list.length` on it directly (it
uses `>aux` first), so the variable may instead get constrained by
the accumulator operations (`i.+` → Int). The captured type resolves
to `Int`, but the actual stack value is a Variant.

The closure-pop verification (which unifies popped caller-stack types
against capture types) runs in `infer_quotation`, but by that point
the capture type is already `Int` (incorrectly resolved). The caller
stack has a Variant, and `unify_types(Variant, Int)` should fail...
unless the Variant is also represented as a type variable (`Var("V$23")`)
that unifies with anything.

## Two Fixes Needed

### Fix 1: Codegen — `env_push_variant` (safe, bounded)

Add a combined get+push function for Variant, following the String
pattern. This is needed regardless of how the type resolution is fixed,
because `emit_capture_push` needs a handler for Variant-typed captures.

Runtime: `patch_seq_env_push_variant(stack, env_data, env_len, index)`
Codegen: match `Type::Var` that represents a Variant and emit the call.

The challenge: identifying *which* `Type::Var` names represent Variants
vs Ints vs other types. The type system doesn't have a `Type::Variant`
enum arm — Variants flow through as `Type::Var("V")`, `Type::Var("V2")`,
etc. One approach: use `push_value` (the generic pusher that already
exists) as a fallback for any `Type::Var` capture, since at runtime
the env stores full `Value` objects regardless of type. This avoids
the type-identification problem entirely.

### Fix 2: Capture type should reflect caller stack, not body inference

The captured type should be whatever the caller's stack has at the
capture position, not whatever the body's unification resolved it to.
This is the soundness fix. The capture-pop loop in `infer_quotation`
already pops from the caller's stack — but it currently unifies against
the body-resolved capture type (which may be wrong). Instead, the
capture types stored in `Type::Closure { captures }` should come from
the *caller's stack* directly.

This may require changing `calculate_captures` to accept the caller
stack and use its types for the captures vector, while still using
the body/expected comparison to determine the *count*.

## Approach

### Conservative path

Use the generic `push_value` for ALL `Type::Var` captures. This
sidesteps both the type-identification problem and the type-resolution
problem:

- `emit_capture_push` handles `Type::Var(_)` by emitting a call to
  `patch_seq_env_get` (which returns a `Value`) followed by
  `patch_seq_push_value` (which pushes any `Value` onto the stack).
- This uses `%Value` in LLVM IR, which the runtime declaration already
  declares: `declare %Value @patch_seq_env_get(ptr, i64, i32)` and
  `declare ptr @patch_seq_push_value(ptr, %Value)`.
- Both functions already exist and are already declared.

**Risk:** `push_value` passes `Value` by value through FFI, which
crashed on Linux for strings (motivating `env_push_string`). The same
crash risk applies here. However, `Value::Variant` contains an
`Arc<VariantData>` (8 bytes + refcount), which is simpler than String.
Need to test on Linux.

### Safer path

Add `patch_seq_env_push_variant` and `patch_seq_env_push_map` following
the `env_push_string` pattern exactly. These avoid passing `Value` by
value through FFI. Then match `Type::Var(_)` in `emit_capture_push` and
emit the combined push call. For the type identification problem: any
`Type::Var` that isn't handled by the concrete type arms falls through
to the Variant/generic pusher. At runtime, the pusher validates the
actual `Value` variant matches.

## Constraints

- Must not break existing closure captures (Int, Bool, Float, String, Quotation)
- Must not break existing auto-capture for strand.spawn, list.fold, etc.
- Variant clone is O(1) refcount bump — no performance concern
- Map capture should work with the same mechanism (also `Arc`-wrapped)

## Checkpoints

1. A list auto-captured into a `list.fold` body compiles AND runs
2. `lagrange-outer-loop` in sss.seq rewritten as `integer-fold`
3. At least 3 other sss.seq loops rewritten
4. Existing closure tests pass unchanged
5. `just ci` clean
