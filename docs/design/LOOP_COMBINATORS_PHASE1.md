# Loop Combinators — Phase 1 (Issue #394)

## Intent

Add `times` and `each-integer` as pure stdlib words, backed by recursion
+ TCO. These cover the most common loop pattern — counted iteration —
which currently requires defining two words (an entry point and a
recursive helper) plus managing an explicit counter. The Shamir example
has four such loops that could each become a single line.

This is the ergonomic angle of `LANGUAGE_GAPS.md` Tier 1, separated
from the harder problem of row-polymorphic combinators like `while`.

## What this is NOT

`while` is **explicitly excluded** from this work. It was the combinator
that caused trouble in past attempts, and prototyping confirms it cannot
be pure stdlib: its quotation arguments need row-polymorphic effects
(`( ..a -- ..a Bool )` for the predicate, `( ..a -- ..a )` for the body)
that today's quotation type syntax can't express. Making `while` work
would require either typechecker special-casing (the `dip`/`keep`/`bi`
pattern) or restricting it to a fixed concrete state shape — both are
larger decisions that should be made separately, if at all.

For while-type loops, the established advice stands: use direct
recursion with TCO. The compiler guarantees tail-call optimization, so
recursive loops are correct and performant — just not as concise.

## Constraints

- **Pure stdlib**: no compiler changes, no runtime changes, no new
  builtin signatures, no new typechecker special cases
- **No new syntax**: ordinary `: word ( effect ) body ;` definitions
- **No new infrastructure for row-polymorphic combinators** — that's
  the decision being deferred
- **TCO must hold**: the recursive helpers must be self-tail-recursive
  so the compiler's existing TCO eliminates stack growth
- **Quotation effects must be concrete**: `times` takes `[ -- ]`,
  `each-integer` takes `[ Int -- ]`. No row variables in quotation types.
- **Out of scope**: `while`, `until`, `each` (over collections — already
  exists as `list.each`), unbounded loops, named index variables

## Prototyping Confirmation

Both words have been prototyped and verified to work as pure stdlib
*today*, with the current compiler:

- `times`: `3 [ "hi" io.write-line ] times` prints "hi" three times.
  Stack effect: `( Int [ -- ] -- )`. ~10 lines of `.seq`.
- `each-integer`: `5 [ int->string io.write-line ] each-integer`
  prints 0..4. Stack effect: `( Int [ Int -- ] -- )`. ~10 lines of
  `.seq`.

Type-checking works because both quotation types are concrete (no row
variables). TCO works because both helpers are self-tail-recursive.

## Approach

### New stdlib file

Create `crates/compiler/stdlib/loops.seq` containing two words:

**`times`** — `( Int [ -- ] -- )`
- Helper `times-loop` decrements the counter while the quotation is
  preserved on the stack via `dup ... call`.
- Base case: counter ≤ 0, drop both.
- Recursive case: `dup call`, decrement, recurse.

**`each-integer`** — `( Int [ Int -- ] -- )`
- Initializes a current index of 0, tracks the limit, and recurses
  through `( current limit quot )` until current ≥ limit.
- Calls the quotation with the current index each iteration.
- Uses `pick`-based stack access to keep the loop state stable.

Document the stack effect convention in a header comment so users
understand the invariants.

### Include from somewhere visible

The new file is loaded via `include std:loops` (relative to the embedded
stdlib). No automatic include — users opt in like `std:list`,
`std:json`, etc.

### Documentation

- Add a section to `LANGUAGE_GAPS.md` noting that loop combinators are
  partially shipped (Phase 1) with the deliberate exclusion of `while`
  and the rationale.
- Update `ROADMAP.md` if loop combinators are mentioned there.
- Add a short example file `examples/language/loop-combinators.seq`.

## Domain Events

**Produced:**
- *New stdlib module `std:loops` available* — users can `include std:loops`
- *Counted iteration patterns become one-liners* — Shamir example and
  similar code can be simplified

**Consumed:**
- *Quotation with concrete `[ -- ]` or `[ Int -- ]` type passed to a
  user-defined word* — already supported, no change

**Not produced:**
- No change to the typechecker or codegen
- No `while` combinator — recursion remains the answer for that pattern

## Checkpoints

1. **`times` works**: `5 [ "x" io.write ] times` prints `xxxxx`.
2. **`times` with zero**: `0 [ "x" io.write ] times` prints nothing.
3. **`times` with negative**: `-3 [ "x" io.write ] times` is a no-op
   (counter ≤ 0 base case fires immediately).
4. **`each-integer` works**: `5 [ int->string io.write-line ] each-integer`
   prints `0\n1\n2\n3\n4\n`.
5. **`each-integer` with zero**: `0 [ ... ] each-integer` is a no-op.
6. **TCO is preserved**: `1000000 [ ] times` does not stack-overflow.
7. **No type checker changes**: `cargo test --workspace` passes
   unchanged.
8. **`just ci` clean**: full pipeline including lint passes.
9. **Shamir rewrite**: at least one of the existing recursive loop
   helpers in `examples/projects/sss.seq` is replaced with a `times`
   or `each-integer` call, demonstrating real ergonomic improvement.
10. **`while` still rejected**: any user who tries to define `while`
    in stdlib hits the same `Occurs check failed` error today —
    intentional, documented in the design.

## Implementation Order

1. Write `crates/compiler/stdlib/loops.seq` with both words
2. Add to embedded stdlib list (wherever `std:list`, `std:json`, etc.
   are registered)
3. Add `examples/language/loop-combinators.seq`
4. Rewrite one Shamir loop helper as a smoke test
5. Update `LANGUAGE_GAPS.md` to mark Tier 1 partially addressed,
   document the `while` deferral
