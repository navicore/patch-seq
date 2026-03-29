# Language Gap Review

## Intent

Identify where Seq falls short for a user expecting a usable general-purpose
language. Seq is intentionally eccentric — concatenative, stack-based, statically
typed — but eccentricity in *paradigm* shouldn't mean eccentricity in *capability*.
A user who accepts the stack model should still be able to write real programs
without hitting walls.

This is a prioritized gap analysis, not a feature wishlist. Each gap is rated by
how likely a motivated user is to hit it in the first week of serious use.

## Constraints

- Don't clone Factor, Forth, or Erlang — keep Seq's identity
- Don't break the concatenative composition model
- Don't add syntax that fights point-free style
- Don't weaken the type system
- Performance regressions in existing code are unacceptable

## Gap Analysis

### Tier 1: Blocking — users will hit these in the first hour

**No loops.** Recursion-only iteration is the single biggest barrier to
adoption. TCO makes it *correct* but not *ergonomic*. Even experienced
concatenative programmers expect loop combinators. The LOOP_LOWERING design
addresses the performance angle; the ergonomic angle needs loop *words*
(`times`, `while`, `until`, `each-integer`) that desugar to recursion or
get native codegen. Factor has `each`, `times`, `while`, `until` — these
are table-stakes.

**Stack juggling pain past 3 values.** `swap rot pick roll` hit a wall
fast. Named locals (like Factor's `[let` or `:> name`) are the standard
escape hatch. Even a minimal `let( a b -- )` that binds the top N values
to names within a block would eliminate the worst stack gymnastics. The aux
stack helps but is word-scoped and unnamed — it's a workaround, not a
solution.

### Tier 2: Frustrating — users will hit these in the first week

**No Result/Option in the type system.** `(value Bool)` is pervasive but
the compiler can't enforce that you check the Bool. A `Result T E` or even
`Result T` union with `match` would let the type system catch unhandled
errors. The `union` + `match` machinery already exists — this is a stdlib
+ convention issue more than a language issue.

**No standard iteration protocol.** `list.map`, `list.filter`, `list.fold`
exist but maps have no `map.each`, `map.fold`, or `map.map`. Strings have
no character iterator. There's no generic "iterable" concept. Users
building anything data-oriented will need to iterate maps immediately.

**String building is painful.** Constructing output requires chains of
`string.concat`. A `string.join` (list of strings + separator → string)
and `string.format` or interpolation would cover 90% of cases. Even just
`join` on lists would be a huge quality-of-life win.

**No integer negation word.** `0 swap i.-` is a constant papercut. `i.neg`
or `negate` is trivial to add.

### Tier 3: Limiting — users hit these when building real programs

**No modules / namespaces.** `include` dumps everything into global scope.
Two libraries defining `parse` will collide. No visibility control (public
vs private words). This becomes acute as programs grow past a few files.

**Typed arithmetic is verbose.** `i.+` vs `f.+` is honest about the type
system but tedious. Factor uses generic dispatch; that's complex. A lighter
approach: the compiler already knows the types at every point — could it
resolve `+` to `i.+` or `f.+` based on inferred stack types? This would
be syntactic sugar over existing infrastructure.

**Collection gaps.** Lists are O(n²) for building (each `lv` copies).
No sets. No sorted collections. No efficient append/prepend (no cons-list
or vector with amortized O(1)). Users doing any batch processing will feel
this. At minimum, `list.reverse`, `list.sort`, `list.concat` (two lists),
and `list.zip` are expected.

**No process spawning / shell execution.** Can't call external commands.
`os.exec` or `os.shell` that returns (stdout, stderr, exit-code) is
essential for scripting use cases.

**No date/time formatting.** `time.now` returns epoch millis but there's
no way to format or parse timestamps.

### Tier 4: Polish — users notice these when comparing to other languages

**No documentation generation.** Comments are freeform. No structured
doc-comments that tools can extract.

**No `cond`-style multi-way dispatch beyond `match`.** `match` works on
unions, but a general `cond` for cascading boolean conditions (like Lisp's
`cond`) would reduce nested `if/else/then` chains. *(Note: `cond` exists
as a builtin but its usage pattern is unclear from the type signature.)*

**No first-class error messages.** `(value Bool)` gives no information
about *what* went wrong. A `(value String Bool)` convention or a proper
Error variant would help debugging.

**No REPL-driven development story.** The TUI REPL exists but there's no
`:reload` or hot-reload for iterating on definitions.

## Recommended Priority

1. **Loop combinators** — `times`, `each-integer`, `while` (stdlib words
   backed by recursion+TCO initially, native loops later via LOOP_LOWERING)
2. **Map iteration** — `map.each`, `map.fold`, `map.entries`
3. **String utilities** — `string.join`, `i.neg`, `list.reverse`
4. **Result convention** — stdlib `Result` union + `result.map`,
   `result.bind` combinators
5. **Named locals** — even a minimal form drastically improves readability
6. **Arithmetic sugar** — resolve `+` `-` `*` `/` from inferred types

## Checkpoints

- Can write FizzBuzz without stack pain → loops + string building
- Can write a CLI tool that reads JSON, transforms, writes JSON → iteration + error handling
- Can write a multi-file project without name collisions → modules
- Can write an HTTP service with structured error responses → Result type + string formatting
