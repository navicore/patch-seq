# Migrating from `if/else/then` to `if`/`when`/`unless` (Seq 6.0)

Status: spec · 2026-04-25

Seq 6.0 removes `if`, `else`, and `then` as parser keywords. Conditional
control flow is now expressed with the `if` combinator (a stack-consuming
word with signature `( ..a Bool [ ..a -- ..b ] [ ..a -- ..b ] -- ..b )`)
and its library variants `when` / `unless`. See
[`design/IF_AS_COMBINATOR.md`](design/IF_AS_COMBINATOR.md) for the
rationale.

This document is the **transformation spec** — the rules below are
sufficient for a careful human or an LLM collaborator to migrate any
existing Seq source. No tool is required.

## Rule 1 — two-armed `if`

```
cond if A else B then
```

becomes

```
cond [ A ] [ B ] if
```

`A` and `B` are the verbatim sequences of statements from the original
branches — preserve order, indentation, comments, and any nested
constructs unchanged. The Bool-producing expression `cond` stays where
it is.

## Rule 2 — one-armed `if`

```
cond if A then
```

becomes

```
cond [ A ] when
```

Use `when` (not `if` with an empty else) — it documents intent and
keeps the line short. The mirror form `cond not [ A ] when` becomes
`cond [ A ] unless` if `A` is the false-case body.

## Examples

### Trivial

```seq
# before
x 0 i.> if "positive" io.write-line then

# after
x 0 i.> [ "positive" io.write-line ] when
```

```seq
# before
x 0 i.= if "zero" else "nonzero" then io.write-line

# after
x 0 i.= [ "zero" ] [ "nonzero" ] if io.write-line
```

### Nested

The transformation is applied **inside-out** — rewrite the innermost
`if` first, then the next layer up. Each rewrite is local; the rules
don't interact.

```seq
# before
x 0 i.> if
  y 0 i.> if "++" else "+-" then
else
  y 0 i.> if "-+" else "--" then
then

# after
x 0 i.> [
  y 0 i.> [ "++" ] [ "+-" ] if
] [
  y 0 i.> [ "-+" ] [ "--" ] if
] if
```

### Multi-line branches

Multi-line bodies wrap inside the brackets. Indent the body to match
the surrounding code and keep the closing `]` on its own line if the
body is more than one line.

```seq
# before
ready? if
  emit-event
  count 1 i.+ store-count
else
  log-warning
  count store-count
then

# after
ready? [
  emit-event
  count 1 i.+ store-count
] [
  log-warning
  count store-count
] if
```

### Inside a quotation

Branches inside a quotation body migrate the same way:

```seq
# before
[ x 0 i.> if "pos" else "neg" then ]

# after
[ x 0 i.> [ "pos" ] [ "neg" ] if ]
```

### Inside a match arm

`match` is **not** changing. Migrate the `if`s inside arms with the
same rules; leave the `match` / pattern syntax alone.

```seq
# before
match
  Just(x) -> x 0 i.> if "pos x" else "non-pos x" then ;
  Nothing -> "no x" ;
;

# after
match
  Just(x) -> x 0 i.> [ "pos x" ] [ "non-pos x" ] if ;
  Nothing -> "no x" ;
;
```

### Divergent (recursive) branch

A branch ending in a self-recursive call (TCO loop) migrates verbatim
into the bracket. The compiler still emits `musttail` for a self-call
in the tail position of either branch.

```seq
# before
: countdown ( Int -- )
  dup 0 i.<= if
    drop
  else
    dup int->string io.write-line
    1 i.- countdown
  then
;

# after
: countdown ( Int -- )
  dup 0 i.<= [
    drop
  ] [
    dup int->string io.write-line
    1 i.- countdown
  ] if
;
```

### Yield-bearing branch

Branches that contain `chan.send`, `chan.receive`, or any other
yielding operation migrate unchanged. The literal-quotation form
(both branches written with `[ ... ]` at the call site) lowers to
the same conditional jump as the keyword form, so yields propagate
the same way.

```seq
# before
ok? if
  msg out-chan chan.send drop
else
  drop
then

# after
ok? [
  msg out-chan chan.send drop
] [
  drop
] if
```

## What does NOT change

- **`match`** and `union` definitions — untouched.
- **`cond`** — the existing variadic predicate-pair conditional is
  unchanged. Only `if`/`else`/`then` keywords go away.
- **Stack effect declarations** — branches still have to produce the
  same stack shape (the typechecker enforces it on the combinator just
  as it did on the keyword).
- **TCO** — self-tail-calls inside branches still get `musttail`
  lowering.

## Common mistakes

1. **Forgetting `when` for the one-armed case.** `cond [ A ] [ ] if`
   compiles, but `cond [ A ] when` is the idiomatic form. The migration
   doc prefers `when`.

2. **Reordering arms.** `if`'s argument order is `[ then-branch ]
   [ else-branch ]` — same order as the keyword form reads. Don't swap
   them.

3. **Missing brackets on a single-statement branch.** Each branch must
   be a quotation, even if it's one statement:
   - ✗ `cond drop 1 if` — three things on the stack, one is not a quot.
   - ✓ `cond [ drop 1 ] [ ] if` — explicit empty else, or use `when`.

4. **Whitespace inside brackets.** `[A]` is one identifier-like token.
   Always: `[ A ]` with spaces.

## Verifying a migration

After rewriting a file, the type-checker is the safety net — every
branch-shape mismatch the keyword form caught is still caught. If a
migrated file compiles cleanly with `seqc build`, the rewrite is sound.

For files that exercise tight inner loops, the literal-quotation `if`
form lowers to the same LLVM IR as the keyword form did (verified via
`--keep-ir`); there is no perf cost to the migration.
