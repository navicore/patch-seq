# Grammar Doc Audit — `docs/GRAMMAR.md`

Status: design · 2026-04-23

## Intent

The BNF in `docs/GRAMMAR.md` was written early in the project and hasn't
been systematically revisited since several features shipped that touch
surface syntax: symbols (`:foo`), auto-capturing quotations, arithmetic
sugar (`+ - * / % = < > <= >= <>`), loop combinators, `>aux`/`aux>`
inside quotations, and shebang tolerance. We want to audit the grammar
against current reality, file the gaps, and fix them — so the doc is
accurate for new users, LSP/tool authors, and anyone writing Seq without
reading the parser source.

This is a plan for doing the audit. The audit itself (and the patches it
produces) are separate work.

## Constraints

- **Grammar must match the parser, not the roadmap.** If a feature is
  designed but not parsed yet, it doesn't belong in `GRAMMAR.md`. Cross-
  check every proposed addition against `crates/compiler/src/parser.rs`.
- **No speculative syntax.** Don't invent notation for features we
  *might* ship. Keep the grammar descriptive of what seqc accepts today.
- **Don't rewrite for style.** If a section is already accurate, leave it
  alone — this is a gap-fill, not a rewrite.
- **Preserve existing examples.** The worked example (safe-divide +
  main + match) anchors the doc; don't reshape it just because newer
  idioms exist.
- **Out of scope:** rewording prose, adding a new "Semantic Notes"
  section the parser can't enforce, pretty-printing EBNF, adding a
  changelog. The audit produces a list of *missing productions* and
  *wrong productions* — nothing more.

## Approach

Three-step gap hunt, done once end-to-end:

1. **Catalog surface changes since the doc was written.** Walk
   `git log -- crates/compiler/src/parser.rs` and the memory ledger
   for language-level PRs. Build a short list of candidate syntax
   additions (symbols, operator sugar, auto-capture, shebang, aux
   shorthands, `variant.make-N`, constructor generation, any others
   surfaced by the walk).
2. **Cross-reference each candidate against the BNF.** For each item,
   answer two questions: (a) is the surface syntax already derivable
   from a current production (e.g. `>aux` matches `word_call` via
   `IDENT`), or (b) is a new/modified production needed. Only case
   (b) becomes a gap entry.
3. **Write gaps as a punch list**, not prose. Each gap gets one line:
   `<feature>` · current grammar says `<X>` · parser accepts `<Y>` ·
   proposed fix `<Z>`. The punch list is the deliverable. The
   follow-up PR applies the fixes.

Open candidates to probe specifically:

- `:foo` symbol literals — almost certainly missing from `literal`.
- Arithmetic sugar operators — the identifier grammar already admits
  `+`/`*`/`/`/`=`/`<`/`>` as identifier characters, so likely covered
  by `word_call`, but worth confirming `<=` / `>=` / `<>` aren't
  tokenized specially.
- Auto-capture semantics — may belong under "Semantic Notes" not the
  grammar itself.
- Shebang line (`#!/usr/bin/env seqc`) — the LSP strips it; is the
  parser lenient the same way, and does the grammar need a rule or
  a prose note?
- Quotation form of `>aux`/`aux>` — confirm these are plain word
  calls, not special syntax.
- Match binding syntax (`>name`) — verify it matches the parser's
  current form (field order, optional trailing comma, ignore
  wildcards).

## Domain Events

- **Audit produces a punch list** → a short markdown file (either a
  tmp/ scratch or a new design doc section) with per-gap entries. No
  commits, no grammar edits, no parser work.
- **Punch list is reviewed** → user decides which gaps to close and
  in what order. Fixes land as a normal PR touching `GRAMMAR.md`
  (and possibly parser if any "wrong production" is actually a
  parser bug, which would be logged separately, not folded into the
  doc patch).
- **Design doc lifecycle** → this file lives in `docs/design/` until
  the audit-and-fix lands, then moves to `docs/design/done/` per the
  standard design-doc flow.

## Checkpoints

Audit considered complete when:

1. Every production in `GRAMMAR.md` has a corresponding parser rule
   or an explicit "semantic, not syntactic" note. No orphans.
2. Every surface form the parser accepts has a production (or an
   explicit comment pointing at the prose that covers it). Verified
   by spot-checking ~10 example programs — stdlib + examples — and
   confirming each token sequence can be derived from the grammar.
3. The worked example at the bottom of `GRAMMAR.md` still parses
   cleanly against the updated grammar (no production references
   `variant.make-N` or `:foo` without those being defined).
4. `just ci` is untouched by this work — it's a doc-only PR.
