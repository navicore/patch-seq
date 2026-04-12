# Integer Fold (discovered via sss.seq)

## Intent

Add `integer-fold` — a fold over an integer range that threads an
accumulator. This is the missing piece between `list.fold` (which
iterates elements but provides no index) and `each-integer` (which
provides indices but threads no accumulator).

The Shamir example has 7 manual-recursive loops that all follow the
exact same pattern: iterate `0..n-1`, thread an accumulator, and use
the index to access captured lists. Every one of them would become a
one-liner with `integer-fold` + auto-capture.

## What sss.seq actually uncovered

The caveats we attributed to "multi-value capture" were misdiagnosed.
Auto-capture already handles multiple values — it captures whatever is
on the stack above the quotation at creation time. The real gap is that
the only fold available (`list.fold`) iterates over *elements*, and the
loops that resist cleanup iterate over *integer positions*. They need
the index to do `list.get` into captured lists, and they need an
accumulator to build results.

## Proposed Word

```
integer-fold : ( Int acc [ acc idx -- acc ] -- result )
```

Calls the quotation with `(acc, 0)`, `(acc', 1)`, ..., `(acc'', n-1)`.
Returns the final accumulator. With auto-capture, any values on the
stack above the quotation at creation time are captured and available
inside the fold body.

Pure stdlib — same pattern as `times` and `each-integer`. Backed by
recursion + TCO. The quotation type `[ Int Int -- Int ]` (or more
generally `[ Acc Int -- Acc ]`) is concrete, so no row-polymorphism
issues.

## Constraints

- Pure stdlib: no compiler, runtime, or typechecker changes
- Lives in `std:loops` alongside `times` and `each-integer`
- TCO must hold (self-tail-recursive helper)
- Zero is a no-op (like `each-integer`)
- Quotation effect must be concrete (no row variables)

## What this solves in sss.seq

| Loop | Pattern | With `integer-fold` |
|------|---------|---------------------|
| `lagrange-outer-loop` | sum y_i * L_i(0) over 0..n-1 | `n 0 xs ys [ ... ] integer-fold` |
| `select-loop` | build list from indices[i] → points | `n list-of points indices [ ... ] integer-fold` |
| `str-to-bytes-loop` | iterate string chars by index | `len list-of str [ ... ] integer-fold` |
| `split-bytes-loop` | split each byte independently | `len list-of bytes k n [ ... ] integer-fold` |
| `reconstruct-loop` | reconstruct each byte | `len list-of all indices [ ... ] integer-fold` |
| `collect-ys-loop` | collect y-values across bytes | `len list-of all share_idx [ ... ] integer-fold` |
| `verify-eq-loop` | compare lists element-wise | `len true actual expected [ ... ] integer-fold` |

Seven loops, one word. The captured values (lists, parameters) are
auto-captured from the stack above the quotation.

## What this does NOT solve

- `gf256-mul-loop`, `gf256-inv-loop`: conditional early exit, tight
  state coupling. These are inherently recursive.
- `split-byte-loop`: iterates 1..n not 0..n-1 (offset). Could add
  `integer-fold-from` later, but the off-by-one variant isn't worth
  the complexity now.
- `print-ints-loop`, `print-all-shares-loop`: side-effect iteration
  without accumulation. These want `each-integer` with auto-capture,
  which already exists but the comma-separation pattern is awkward.
  Not a fold problem.

## Checkpoints

1. `5 0 [ i.+ ] integer-fold` → 10 (0+1+2+3+4)
2. `0 99 [ drop ] integer-fold` → 99 (no-op, returns initial acc)
3. Shamir `lagrange-outer-loop` rewritten as `integer-fold`
4. At least 3 other sss.seq loops rewritten
5. `just ci` clean
6. 1,000,000 iterations doesn't stack-overflow (TCO)
