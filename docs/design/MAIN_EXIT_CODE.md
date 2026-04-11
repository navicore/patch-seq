# Honor `main`'s Return Value as Process Exit Code (Issue #355)

## Intent

Make `: main ( -- Int )` actually return the Int as the process exit code,
matching 50 years of Unix convention. Today the value is silently
discarded — every example in the codebase that writes `0` at the end of
`main` is performing meaningless ceremony. This is a quiet correctness
bug that compromises composition with `&&`, `||`, `$?`, `set -e`, exit
status checks in test harnesses, CI gates, and basically every shell
idiom built since 1971.

The user's stated position: this matters enough to break things to fix.

## What The Issue Got Wrong

The issue claims "Script mode already supports `main ( -- Int )`."
**It does not.** Script mode just `exec`s the compiled binary
(`script.rs:219`). The compiled binary always returns 0
(`statements.rs:521`). Both modes have the identical bug. There is no
prior art to copy from script mode — both modes need the same
underlying fix, and once it lands the script mode wrapper inherits it
for free.

## Where The Bug Lives — Three Layers

All three need to change:

**Layer 1: `seq_main` epilogue (`codegen/words.rs:124-131`).** When
`is_main` is true, the generated `seq_main` calls `seq_stack_free` and
returns null. Whatever Int the user pushed is freed before anyone reads
it.

**Layer 2: Strand spawn lifetime
(`runtime/scheduler.rs:491-494`).** The scheduler spawns `seq_main` as a
strand. When the strand finishes, the scheduler calls
`free_stack(final_stack)` immediately — even if `seq_main` had left a
value, the scheduler would free it before the C `main` could read it.

**Layer 3: C `@main` epilogue
(`codegen/statements.rs:494-522`).** After `patch_seq_scheduler_run`,
the generated C `main` unconditionally emits `ret i32 0`. It has no
mechanism to learn what the main strand produced.

## Constraints

- **Programs declaring `main ( -- )` keep working unchanged.** Exit
  code 0, stack freed normally. Backwards compatible.
- **Programs declaring `main ( -- Int )` must exit with that Int as
  the process exit code.** Truncated to i32 (Unix exit codes are
  limited; Linux uses bits 0-7, others vary).
- **Type-check enforcement is unchanged.** The typechecker already
  rejects programs whose body doesn't match the declared effect — a
  `main ( -- )` that pushes a value gets caught today, and a
  `main ( -- Int )` that doesn't get caught today. Both behaviors
  remain correct.
- **The fix lives in the *binary*, not the script wrapper.** Script
  mode inherits the fix automatically because it execs the binary.
- **Out of scope**: structured error reporting from main, multi-value
  returns, panic-to-exit-code mapping, signal-to-exit-code conventions.

## Approach

A combined approach: a small piece of runtime state to carry the value
across the strand/coroutine boundary, plus codegen branches that
differentiate `void main` from `int main`.

### Runtime addition

Add `static EXIT_CODE: AtomicI64 = AtomicI64::new(0)` to the runtime,
plus an exported setter `patch_seq_set_exit_code(i64)` that the
generated `seq_main` calls before its stack is freed. Add an exported
getter `patch_seq_get_exit_code() -> i64` that the C `main` reads after
`scheduler_run` returns.

This is the minimum mechanism to bridge the strand/coroutine boundary.
It's a global, but only the main strand writes to it, and only after
all other strands have completed (since `scheduler_run` waits for all
strands). No race.

### Typechecker

Detect at type-check time whether the user's declared `main` has effect
`( -- )` or `( -- Int )`. Store this on the `CodeGen` state alongside
`current_word_name`, etc. — a simple bool `main_returns_int`.

Reject any other `main` signatures with a clear error: `main` must be
either `( -- )` (no exit code, exits 0) or `( -- Int )` (exit code is
the returned Int). No other shapes are allowed.

### Codegen

**Layer 1 fix** (`codegen_word`, the `is_main` branch):
- If `main_returns_int`: peek the top int from the stack, call
  `patch_seq_set_exit_code(i64)`, then call `seq_stack_free`, then
  `ret ptr null` as today.
- If void main: unchanged.

**Layer 2 fix** (`scheduler.rs`):
- No change needed. The strand can free its stack as today; the exit
  code has already been written to the global before the strand
  returned its final stack pointer.

**Layer 3 fix** (`codegen_main`, the C-level `@main`):
- After `scheduler_run` returns, emit a call to
  `patch_seq_get_exit_code()`, truncate the result to `i32`, and
  return it instead of the hardcoded `ret i32 0`.
- This change is unconditional — even void mains return 0 via the
  global, which was initialized to 0 and never written.

