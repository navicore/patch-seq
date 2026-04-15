# Migrating to Seq 5.5

## Breaking Changes

### Multi-value closure captures now preserve caller order

Prior to 5.5, when a quotation auto-captured two or more values from the
caller's stack, those values arrived **reversed** inside the body: the
value that was deepest on the caller's stack appeared on top inside the
closure, and the value that was on top appeared deepest.

Starting in 5.5, captures are stored bottom-to-top, matching the caller's
visual stack order. Whatever was on top of the caller's stack right before
the quotation is on top inside the body.

**Single-value captures are unaffected** — reversing a one-element vector
is a no-op. Only code that captured two or more values needs to migrate.

**Why the change:** The old behavior forced every multi-capture body to
begin with a compensating `swap` / `rot` whose sole purpose was undoing
the inversion. Worse, the capture-type vector and the runtime env vector
disagreed on index order, which meant a heterogeneous capture (e.g. an
`Int` and a `Variant` together) would crash at runtime when codegen
emitted `env_get_int` against a Variant value. Both issues are fixed
together by making the runtime match the typechecker's convention.

**Before (v5.4):**
```seq
: split-bytes ( List Int Int -- List )
  list-of rot rot
  [
    # Body sees ( acc byte n k ) — captures reversed, so k on top.
    swap split-byte lv     # manual reorder to ( byte k n )
  ] list.fold
;
```

**After (v5.5):**
```seq
: split-bytes ( List Int Int -- List )
  list-of rot rot
  [
    # Body sees ( acc byte k n ) — matches caller order.
    split-byte lv
  ] list.fold
;
```

To port existing code: find quotations passed to combinators where two or
more values sit above the combinator's expected inputs on the caller's
stack. If the body started with a `swap` (or equivalent) that only
existed to compensate for reversed capture order, remove it. If the body
genuinely depended on the reversed order, either add a `swap` at the
caller before the quotation, or reorder the caller's pushes.
