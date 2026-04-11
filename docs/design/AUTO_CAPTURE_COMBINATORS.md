# Auto-Capture for Quotations Passed to Combinators (Issue #395)

## Intent

Extend the existing closure auto-capture mechanism to fire when a
quotation literal is passed directly to a combinator whose expected
quotation effect is known and provides *fewer* inputs than the body
needs. The excess inputs become closure captures, taken from the
caller's stack at quotation creation time.

This eliminates the manual recursion + `pick`/`roll` workaround for
patterns like Horner's polynomial evaluation via `list.fold`, where the
fold body needs context (the evaluation point `x`) beyond what fold
provides (`acc`, `coeff`).

## Why This Is Different From Past Risky Changes

This is **not** new mechanism. Closures, capture analysis, and runtime
dispatch on `Quotation` vs `Closure` all exist and work today. The
typechecker already auto-captures when the expected quotation type has
**empty** inputs (used by `strand.spawn`). This change extends that
exact same logic to fire when expected has *non-empty* inputs and the
body still needs more.

The mechanism the user was historically wary of was *loops as language
primitives*. This is *closures fired in more situations*, which is a
meaningfully smaller change.

## Constraints

- **No new runtime mechanism**: closures already work end-to-end for
  every affected combinator (`list.fold`, `list.map`, `list.filter`,
  `list.each`, `map.each`, `map.fold`). All dispatch through
  `invoke_callable`.
- **No new typechecker special case for the *combinator***: the rule
  fires whenever `expected_quotation_type` is set (which the existing
  lookahead already populates for any word that declares a quotation
  parameter). Don't hardcode a list of "blessed" combinators.
- **Capture order must respect the existing convention**: captures live
  at the bottom of the body's input stack, with the combinator-provided
  values pushed on top at invocation time.
- **Out of scope**: capturing `Variant` or nested `Closure` types
  (existing closure codegen limitation, see codegen/words.rs:340-379).
  Out of scope: row-polymorphic captures.

## Validation (Already Done)

Verified against the current code:

- `capture_analysis::calculate_captures` exists, handles arbitrary
  capture counts, and is unit-tested. Returns capture types
  bottom-to-top, matching `push_closure`'s pop order.
- `typechecker::analyze_captures` already invokes `calculate_captures`
  when `expected_quotation_type` is `Quotation` with empty inputs and
  body needs inputs (line 2024). The non-empty case currently falls
  through to a unification that fails with `Occurs check failed`.
- The lookahead at typechecker.rs:598-621 already populates
  `expected_quotation_type` from the next-word call, so the expected
  type is in the cell when the quotation body is checked — including
  for `list.fold`.
- `Type::Closure { fn_ptr, env }` runtime dispatch exists in
  `invoke_callable` (quotations.rs) and is used by every list/map
  combinator. Closures with captured environments already work.
- The motivating failure (`[ 3-arg body ] list.fold` where fold
  provides 2) reproduces today with `Occurs check failed: cannot unify
  ..rest$N with (..rest$N Int)`.

## Latent Issue To Fix Along The Way

`analyze_captures` returns `Type::Closure { captures, ... }` and the
caller (`infer_quotation` at typechecker.rs:1218-1226) pops `captures.len()`
values from the caller stack via `pop_type` — but **does not unify** the
popped types against the calculated captures. This is benign today
because `strand.spawn`'s capture types match whatever the user has on
the stack by construction, but it's a soundness gap that becomes acute
when auto-capture fires in more places. The fix is mechanical:
substitute `pop_type` with a unification check that the popped type
matches the calculated capture type at the corresponding position.

## Approach

### Typechecker (`analyze_captures`)

Extend the `Some(Type::Quotation(expected_effect))` arm:

1. **Today**: only auto-captures when `expected_is_empty`.
2. **Change**: also auto-capture when body inputs have *more concrete
   types* than expected inputs. Use `extract_concrete_types` (already in
   `capture_analysis`) on both, compute the difference, call
   `calculate_captures`.
3. The "topmost N inputs of the body" must structurally match the
   "topmost N inputs of expected." Verify this before capturing —
   if the topmost types don't align, the body is incompatible with
   the combinator regardless of captures, and the existing unification
   error message still applies.
