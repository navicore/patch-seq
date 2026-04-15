# Multi-Capture Order Fix (Breaking Change)

## Intent

Multi-value closure captures currently arrive in the body's stack **reversed**
relative to the caller's stack. This wart has a tiny blast radius today
(one site in `sss.seq`, two integration tests), so now is the cheap moment
to fix it. Single-value captures are unaffected (one element has no order).

Target the next minor rev: `5.4 → 5.5`. Document in `MIGRATION_5.5.md`.

## Why This Is Actually Two Bugs

Tracing a 2-capture example `( ... v_deep v_shallow quot )`:

1. **Runtime** (`closures.rs:535`): `patch_seq_push_closure` pops top-down,
   pushes into `Vec<Value>` via `.push()`. Result: `env[0] = v_shallow`,
   `env[1] = v_deep`. (env stored top-down.)
2. **Codegen** (`words.rs:285`): closure-entry pushes `env[0]` first, `env[N-1]`
   last. Body top = `env[N-1]` = `v_deep` — reversed from caller where
   `v_shallow` was on top.
3. **Typechecker** (`typechecker.rs:1335`): `actual_captures.reverse()`
   forces the type vector to bottom-to-top, so `captures[0]` = deepest type.
   But env values are top-down. **Types and values disagree on index mapping.**
   This is latent today because every multi-capture test uses homogeneous
   types (all `Int`); a mixed-type multi-capture (e.g., `Int` + `Variant`)
   would crash at codegen-emitted `env_get_int` against a `Variant` value.

The `calculate_captures` doc block (`capture_analysis.rs:36-46`) claims the
reversal cancels out. It does not. The comment is the "documented weirdness"
the user flagged; ripping it out is part of the fix.

## Approach

Fix the runtime to match the typechecker's convention (bottom-to-top). One
line of runtime + a deleted comment:

- `patch_seq_push_closure`: after the pop loop, `captures.reverse()`.
  Result: `env[0]` = caller's deepest capture, `env[N-1]` = caller's
  shallowest — matching the typechecker's type order and the caller's
  visual order.
- `calculate_captures` doc block: rewrite to reflect the corrected model
  (env stored bottom-to-top; codegen pushes in index order; body top =
  caller top).
- Typechecker `analyze_captures`: no change. It already produces
  bottom-to-top types.
- Codegen `emit_capture_push`: no change.

## Constraints

- Single-value captures must remain byte-identical (reversing a 1-element
  vec is a no-op — trivially satisfied).
- `strand.spawn`, `list.fold`, `list.map`, `list.filter`, `list.each`,
  `map.fold`, `each-integer`, `times` all route through the same
  `push_closure`. All get the fix simultaneously.
- Out of scope: changing how captures are declared, serialised, or
  introspected. Out of scope: row-polymorphic captures (still future).
- `rust-toolchain.toml` pin unchanged; no dependency bumps.

## Domain Events

**Produced:**
- *Breaking change in closure capture ordering* — existing code with 2+
  captures in a quotation body will need a `swap`/`rot` removed (or added,
  depending on how they worked around the old order).

**Consumed:**
- *Caller-stack convention* — now preserved end-to-end into the body.

**No longer produced:**
- *Latent type/value index mismatch* for mixed-type multi-captures — the
  crash that hasn't happened yet because nobody's mixed types in a
  multi-capture.
- *"Captures arrive reversed" mental-model rule* — deleted from
  `feedback_capture_order` memory.

## Checkpoints

1. **Baseline test (new):** multi-capture integration test with
   heterogeneous types (`Int` + `Variant`). Against current main: expect
   crash or wrong result. After fix: passes.
2. **Ordering test (new):** 3-capture fold body where each capture is a
   distinct int; assert body sees them in caller order without any in-body
   rearrangement.
3. **Regression — single capture:** every existing test in
   `test-auto-capture.seq` and `test-std-loops.seq` passes unchanged.
4. **Regression — `strand.spawn`:** its tests pass unchanged (single
   capture in practice, but verify explicitly).
5. **Shamir rewrite:** `split-bytes` in `examples/projects/sss.seq`
   simplifies — the `swap` that reorders `( acc byte n k )` into the
   `split-byte` argument order goes away; body becomes just
   `split-byte lv`. Output matches.
6. **`just ci` clean.**
7. **Memory updated:** delete or rewrite
   `memory/feedback_capture_order.md` — rule no longer applies.
8. **Migration note:** `MIGRATION_5.5.md` with a one-paragraph description
   and a `sed`-style before/after for the common case.

## Implementation Order

1. Add the new tests (2-capture ordering, heterogeneous types) against
   current main. They should fail. This proves we're measuring the right
   thing.
2. One-line runtime change + comment rewrite. Tests from step 1 pass.
3. Simplify `sss.seq split-bytes`; rerun demo — must produce `SUCCESS —
   secret recovered!`.
4. Full `just ci`.
5. Delete stale memory; add migration doc.

## What This Does NOT Do

- Does not change how auto-capture is *triggered*; only the in-body order
  of captures is affected.
- Does not change single-capture semantics.
- Does not touch codegen or the typechecker (they're already consistent
  with bottom-to-top; runtime is the outlier).
- Does not affect TCO, Arc sharing, or the `Value` layout.
