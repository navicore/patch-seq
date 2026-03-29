# Lint Strengthening

## Intent

The type system can't enforce Result-style error handling without generics,
and generics aren't coming. But the lint + LSP infrastructure is already
surprisingly capable: pattern-based linting, abstract stack simulation
(resource_lint), per-statement type info, and full type-checker integration
in the LSP. We can get most of the safety benefits of a Result type by
making the linter smarter about the `(value Bool)` pattern — catching
unchecked error flags at lint time rather than compile time.

The goal: a user who writes `file.slurp drop` or who forgets to check a
division-by-zero Bool gets a warning in their editor, not a silent bug.

## Constraints

- Lint-only — no type system changes, no new syntax, no runtime changes
- Must not produce false positives on idiomatic code (better to miss a
  bug than cry wolf)
- Must work in both `seqc lint` and the LSP (same analysis, two frontends)
- Suppressible via existing `# seq:allow(lint-id)` mechanism
- Generics are out of scope — we work with concrete types

## Current State

**Already works:**
- 17 TOML-based pattern rules (e.g., `file.slurp drop` → warning)
- Resource leak detection via abstract stack simulation
- LSP runs lints on every keystroke, shows inline diagnostics
- `statement_top_types` gives per-statement top-of-stack type (Int/Float/Bool)

**Gaps the pattern linter can't catch:**
- `file.slurp swap drop` (non-adjacent Bool drop)
- Bool consumed by unrelated operation (e.g., used as an Int)
- Fallible result stored on aux stack and never checked

## Proposed Lint Rules (Phased)

### Phase 1: Expand pattern coverage (TOML rules, no code changes)

Add rules for every fallible builtin that returns `(value Bool)`:

```toml
# Division
[[lint]]
id = "unchecked-division"
pattern = "i./ drop"
message = "division result Bool dropped — division by zero not checked"
severity = "warning"

# Also: i.%, string->int, string->float, tcp.*, regex.*, etc.
```

This is ~15 new TOML entries. Zero code, immediate value.

### Phase 2: Bool-tracking lint (new analysis pass)

Extend the abstract stack simulation (resource_lint pattern) to track
Bool values produced by fallible operations. The analysis:

1. Maintain a shadow stack alongside the real type stack
2. When a known fallible operation executes, tag the top Bool as
   "unchecked error flag from {operation}"
3. When a Bool is consumed by `if`, `cond`, or a word whose effect
   includes `( ..a Bool -- ..a )`, mark it as "checked"
4. When a tagged Bool is consumed by `drop`, `nip`, or any non-checking
   word, emit a warning

This catches the non-adjacent cases that patterns miss:
```seq
file.slurp swap drop  # swap moves Bool down, drop kills it — caught
file.slurp >aux ... aux> drop  # aux round-trip — caught
```

**Key insight:** the resource_lint already does exactly this kind of
abstract stack simulation for WeaveHandle/Channel tracking. The Bool
tracker is the same architecture with different tracked values.

**False-positive mitigation:**
- Only track Bools from known fallible builtins (not all Bools)
- If Bool flows into a user-defined word, assume it's checked
  (conservative — avoids cross-word false positives)
- `# seq:allow(unchecked-result)` suppresses per-word

### Phase 3: LSP-enhanced diagnostics

Use the typechecker's `statement_top_types` to enrich warnings:

- **Hover on fallible operations**: show "returns (value, Bool) —
  Bool indicates success" in the hover text
- **Inline hint**: after `file.slurp`, show ghost text `# -> String Bool`
  so the programmer sees the Bool they need to handle
- **Code action**: "Add error check" — wraps the operation in an
  `if ... else ... then` skeleton

These are editor-experience improvements, not new analysis.

## What This Does NOT Replace

- Compile-time exhaustiveness checking on Result types (needs generics)
- Automatic error propagation (needs effect system or Result monad)
- Runtime error messages (the Bool carries no "why" — that's a separate
  design question about `(value String Bool)` vs `(value Bool)`)

## Checkpoints

1. **Phase 1**: `seqc lint` warns on `i./ drop`, `string->int drop`,
   `tcp.listen drop`, etc. — 15+ new TOML rules, zero code changes
2. **Phase 2**: `seqc lint` warns on `file.slurp swap drop` (non-adjacent)
3. **Phase 2**: No false positives on existing examples/ directory
4. **Phase 3**: LSP shows `# -> String Bool` inlay hint after `file.slurp`
