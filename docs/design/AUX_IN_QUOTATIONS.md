# Aux Stack Inside Quotations (Issue #393)

## Intent

Allow `>aux` and `aux>` inside quotation bodies. Today they are rejected
with "Quotations are compiled as separate functions without aux stack
slots. Extract the quotation body into a named word if you need aux."
This forces fragmentation: a `list.fold` callback that wants temporary
storage must be lifted to a top-level word, splitting the algorithm
across the file.

The motivating case is Lagrange interpolation in the Shamir's Secret
Sharing example, where the fold body needs to thread context beyond
`(acc, elem)`. Aux is the natural answer; lifting to a named word loses
the locality of the algorithm.

## Constraints

- **Lexical scoping must hold**: every `>aux` must be paired with an
  `aux>` *within the same quotation scope*. You cannot `>aux` outside
  a quotation and `aux>` inside it, or vice versa.
- **No runtime changes**: aux is purely compile-time `alloca` slots.
- **No new syntax, no new operators**: same `>aux`/`aux>` words.
- **Type checker semantics unchanged for words**: existing aux behavior
  in named-word bodies stays exactly as it is.
- **Closures included**: the change applies to both stateless quotations
  and closures — both compile to independent LLVM functions.
- **Out of scope**: cross-quotation aux flow, aux as a first-class value,
  any interaction with `dip`/`keep`/`bi` beyond what falls out naturally.

## Validation (Already Done)

The infrastructure to support this already exists at the type level:

- `infer_quotation` (typechecker.rs:1178-1199) **already** saves the
  enclosing aux stack, sets it to Empty for the quotation body, runs
  inference, validates the quotation's aux is balanced at exit, and
  restores the enclosing state. The `in_quotation_scope` cell exists
  *only* to power the rejection guards.
- `codegen_quotation` (codegen/words.rs:165-171) **already** saves and
  clears `current_aux_slots` and `current_aux_sp` defensively. The
  comment even acknowledges it's defensive prep work.
- Quotations compile to independent LLVM functions with their own
  `entry:` block (codegen/words.rs:182-187, 251-255). Adding `alloca
  %Value` slots after `entry:` follows the same pattern as
  `codegen_word` (codegen/words.rs:81-89).
- Runtime is uninvolved — `aux` is not in any runtime symbol table.

The two `in_quotation_scope` guards (typechecker.rs:1354 and 1385) and
the missing per-quotation depth tracking are the only blockers.

## Approach

### Typechecker

1. Remove the `in_quotation_scope` guards in `infer_to_aux` (line 1354)
   and `infer_from_aux` (line 1385).
2. Add a new map `quotation_aux_depths: HashMap<usize, usize>` keyed by
   quotation ID (already program-wide unique, assigned in `parser.rs`).
3. In `infer_to_aux`, after pushing onto `current_aux_stack`, also
   record the new depth in the appropriate scope: if currently checking
   a quotation body, update `quotation_aux_depths[quot_id]`; otherwise
   continue updating `aux_max_depths[word_name]`.
4. The `in_quotation_scope` cell can stay (still useful for context) or
   be replaced with a stack of "current quotation IDs" for nested
   quotations. A stack is cleaner — depth-1 nested quotations get their
   own slot table.
5. Expose `take_quotation_aux_depths()` alongside the existing
   `take_aux_max_depths()`.

### Codegen

1. Plumb `quotation_aux_depths` through the same path as
   `aux_slot_counts` (lib.rs entry points → `set_quotation_aux_slot_counts`).
2. In `codegen_quotation`, instead of clearing `current_aux_slots`, look
   up the quotation's depth and emit `alloca %Value` slots into the
   quotation's `entry:` block — the same loop used at words.rs:85-89.
3. Save/restore aux slots across nested quotations (already done; just
   no longer "defensive — it's now load-bearing).

### What doesn't change

- The balance check at `infer_quotation` (line 1188) already enforces
  lexical scoping — no changes needed.
- The save/restore of `current_aux_stack` across quotation boundaries
  is already correct.
- Word codegen for aux is untouched.

## Domain Events

**Produced:**
- *Aux slots allocated in quotation function entry block* — new LLVM
  IR pattern to verify in tests
- *Per-quotation aux depth recorded* — new metadata flowing from
  typechecker to codegen

**Consumed:**
- *Quotation type-checked* — must record aux depth for the quotation ID
- *Quotation codegen begins* — must look up depth and emit allocas

**No longer produced:**
- *"aux not supported in quotations" error* — the diagnostic goes away
- The "extract to a named word" workaround friction

## Checkpoints

1. **Typechecker test**: `[ 5 >aux 10 aux> i.+ ] call` type-checks and
   produces `( -- Int )` with the inner aux balanced.
2. **Typechecker test**: `[ >aux ]` (unbalanced) fails with the existing
   "Quotation has unbalanced aux stack" error — that path still works.
3. **Codegen test**: the IR for the above quotation contains an
   `alloca %Value` in the quotation function's entry block.
4. **Nesting test**: `[ 5 >aux [ 10 >aux aux> ] call aux> i.+ ] call`
   — nested quotations each get their own slot table.
5. **Closure test**: a closure capturing a value AND using aux inside
   compiles and runs correctly.
6. **Original motivating case**: rewrite the Shamir example's
   `eval-poly` to use aux inside the fold body, verify it produces the
   same secret reconstruction output.
7. **Existing examples still pass**: full `just ci` clean.
8. **Cross-scope rejection**: `5 >aux [ aux> ] call` (>aux outside,
   aux> inside) is rejected at type-check time — verify the error.

## Implementation Order

1. Add `quotation_aux_depths` to typechecker, plumb through to codegen
   (no behavior change yet — guards still in place)
2. Emit allocas in `codegen_quotation` based on the new map (still no
   behavior change — depth is always 0)
3. Remove the two `in_quotation_scope` guards
4. Update aux depth tracking in `infer_to_aux` to write to the
   quotation map when inside a quotation scope
5. Add tests for all checkpoints
6. Rewrite the Shamir motivating case
