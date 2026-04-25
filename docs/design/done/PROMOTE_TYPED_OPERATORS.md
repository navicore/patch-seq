# Promote Typed Operators as Idiomatic Seq

Status: design · 2026-04-24

## Intent

Arithmetic sugar (`+`, `-`, `*`, `/`, `%`, `=`, `<`, `>`, `<=`, `>=`,
`<>`) works at the top level but cannot resolve inside a quotation body
— the typechecker has no concrete operand types to dispatch on without
adding constrained polymorphism (rejected by project memory). The
current behavior is consistent and the LSP already flags failures.

Rather than build a sugar-in-quotations escape hatch, **shift the
documented idiom toward the typed form** (`i.+`, `f.+`, `string.concat`,
…). Sugar stays available as a top-level convenience, but example code,
guides, and error messages should treat the typed form as the default
that always works.

This is a docs + diagnostics + tooling change, not a compiler change.

## Constraints

- **No language change.** Sugar resolution stays exactly as today.
  Don't touch `resolve_arithmetic_sugar`, codegen, or the
  `resolved_sugar` map.
- **No new lint.** A "prefer-typed-operators" rule would nag on every
  top-level `+` and conflict with sugar's purpose (readability where
  it's safe). The existing failure path is the right place to nudge.
- **No silent rewrite of existing examples.** The current corpus uses
  sugar in plenty of top-level contexts; that stays. Changes only
  apply to (a) examples that fail inside quotations today (already
  broken — they need rewriting anyway), and (b) new docs/examples.
- **Don't bury the sugar feature.** It's still useful and idiomatic
  at the top level. The shift is about quotation contexts and
  about showing learners the always-works form first.
- **Out of scope:** any change that makes `[ + ]` actually compile,
  any change to the sugar token set, any change to the parser.

## Approach

Three small, independent landings:

1. **Docs note in `docs/TYPE_SYSTEM_GUIDE.md` (or `language-guide.md`,
   wherever sugar is currently introduced).** One short paragraph
   under the existing "Arithmetic Sugar" coverage that says:
   *Sugar resolves from the typechecker's stack at the use site.
   Inside a quotation body the stack is empty (the quotation has its
   own effect signature), so sugar can't resolve there — write the
   typed form (`i.+`, `f.+`, `string.concat`) inside quotation bodies.*
   Add a tiny example showing `[ i.+ ]` as the idiomatic form for
   combinator targets.

2. **Improve the failure message in
   `crates/compiler/src/typechecker/words.rs::infer_word_call`.** The
   existing message already names alternatives:
   `"Use `i.+`, `f.+`, or `string.concat`."`. Strengthen it for the
   common `(empty, empty)` case (which is the one quotation-body
   failures hit) so the suggestion comes first and the wording
   doesn't sound like a generic type error. Something like:
   *"`+` cannot resolve inside a quotation body — write the typed
   form (`i.+` for Int, `f.+` for Float, `string.concat` for
   String)."*

3. **LSP code action.** When the LSP sees a sugar-resolution error at
   a position where the sugar token is known and the parser already
   has the line/column, offer a quick-fix that rewrites `+` →
   `i.+` / `f.+` / `string.concat` based on the surrounding context
   (or a three-option menu when the type can't be uniquely
   determined). Reuses the existing diagnostic→code-action plumbing
   in `crates/lsp/src/diagnostics.rs` (where unchecked-* lints
   already produce code actions).

Items 1 and 2 are roughly an hour each. Item 3 is half a day if it
fits the existing code-action shape; longer if the LSP needs new
type-info plumbing — in which case defer it as a separate follow-up.

## Domain events

- **A user writes sugar inside a quotation** → the existing error
  fires, but with the new wording it points directly at the typed
  form. LSP (when item 3 lands) offers a one-keystroke rewrite.
- **A user reads the language guide** → the typed form is shown as
  the always-works idiom alongside the convenience-sugar form, so
  habits build around the form that survives every context.
- **No runtime, codegen, or seqlings change.** The shift is
  observable only in docs, error wording, and LSP UX.

## Checkpoints

1. Docs change lands; spot-read shows the typed form is presented
   as the always-works default and sugar as a top-level convenience.
2. Compile a sample with sugar inside a quotation; the error message
   is short, names the typed alternatives, and reads as guidance
   rather than a generic type error.
3. (If item 3 ships) In the LSP, opening a `.seq` file with sugar
   inside a quotation surfaces a code action that rewrites the
   token. Accepting the action produces a file that compiles.
4. No change to `just ci` results before/after items 1 and 2.
   Existing sugar usages compile unchanged.
