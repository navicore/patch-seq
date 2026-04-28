# `if` as a Combinator (Seq 6.0 Breaking Change)

Status: design · 2026-04-25 · issue [#430]

## Intent

`if/else/then` is the one corner of Seq that isn't concatenative. Everything
else — including the dataflow combinators (`dip`, `keep`, `bi`), loop
combinators (`times`, `each-integer`), and quotation auto-capture — is
expressed as words that consume the stack. Control flow stays a parser-level
special form held over from Forth.

The proposal: make `if` an ordinary word with the signature

```
: if ( ..a Bool [ ..a -- ..b ] [ ..a -- ..b ] -- ..b )
```

`when` / `unless` follow as library combinators. `if`/`else`/`then` are
removed from the parser entirely. The new shape is the natural payoff of
row polymorphism — it's only typeable *because* `..a`/`..b` exist — and
it makes `if` a value: nameable, returnable, dynamically constructible,
composable with the rest of the combinator vocabulary. The current syntax
forecloses all of that.

This is a 6.0 release because every existing program that uses `if` has to
move. We accept that. The user has been coding in Seq day-to-day and
control flow is the one part of the language that consistently feels
wrong; the rest of the design has earned the right to push this through.

**We explicitly value internal consistency over best-case line length.**
A one-armed conditional today reads `cond if X then`; tomorrow it reads
`cond [ X ] when`. The new form is a few characters longer in the
trivially simple case, and we accept that cost in exchange for the
language being honest about what it is.

## Constraints

- **No regression in compiled-code performance for literal-quotation `if`.**
  `cond [ A ] [ B ] if` must lower to the same conditional jump that
  `cond if A else B then` lowers to today. This requires literal-quotation
  inlining in codegen (Factor solves this; see `LOOP_LOWERING.md` for the
  closest existing precedent in seqc). If the optimization isn't ready,
  6.0 isn't ready.
- **`match` is out of scope.** Tagged-union dispatch is a different
  concept — arms with patterns are the right shape for it. Don't touch it.
- **`cond` with `{ ... }` table syntax is out of scope.** That's a
  separate variadic-dispatch design problem. Ship `if`/`when`/`unless`
  in 6.0; revisit `cond` once the new shape has settled in real code.
- **A migration doc must exist** before the cutover. The transformation
  is local and unambiguous, and the only known downstream consumer
  (seq-lisp, maintained by an LLM collaborator) can apply it from
  written rules. No tool is required — neither this repo nor seq-lisp
  needs one.
- **`just ci` stays green** through every step of the migration —
  including the stdlib (`crates/compiler/stdlib/*.seq`), every example
  in `examples/`, every integration test in `tests/integration/`.
- **LSP support is a release gate.** 6.0 does not ship until we have
  manually walked through the entire Seqlings exercise set with the new
  syntax, using the LSP for completion and hover, and confirmed the
  developer experience is at least as good as 5.x. Bracket matching for
  `[ ]` becomes load-bearing for control flow correctness — that has to
  feel solid before users see it.

## Approach

Five phases, each independently shippable to a feature branch but landing
together as 6.0:

1. **Combinator runtime.** Add `if` as a runtime function that pops two
   quotations and a bool, calls one. `when`/`unless` as stdlib `.seq`
   files using `if`. Empty quotation `[ ]` is the natural "do nothing".
2. **Codegen literal-quotation inlining.** When the codegen sees
   `[ ... ] [ ... ] if` with both quotations literal, lower to a
   conditional branch with both bodies inlined. Falls back to runtime
   dispatch when one or both branches are dynamic.
3. **Parser removal.** Drop `if`/`else`/`then` keywords from the lexer
   and parser. Parser errors at old syntax cite the migration doc.
4. **Migration doc.** Document the transformation rules:
   `cond if A else B then` → `cond [ A ] [ B ] if`,
   `cond if A then` → `cond [ A ] when`. Cover edge cases that arise
   while migrating the stdlib (nested ifs, multi-line branches, ifs
   inside quotations). The doc is the spec; an LLM or a careful human
   can apply it.
5. **Cutover.** Migrate `crates/compiler/stdlib/`, `examples/`, and
   `tests/integration/` by hand using the doc. Hand-walk Seqlings.
   Confirm LSP completion/hover/bracket-matching feels right.

## Domain events

- **User compiles 5.x code under 6.0** → parser rejects `if`/`then` →
  error message points at the migration doc → user (or LLM helper)
  applies the rewrite → code parses → typechecker validates that both
  quotations have the same effect → codegen inlines or dispatches →
  identical or near-identical binary.
- **Codegen sees a literal-quotation `if`** → emits a conditional branch
  with inlined bodies → no perf regression.
- **Codegen sees a dynamic `if`** (a quotation passed in via word
  argument or built at runtime) → emits a runtime dispatch through the
  quotation pointer → minor cost paid only by code that *needs* the
  dynamism, which couldn't exist at all under 5.x.
- **LSP encounters the new syntax** → `if` shows up as an OPERATOR
  completion item like every other combinator → hover shows its signature →
  bracket matching pairs `[`/`]` for branch boundaries.

## Checkpoints

1. The whole stdlib parses, typechecks, and runs identically after
   migration. Spot-check `json.seq`, `loops.seq`, `zipper.seq`.
2. A microbenchmark of the form `: f ( -- Int ) ... if ... ;` produces
   the same LLVM IR (modulo metadata) before and after, when both
   branches are literal quotations. Confirm with `--keep-ir`.
3. Every example under `examples/` runs and produces the same output.
4. `just ci` is green end-to-end.
5. **Seqlings walkthrough.** A human (the author) completes every
   Seqlings exercise with the new syntax and the LSP, confirms the
   experience is at least as good as 5.x. This is the release gate.
6. Migration doc covers every form actually encountered while migrating
   the repo. Anything that surprised the human migrator gets added to
   the doc before cutover, so seq-lisp's maintainer can apply the
   transformation cleanly without re-deriving the rules.

## Out of scope, explicitly

- `cond` with table syntax. Defer to post-6.0.
- `match` syntax changes. Untouched.
- Any speculative new combinators beyond `when`/`unless`. The community
  (or the author's day-to-day usage) will surface what else is wanted;
  resist the temptation to design it now.
