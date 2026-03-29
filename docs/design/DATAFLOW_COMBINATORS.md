# Dataflow Combinators

## Intent

Stack juggling with `swap rot pick >aux aux>` is the primary ergonomic pain
point in Seq. Rather than introducing local variables (which compromise
point-free style) or loops (which conflicted with the execution model),
we add **dataflow combinators** — higher-order words that express common
value-flow patterns. These are the idiomatic concatenative solution: no
new syntax, no new semantics, just words that compose.

The goal is to make 3-4 value stack management readable without `>aux`.

## Constraints

- No new syntax — combinators are builtin words that accept quotations
- Must not break existing type checking or codegen
- Quotation typing must be sound — row variables in quotation effects must
  unify correctly with the surrounding stack context
- Runtime implementation follows the established `list.each` / `call` pattern
- Aux stack remains available — combinators are an alternative, not a replacement

## Feasibility Assessment

**Typechecker**: Already has the machinery. `infer_call` pops a quotation,
extracts its `Effect`, freshens it, and applies it to the remaining stack.
Row variables in the quotation's effect unify with the caller's stack via
`unify_stacks`. This is exactly what `dip` needs: pop quotation, pop the
"preserved" value, apply quotation effect to the remaining stack, push the
preserved value back. `keep` and `bi` are variations on the same pattern.

The critical insight: `call` already does caller-stack-to-quotation-row
unification. `dip` is `call` with a value temporarily removed. `keep` is
`dip` that re-pushes. `bi` is two `keep`+`call` sequences with a cleanup.

**Codegen**: No changes needed to LLVM IR generation. Combinators are
runtime builtins (like `list.each`), not inline-expanded operations. They
pop a quotation, manipulate the stack, and invoke `patch_seq_call` or use
the same `call_with_value` pattern. All existing Quotation/Closure calling
conventions work unchanged.

**Runtime**: Follows the `list.each` pattern exactly — pop quotation, pop
values, call quotation with a temp stack or the real stack, push results.
The `call_with_value` helper already handles both Quotation and Closure.

## Proposed Combinators (Phased)

### Phase 1: Foundation

```
dip  : ( ..a x [..a -- ..b] -- ..b x )
```
Hide top value, run quotation on rest, restore value.
Subsumes most `>aux ... aux>` patterns.

```
keep : ( ..a x [..a x -- ..b] -- ..b x )
```
Run quotation on x (consuming it), but also preserve x.
Equivalent to `[ dup ] dip ... ` — common enough to deserve a name.

### Phase 2: Cleave (apply N quotations to same value(s))

```
bi   : ( ..a x [..a x -- ..b] [..b x -- ..c] -- ..c )
```
Apply two quotations to the same value. Eliminates `2dup >aux >aux`
patterns. This is the workhorse — covers "I need two views of the
same data."

```
bi*  : ( ..a x y [..a x -- ..b] [..b y -- ..c] -- ..c )
```
Apply different quotations to different values. Covers "process two
items differently."

```
bi@  : ( ..a x y [..a x -- ..b] -- ..c )  where ..b y [..a x -- ..b] is called again
```
Apply same quotation to two values. Covers "do the same thing to
both."

### Phase 3: Extended (only if Phase 1-2 prove their value)

```
tri  : ( ..a x [q1] [q2] [q3] -- ..d )  — three views of same value
2dip : ( ..a x y [..a -- ..b] -- ..b x y )  — hide two values
```

## Approach

### Typechecker Changes

Add special-case handling in `infer_word_call` for `dip` and `keep`,
similar to how `call` is handled today. The pattern:

1. Pop quotation type from stack
2. Pop the "preserved" value(s) and remember their type(s)
3. Extract quotation's `Effect`, freshen it
4. Apply the freshened effect to the remaining stack (this is where
   row-variable unification happens — same as `call`)
5. Push preserved value(s) back onto the result stack

For `bi`, step 4-5 happens twice (once per quotation), with the
preserved value re-introduced between applications.

This is ~50 lines per combinator in the typechecker, following the
existing `infer_call` pattern closely.

### Runtime Changes

Add to `runtime/src/quotations.rs` (or a new `combinators.rs`):

```rust
// dip: pop quot, pop x, call quot, push x
pub unsafe extern "C" fn patch_seq_dip(stack: Stack) -> Stack {
    let (stack, quot) = pop(stack);    // quotation
    let (stack, x) = pop(stack);       // preserved value
    let stack = call_callable(stack, &quot);
    push(stack, x)
}
```

`keep` and `bi` are similarly straightforward.

### Builtin Signatures

The type signatures need quotation types with row variables that
reference the surrounding context. This is the same pattern as
`list.map` but with the quotation's rows tied to the outer stack:

```rust
// dip: ( ..a x Quotation[..a -- ..b] -- ..b x )
// The key: quotation's input row "a" IS the caller's stack below x
```

This requires the signatures to be defined manually (not via the
`builtin!` macro) since the quotation's row variables must be shared
with the outer effect's row variables. Same approach used for
`list.map`, `list.fold`, etc.

## Concrete Examples

### Before/After: format-point

```seq
# Before (aux):
: format-point ( Int Int -- String )
  >aux int->string "Point(" swap string.concat
  ", " string.concat aux> int->string string.concat
  ")" string.concat ;

# After (dip):
: format-point ( Int Int -- String )
  [ int->string "Point(" swap string.concat ", " string.concat ] dip
  int->string string.concat ")" string.concat ;
```

### Before/After: sum-and-product

```seq
# Before (aux):
: sum-and-product ( Int Int -- String )
  2dup >aux >aux
  i.+ int->string " sum, " string.concat
  aux> aux> i.* int->string string.concat " product" string.concat ;

# After (bi):
: sum-and-product ( Int Int -- String )
  [ i.+ int->string " sum, " string.concat ]
  [ i.* int->string " product" string.concat ]
  bi string.concat ;
```

### Before/After: labeled-area

```seq
# Before (aux):
: labeled-area ( String Shape -- String )
  swap >aux match ... end
  int->string aux> ": area = " string.concat swap string.concat ;

# After (dip):
: labeled-area ( String Shape -- String )
  [ match ... end int->string ] dip
  ": area = " string.concat swap string.concat ;
```

## Checkpoints

1. **`dip` type-checks correctly** — `5 [ 1 i.+ ] dip` on stack `( 10 5 )`
   produces `( 11 5 )` with correct inferred types
2. **`keep` type-checks correctly** — `5 [ dup i.* ] keep` produces
   `( 25 5 )`
3. **`bi` type-checks correctly** — `5 [ 2 i.* ] [ 3 i.+ ] bi` produces
   `( 10 8 )`
4. **Existing test suite passes** — no regressions
5. **Closure interaction** — `dip` with closures (not just quotations)
   works correctly
6. **Error messages** — type errors in quotations passed to combinators
   point to the right source location
7. **Rewrite 3+ existing examples** using combinators, verify they compile
   and produce identical output