### Existing examples

Every example in the repo currently writes `: main ( -- Int ) ... 0 ;`.
After this change, those programs all start *actually* returning 0.
Behavior unchanged. No code breaks.

The interesting cases are programs that write something other than 0
without realizing it (or programs that intend to return non-zero on
failure but currently fail silently). The user has indicated they're
fine with this exposure — that's the *point*.

## Domain Events

**Produced:**
- *Process exits with user-specified exit code* — new event,
  observable to shell, CI, test harnesses, anything that reads `$?`
- *Compile-time rejection of `main` with disallowed effects* — only
  `( -- )` and `( -- Int )` are valid going forward
- *Test programs can signal failure via exit code* — enables `set -e`
  patterns in integration test scripts

**Consumed:**
- *Type-checked main word body* — the codegen uses the declared effect
  to decide which epilogue to emit
- *Scheduler completion* — the C `main` reads the exit code after all
  strands have finished

**No longer produced:**
- *Silent discard of `main`'s return value* — the value is now honored
- *False sense of security from `0` at the end of main* — the value
  now actually matters

## Checkpoints

1. **Trivial exit code**: `: main ( -- Int ) 42 ;` produces a binary
   that exits with 42. `echo $?` in shell shows 42.
2. **Zero is success**: `: main ( -- Int ) 0 ;` exits with 0.
3. **Void main works**: `: main ( -- ) "hi" io.write-line ;` exits with 0.
4. **Disallowed shapes rejected**: `: main ( -- Int Int ) ...` produces
   a clear type error pointing at the `main` declaration.
5. **Disallowed shapes rejected**: `: main ( -- String ) ...` produces a
   clear type error.
6. **Truncation behavior documented**: `: main ( -- Int ) 256 ;`
   exits with 0 (Linux truncates to low 8 bits) — verify the behavior
   and document it.
7. **Negative exit codes**: `: main ( -- Int ) -1 ;` — verify the
   behavior, document it (Unix convention: exit codes 0-255 only,
   negatives are implementation-defined).
8. **Concurrent strands**: a program that spawns strands, waits for
   them, then returns 7 from main — exits with 7. Verifies the
   exit code survives the scheduler join.
9. **Composition with shell**: `seqc build prog.seq -o prog && ./prog
   && echo "ok" || echo "fail"` actually branches on the exit code.
10. **Script mode propagation**: running `seqc script.seq` with the
    same `main ( -- Int ) 42 ;` body exits with 42 — script mode
    inherits the fix automatically.
11. **CI gates work**: integration test runner can detect a failed
    test by checking the binary's exit code instead of parsing stdout.
12. **All existing examples still pass**: every example in the repo
    that writes `0` at the end of main continues to work; any that
    accidentally write something else gets caught (the user's fine
    with this — it's the point).
13. **`just ci` clean**: full pipeline passes with no regressions.

## What This Does Not Do

- Does not introduce panic-to-exit-code mapping. A panicking program
  still exits via Rust's panic handler (typically 101 on Unix).
- Does not introduce signal-to-exit-code conventions. SIGINT etc.
  follow whatever the runtime/scheduler does today.
- Does not introduce multi-value returns from main, "exit with
  message," or any structured error mechanism. Just the integer.
- Does not change script mode's caching, exec behavior, or argument
  passing. Script mode is purely a wrapper that inherits the fix.

## Implementation Order

1. Add `EXIT_CODE` global + `patch_seq_set_exit_code` /
   `patch_seq_get_exit_code` to the runtime; export from `lib.rs`;
   declare in `codegen/runtime.rs`.
2. Add `main_returns_int` flag to `CodeGen` state, set it during the
   pre-pass that detects the `main` word.
3. Add typechecker rejection of disallowed `main` shapes (clear error).
4. Modify `codegen_word`'s `is_main` epilogue to call
   `patch_seq_set_exit_code` when `main_returns_int`.
5. Modify `codegen_main` (the C `@main`) to read the global and return
   it instead of hardcoded 0.
6. Add tests covering each checkpoint.
7. Audit existing examples for any that "return" non-zero — fix or
   verify intent.
8. Document the new behavior in user-facing docs (README, ARCHITECTURE).

## Why This Is Worth Doing Even If Things Break

Quiet correctness bugs are the worst kind. Today, anyone using Seq for
scripting sees `main ( -- Int ) ... 0 ;` in every example, types it
themselves, and reasonably assumes their exit codes work. They don't.
The first time this matters is the first time someone composes a Seq
program with `set -e` or a CI gate or a test harness — and the silent
behavior wastes hours of debugging time. Fixing this respects the
fifty years of shell tooling that depends on exit codes meaning
something.
