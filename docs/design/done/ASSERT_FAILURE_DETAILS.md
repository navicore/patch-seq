# Enriching `test.assert*` Failure Output

Status: design · 2026-04-24 · issue [#422]

## Intent

When a Seq test fails today, the user sees only the failing test's name.
But a typical test word fires several `test.assert-eq` / `test.assert`
calls, any one of which could be the culprit. Without *which* assertion
failed, *what value* was on the stack, or *which line of source* to look
at, learners (especially in Seqlings) are stuck retrying blindly.

Ship a shape like:

```
test-fizzbuzz ... FAILED
  at line 17: expected 8, got 13
```

and, where line numbers aren't yet available, at least:

```
test-fizzbuzz ... FAILED
  assertion failed: expected 8, got 13
```

## Constraints

- **Plain-text output only.** Seqlings parses stdout; no JSON, no
  structured stream. A human should still be able to read it.
- **Don't change user-visible Seq signatures.** `test.assert-eq` stays
  `( ..a Int Int -- ..a )`. Any plumbing for line numbers has to be
  compiler-managed; users don't push line numbers onto the stack.
- **Don't weaken existing behaviour.** Passing tests still print
  `test-X ... ok`. The summary footer stays. Non-zero exit on any
  failure stays. Existing Seqlings integrations must keep parsing.
- **Runtime already captures what we need.** `patch_seq_test_assert_eq`
  already stores `expected` and `actual` strings in `TestFailure`.
  `patch_seq_test_finish` already prints them. The fix surface is
  primarily in how that output flows through `test_runner.rs`.
- **Out of scope:** a whole new test-framework DSL, coloured output,
  diff-style string rendering, capturing intermediate stack state,
  assertion macros.

## Approach

Three phases. Phases 1 and 2 are shippable independently; phase 3 is a
polish item.

### Phase 1 — unify the output stream (runtime + test runner)

The runtime's `test.finish` currently prints the test-name line to
**stdout** and the assertion details to **stderr**. The `TestRunner` in
`crates/compiler/src/test_runner.rs` reads both but discards stderr
except when the whole process crashed. That's the silent loss.

Fix: move the `expected:` / `actual:` / `assertion failed:` lines from
stderr to stdout so they follow the `FAILED` line they belong to. Then
`parse_test_output` can attach the indented detail lines after each
`FAILED` marker to the corresponding `TestResult.error_output`, and the
summary printer appends that block under each failure.

No compiler or codegen changes. Runtime change is one-line-per-println.
Test runner gains a small state machine: current-failing-test → append
its detail lines until the next `test-X ...` header.

Outcome: every failure shows actual vs expected values. Line numbers
remain absent.

### Phase 2 — plumb the source line through codegen

The compiler has the `WordCall`'s `Span` (line, column) at codegen time.
Add a tiny runtime hook — e.g. `patch_seq_test_set_line(line: i64)`
that writes to `TestContext.current_line`. In the codegen path for
`WordCall`, before emitting the call to any `test.assert*` builtin,
emit a call to `patch_seq_test_set_line` with the span's line as an
integer literal. The assertion's failure-record path reads
`current_line` and includes it in the message.

Line 0 / unset line means "no info"; the printer falls back to
Phase 1's format.

No change to the user's Seq source. No change to `test.assert*`'s
visible stack effect.

### Phase 3 — cap the per-test failure volume

If a single test word triggers many failures (e.g. loop-like tests that
compare a list element by element) the output can drown the real
signal. In `patch_seq_test_finish`, print the first 5 failures in full,
then `+N more failures` if there are more. Easy to tune later.

## Domain events

- **Assertion fails** → Runtime records the failure with
  `expected`/`actual`/(line, phase 2) → `test.finish` emits the detail
  lines on stdout → test runner associates them with the failing test →
  summary output surfaces them under the FAILED header.
- **Seqlings consumes the richer output** → no Seqlings code change
  required; it forwards `seqc test` stdout verbatim. Existing Seqlings
  regression checks that look for `... ok` / `... FAILED` keep working.
- **Test passes** → no change. `test-X ... ok`, no detail lines.

## Checkpoints

1. After phase 1: A crafted two-assertion test in
   `tests/integration/src/` where one asserts `5 5 test.assert-eq` and
   one asserts `1 2 test.assert-eq`. Expected output includes a
   `expected: 1` and `actual: 2` block under the failing test's
   `FAILED` line. Passing assertions remain invisible.
2. After phase 1: Seqlings' current success/failure parsing continues
   to succeed on its existing exercise set (smoke-test by running
   Seqlings against the new binary).
3. After phase 2: The detail line reads `at line N:` with `N` matching
   the source line of the failing `test.assert-eq` call. Spot-check
   with a multi-line test word.
4. `just ci` green through both phases; existing runtime and
   test-runner unit tests untouched except where wording changes.
5. Exit code of `seqc test` remains 1 on any failure, 0 on all-pass.