4. The "bottom-most M inputs of the body" become the captures.

### Closure pop verification (`infer_quotation`)

In the closure branch (line 1218), replace the blind `pop_type` loop
with one that unifies the popped caller-stack type against the
corresponding entry in `captures`. This closes the latent soundness
gap and makes the new auto-capture path correct.

### Codegen

**Zero changes.** Closures already codegen correctly, capture push/pop
already works, runtime dispatch already handles both `Value::Quotation`
and `Value::Closure`.

### Tests

- Existing test: `strand.spawn` auto-capture still works (regression).
- New test: `[ 3-arg body ] list.fold` with one excess input compiles,
  runs, and produces the right result.
- New test: `[ 4-arg body ] list.fold` with two excess inputs.
- New test: type mismatch in captured slot is rejected with a clear
  error (the fixed pop verification).
- New test: capture types must be in the supported set
  (`Int`, `Bool`, `Float`, `String`, `Quotation`) — `Variant` and
  nested `Closure` are still rejected with the existing error.
- New test: the topmost inputs of the body must align with what the
  combinator provides — misalignment is rejected.
- Negative test: ambiguous case where both interpretations are valid is
  resolved by preferring the minimal capture (i.e., capture only what's
  needed to make the topmost inputs align).

## Domain Events

**Produced:**
- *Quotation literal promoted to Closure during type-checking* — same
  event as today's `strand.spawn` path, just fires more often
- *Captured values popped from caller stack at closure creation* —
  unchanged runtime mechanism
- *Pattern: combinator body needs context beyond what combinator
  provides → no longer requires manual recursion*

**Consumed:**
- *Quotation literal followed by combinator call* — already detected
  by the lookahead that populates `expected_quotation_type`
- *`expected_quotation_type` set with non-empty inputs* — was
  passed through to unification before, now triggers capture analysis
  first

**No longer produced:**
- *`Occurs check failed: cannot unify ..rest with ..rest Int` for
  combinator-quotation pairs that should auto-capture* — the error
  message that motivated the issue

## Checkpoints

1. **Reproduce baseline**: the Horner example from the issue
   (`[ x acc coeff -- result ] list.fold`) fails with `Occurs check
   failed` against current main. Confirmed.
2. **Auto-capture fires**: same code compiles after the change.
3. **Runtime correctness**: the polynomial evaluates to the same
   answer as the manual-recursion version in the Shamir example.
4. **`strand.spawn` regression**: the empty-input auto-capture path
   still works (existing tests pass unchanged).
5. **Latent fix verified**: write a test where caller stack has the
   wrong type at a captured slot — must be rejected with a clear error
   (it's accepted today, which is the soundness gap).
6. **Topmost-input alignment**: a body that "claims" to handle fold's
   `(acc, coeff)` but actually expects them in the wrong order is
   rejected.
7. **Variant capture rejection unchanged**: trying to capture a
   `Variant` from the stack still fails with the existing error —
   this isn't broadened by the change.
8. **Shamir rewrite**: `eval-poly` in `examples/projects/sss.seq` is
   rewritten using the new mechanism, replacing manual recursion. Must
   produce identical secret-reconstruction output.
9. **Full `just ci` clean**: no regressions in any existing example
   or test, no new clippy warnings.

## Implementation Order

1. Add tests for current behavior (baseline + the latent soundness gap)
2. Fix the latent pop-verification gap in `infer_quotation` —
   landing this first makes existing tests strictly stronger
3. Extend `analyze_captures` to handle the non-empty-expected,
   excess-body-inputs case
4. Add positive tests (the Horner example, multi-capture variants)
5. Add negative tests (alignment failures, unsupported capture types)
6. Rewrite Shamir's `eval-poly` as the integration test

## What This Does NOT Do

- Does not enable capturing `Variant` or `Closure` types — same
  limitation as today.
- Does not introduce new typechecker mechanism — extends one branch of
  one existing function.
- Does not change runtime, codegen, or any builtin signatures.
- Does not require a hardcoded list of "supported combinators." The
  rule generalizes to any word that declares a quotation parameter.
- Does not address the broader question of row-polymorphic captures
  (which would be needed for things like `while` from #394) — that
  remains future work and a separate decision.
