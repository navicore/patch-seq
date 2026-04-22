//! Built-in word signatures for Seq
//!
//! Defines the stack effects for all runtime built-in operations.
//!
//! Uses declarative macros to minimize boilerplate. The `builtin!` macro
//! supports a Forth-like notation: `(a Type1 Type2 -- a Type3)` where:
//! - `a` is the row variable (representing "rest of stack")
//! - Concrete types: `Int`, `String`, `Float`
//! - Type variables: single uppercase letters like `T`, `U`, `V`

use crate::types::{Effect, SideEffect, StackType, Type};
use std::collections::HashMap;

/// Convert a type token to a Type expression
macro_rules! ty {
    (Int) => {
        Type::Int
    };
    (Bool) => {
        Type::Bool
    };
    (String) => {
        Type::String
    };
    (Float) => {
        Type::Float
    };
    (Symbol) => {
        Type::Symbol
    };
    (Channel) => {
        Type::Channel
    };
    // Single uppercase letter = type variable
    (T) => {
        Type::Var("T".to_string())
    };
    (U) => {
        Type::Var("U".to_string())
    };
    (V) => {
        Type::Var("V".to_string())
    };
    (W) => {
        Type::Var("W".to_string())
    };
    (K) => {
        Type::Var("K".to_string())
    };
    (M) => {
        Type::Var("M".to_string())
    };
    (Q) => {
        Type::Var("Q".to_string())
    };
    // Multi-char type variables (T1, T2, etc.)
    (T1) => {
        Type::Var("T1".to_string())
    };
    (T2) => {
        Type::Var("T2".to_string())
    };
    (T3) => {
        Type::Var("T3".to_string())
    };
    (T4) => {
        Type::Var("T4".to_string())
    };
    (V2) => {
        Type::Var("V2".to_string())
    };
    (M2) => {
        Type::Var("M2".to_string())
    };
    (Acc) => {
        Type::Var("Acc".to_string())
    };
}

/// Build a stack type from row variable 'a' plus pushed types
macro_rules! stack {
    // Just the row variable
    (a) => {
        StackType::RowVar("a".to_string())
    };
    // Row variable with one type pushed
    (a $t1:tt) => {
        StackType::RowVar("a".to_string()).push(ty!($t1))
    };
    // Row variable with two types pushed
    (a $t1:tt $t2:tt) => {
        StackType::RowVar("a".to_string())
            .push(ty!($t1))
            .push(ty!($t2))
    };
    // Row variable with three types pushed
    (a $t1:tt $t2:tt $t3:tt) => {
        StackType::RowVar("a".to_string())
            .push(ty!($t1))
            .push(ty!($t2))
            .push(ty!($t3))
    };
    // Row variable with four types pushed
    (a $t1:tt $t2:tt $t3:tt $t4:tt) => {
        StackType::RowVar("a".to_string())
            .push(ty!($t1))
            .push(ty!($t2))
            .push(ty!($t3))
            .push(ty!($t4))
    };
    // Row variable with five types pushed
    (a $t1:tt $t2:tt $t3:tt $t4:tt $t5:tt) => {
        StackType::RowVar("a".to_string())
            .push(ty!($t1))
            .push(ty!($t2))
            .push(ty!($t3))
            .push(ty!($t4))
            .push(ty!($t5))
    };
    // Row variable 'b' (used in some signatures)
    (b) => {
        StackType::RowVar("b".to_string())
    };
    (b $t1:tt) => {
        StackType::RowVar("b".to_string()).push(ty!($t1))
    };
    (b $t1:tt $t2:tt) => {
        StackType::RowVar("b".to_string())
            .push(ty!($t1))
            .push(ty!($t2))
    };
}

/// Define a builtin signature with Forth-like stack effect notation
///
/// Usage: `builtin!(sigs, "name", (a Type1 Type2 -- a Type3));`
macro_rules! builtin {
    // (a -- a)
    ($sigs:ident, $name:expr, (a -- a)) => {
        $sigs.insert($name.to_string(), Effect::new(stack!(a), stack!(a)));
    };
    // (a -- a T)
    ($sigs:ident, $name:expr, (a -- a $o1:tt)) => {
        $sigs.insert($name.to_string(), Effect::new(stack!(a), stack!(a $o1)));
    };
    // (a -- a T U)
    ($sigs:ident, $name:expr, (a -- a $o1:tt $o2:tt)) => {
        $sigs.insert($name.to_string(), Effect::new(stack!(a), stack!(a $o1 $o2)));
    };
    // (a T -- a)
    ($sigs:ident, $name:expr, (a $i1:tt -- a)) => {
        $sigs.insert($name.to_string(), Effect::new(stack!(a $i1), stack!(a)));
    };
    // (a T -- a U)
    ($sigs:ident, $name:expr, (a $i1:tt -- a $o1:tt)) => {
        $sigs.insert($name.to_string(), Effect::new(stack!(a $i1), stack!(a $o1)));
    };
    // (a T -- a U V)
    ($sigs:ident, $name:expr, (a $i1:tt -- a $o1:tt $o2:tt)) => {
        $sigs.insert($name.to_string(), Effect::new(stack!(a $i1), stack!(a $o1 $o2)));
    };
    // (a T U -- a)
    ($sigs:ident, $name:expr, (a $i1:tt $i2:tt -- a)) => {
        $sigs.insert($name.to_string(), Effect::new(stack!(a $i1 $i2), stack!(a)));
    };
    // (a T U -- a V)
    ($sigs:ident, $name:expr, (a $i1:tt $i2:tt -- a $o1:tt)) => {
        $sigs.insert($name.to_string(), Effect::new(stack!(a $i1 $i2), stack!(a $o1)));
    };
    // (a T U -- a V W)
    ($sigs:ident, $name:expr, (a $i1:tt $i2:tt -- a $o1:tt $o2:tt)) => {
        $sigs.insert($name.to_string(), Effect::new(stack!(a $i1 $i2), stack!(a $o1 $o2)));
    };
    // (a T U -- a V W X)
    ($sigs:ident, $name:expr, (a $i1:tt $i2:tt -- a $o1:tt $o2:tt $o3:tt)) => {
        $sigs.insert($name.to_string(), Effect::new(stack!(a $i1 $i2), stack!(a $o1 $o2 $o3)));
    };
    // (a T U -- a V W X Y)
    ($sigs:ident, $name:expr, (a $i1:tt $i2:tt -- a $o1:tt $o2:tt $o3:tt $o4:tt)) => {
        $sigs.insert($name.to_string(), Effect::new(stack!(a $i1 $i2), stack!(a $o1 $o2 $o3 $o4)));
    };
    // (a T U V -- a)
    ($sigs:ident, $name:expr, (a $i1:tt $i2:tt $i3:tt -- a)) => {
        $sigs.insert($name.to_string(), Effect::new(stack!(a $i1 $i2 $i3), stack!(a)));
    };
    // (a T U V -- a W)
    ($sigs:ident, $name:expr, (a $i1:tt $i2:tt $i3:tt -- a $o1:tt)) => {
        $sigs.insert($name.to_string(), Effect::new(stack!(a $i1 $i2 $i3), stack!(a $o1)));
    };
    // (a T U V -- a W X)
    ($sigs:ident, $name:expr, (a $i1:tt $i2:tt $i3:tt -- a $o1:tt $o2:tt)) => {
        $sigs.insert($name.to_string(), Effect::new(stack!(a $i1 $i2 $i3), stack!(a $o1 $o2)));
    };
    // (a T U V -- a W X Y)
    ($sigs:ident, $name:expr, (a $i1:tt $i2:tt $i3:tt -- a $o1:tt $o2:tt $o3:tt)) => {
        $sigs.insert($name.to_string(), Effect::new(stack!(a $i1 $i2 $i3), stack!(a $o1 $o2 $o3)));
    };
    // (a T U V W -- a X)
    ($sigs:ident, $name:expr, (a $i1:tt $i2:tt $i3:tt $i4:tt -- a $o1:tt)) => {
        $sigs.insert($name.to_string(), Effect::new(stack!(a $i1 $i2 $i3 $i4), stack!(a $o1)));
    };
    // (a T U V W X -- a Y)
    ($sigs:ident, $name:expr, (a $i1:tt $i2:tt $i3:tt $i4:tt $i5:tt -- a $o1:tt)) => {
        $sigs.insert($name.to_string(), Effect::new(stack!(a $i1 $i2 $i3 $i4 $i5), stack!(a $o1)));
    };
}

/// Define multiple builtins with the same signature
/// Note: Can't use a generic macro due to tt repetition issues, so we use specific helpers
macro_rules! builtins_int_int_to_int {
    ($sigs:ident, $($name:expr),+ $(,)?) => {
        $(
            builtin!($sigs, $name, (a Int Int -- a Int));
        )+
    };
}

macro_rules! builtins_int_int_to_bool {
    ($sigs:ident, $($name:expr),+ $(,)?) => {
        $(
            builtin!($sigs, $name, (a Int Int -- a Bool));
        )+
    };
}

macro_rules! builtins_bool_bool_to_bool {
    ($sigs:ident, $($name:expr),+ $(,)?) => {
        $(
            builtin!($sigs, $name, (a Bool Bool -- a Bool));
        )+
    };
}

macro_rules! builtins_int_to_int {
    ($sigs:ident, $($name:expr),+ $(,)?) => {
        $(
            builtin!($sigs, $name, (a Int -- a Int));
        )+
    };
}

macro_rules! builtins_string_to_string {
    ($sigs:ident, $($name:expr),+ $(,)?) => {
        $(
            builtin!($sigs, $name, (a String -- a String));
        )+
    };
}

macro_rules! builtins_float_float_to_float {
    ($sigs:ident, $($name:expr),+ $(,)?) => {
        $(
            builtin!($sigs, $name, (a Float Float -- a Float));
        )+
    };
}

macro_rules! builtins_float_float_to_bool {
    ($sigs:ident, $($name:expr),+ $(,)?) => {
        $(
            builtin!($sigs, $name, (a Float Float -- a Bool));
        )+
    };
}

/// Get the stack effect signature for a built-in word
pub fn builtin_signature(name: &str) -> Option<Effect> {
    let signatures = builtin_signatures();
    signatures.get(name).cloned()
}

/// Get all built-in word signatures
pub fn builtin_signatures() -> HashMap<String, Effect> {
    let mut sigs = HashMap::new();

    // =========================================================================
    // I/O Operations
    // =========================================================================

    builtin!(sigs, "io.write", (a String -- a)); // Write without newline
    builtin!(sigs, "io.write-line", (a String -- a));
    builtin!(sigs, "io.read-line", (a -- a String Bool)); // Returns line + success flag
    builtin!(sigs, "io.read-line+", (a -- a String Int)); // DEPRECATED: use io.read-line instead
    builtin!(sigs, "io.read-n", (a Int -- a String Int)); // Read N bytes, returns bytes + status

    // =========================================================================
    // Command-line Arguments
    // =========================================================================

    builtin!(sigs, "args.count", (a -- a Int));
    builtin!(sigs, "args.at", (a Int -- a String));

    // =========================================================================
    // File Operations
    // =========================================================================

    builtin!(sigs, "file.slurp", (a String -- a String Bool)); // returns (content success) - errors are values
    builtin!(sigs, "file.exists?", (a String -- a Bool));
    builtin!(sigs, "file.spit", (a String String -- a Bool)); // (content path -- success)
    builtin!(sigs, "file.append", (a String String -- a Bool)); // (content path -- success)
    builtin!(sigs, "file.delete", (a String -- a Bool));
    builtin!(sigs, "file.size", (a String -- a Int Bool)); // (path -- size success)

    // Directory operations
    builtin!(sigs, "dir.exists?", (a String -- a Bool));
    builtin!(sigs, "dir.make", (a String -- a Bool));
    builtin!(sigs, "dir.delete", (a String -- a Bool));
    builtin!(sigs, "dir.list", (a String -- a V Bool)); // V = List variant

    // file.for-each-line+: Complex quotation type - defined manually
    sigs.insert(
        "file.for-each-line+".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string())
                .push(Type::String)
                .push(Type::Quotation(Box::new(Effect::new(
                    StackType::RowVar("a".to_string()).push(Type::String),
                    StackType::RowVar("a".to_string()),
                )))),
            StackType::RowVar("a".to_string())
                .push(Type::String)
                .push(Type::Bool),
        ),
    );

    // =========================================================================
    // Type Conversions
    // =========================================================================

    builtin!(sigs, "int->string", (a Int -- a String));
    builtin!(sigs, "int->float", (a Int -- a Float));
    builtin!(sigs, "float->int", (a Float -- a Int));
    builtin!(sigs, "float->string", (a Float -- a String));
    builtin!(sigs, "string->int", (a String -- a Int Bool)); // value + success flag
    builtin!(sigs, "string->float", (a String -- a Float Bool)); // value + success flag
    builtin!(sigs, "char->string", (a Int -- a String));
    builtin!(sigs, "symbol->string", (a Symbol -- a String));
    builtin!(sigs, "string->symbol", (a String -- a Symbol));

    // =========================================================================
    // Integer Arithmetic ( a Int Int -- a Int )
    // =========================================================================

    builtins_int_int_to_int!(sigs, "i.add", "i.subtract", "i.multiply");
    builtins_int_int_to_int!(sigs, "i.+", "i.-", "i.*");

    // Division operations return ( a Int Int -- a Int Bool ) for error handling
    builtin!(sigs, "i.divide", (a Int Int -- a Int Bool));
    builtin!(sigs, "i.modulo", (a Int Int -- a Int Bool));
    builtin!(sigs, "i./", (a Int Int -- a Int Bool));
    builtin!(sigs, "i.%", (a Int Int -- a Int Bool));

    // =========================================================================
    // Integer Comparison ( a Int Int -- a Bool )
    // =========================================================================

    builtins_int_int_to_bool!(sigs, "i.=", "i.<", "i.>", "i.<=", "i.>=", "i.<>");
    builtins_int_int_to_bool!(sigs, "i.eq", "i.lt", "i.gt", "i.lte", "i.gte", "i.neq");

    // =========================================================================
    // Boolean Operations ( a Bool Bool -- a Bool )
    // =========================================================================

    builtins_bool_bool_to_bool!(sigs, "and", "or");
    builtin!(sigs, "not", (a Bool -- a Bool));

    // =========================================================================
    // Bitwise Operations
    // =========================================================================

    builtins_int_int_to_int!(sigs, "band", "bor", "bxor", "shl", "shr");
    builtins_int_to_int!(sigs, "bnot", "popcount", "clz", "ctz");
    builtins_int_to_int!(sigs, "i.neg", "negate"); // Integer negation (inline)
    builtin!(sigs, "int-bits", (a -- a Int));

    // =========================================================================
    // Stack Operations (Polymorphic)
    // =========================================================================

    builtin!(sigs, "dup", (a T -- a T T));
    builtin!(sigs, "drop", (a T -- a));
    builtin!(sigs, "swap", (a T U -- a U T));
    builtin!(sigs, "over", (a T U -- a T U T));
    builtin!(sigs, "rot", (a T U V -- a U V T));
    builtin!(sigs, "nip", (a T U -- a U));
    builtin!(sigs, "tuck", (a T U -- a U T U));
    builtin!(sigs, "2dup", (a T U -- a T U T U));
    builtin!(sigs, "3drop", (a T U V -- a));

    // pick and roll: Type approximations (see detailed comments below)
    // pick: ( ..a T Int -- ..a T T ) - copies value at depth n to top
    builtin!(sigs, "pick", (a T Int -- a T T));
    // roll: ( ..a T Int -- ..a T ) - rotates n+1 items, bringing depth n to top
    builtin!(sigs, "roll", (a T Int -- a T));

    // =========================================================================
    // Aux Stack Operations (word-local temporary storage)
    // Note: actual aux stack effects are handled specially by the typechecker.
    // These signatures describe only the main stack effects.
    // =========================================================================

    builtin!(sigs, ">aux", (a T -- a));
    builtin!(sigs, "aux>", (a -- a T));

    // =========================================================================
    // Channel Operations (CSP-style concurrency)
    // Errors are values, not crashes - all ops return success flags
    // =========================================================================

    builtin!(sigs, "chan.make", (a -- a Channel));
    builtin!(sigs, "chan.send", (a T Channel -- a Bool)); // returns success flag
    builtin!(sigs, "chan.receive", (a Channel -- a T Bool)); // returns value and success flag
    builtin!(sigs, "chan.close", (a Channel -- a));
    builtin!(sigs, "chan.yield", (a - -a));

    // =========================================================================
    // Quotation/Control Flow Operations
    // =========================================================================

    // call: Polymorphic - accepts Quotation or Closure
    // Uses type variable Q to represent "something callable"
    sigs.insert(
        "call".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string()).push(Type::Var("Q".to_string())),
            StackType::RowVar("b".to_string()),
        ),
    );

    // =========================================================================
    // Dataflow Combinators
    // =========================================================================

    // dip: ( ..a x Quotation[..a -- ..b] -- ..b x )
    // Hide top value, run quotation on rest, restore value.
    // Type-checked specially in typechecker (like `call`); this is a placeholder.
    // Same placeholder shape as keep — both take (value, quotation) and preserve value.
    sigs.insert(
        "dip".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string())
                .push(Type::Var("T".to_string()))
                .push(Type::Var("Q".to_string())),
            StackType::RowVar("b".to_string()).push(Type::Var("T".to_string())),
        ),
    );

    // keep: ( ..a x Quotation[..a x -- ..b] -- ..b x )
    // Run quotation on value, but preserve the original.
    // Type-checked specially in typechecker (like `call`); this is a placeholder.
    // Same placeholder shape as dip — both take (value, quotation) and preserve value.
    sigs.insert(
        "keep".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string())
                .push(Type::Var("T".to_string()))
                .push(Type::Var("Q".to_string())),
            StackType::RowVar("b".to_string()).push(Type::Var("T".to_string())),
        ),
    );

    // bi: ( ..a x Quotation[..a x -- ..b] Quotation[..b x -- ..c] -- ..c )
    // Apply two quotations to the same value.
    // Type-checked specially in typechecker (like `call`); this is a placeholder.
    // Q1/Q2 are distinct type vars — the two quotations may have different types.
    sigs.insert(
        "bi".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string())
                .push(Type::Var("T".to_string()))
                .push(Type::Var("Q1".to_string()))
                .push(Type::Var("Q2".to_string())),
            StackType::RowVar("b".to_string()),
        ),
    );

    // cond: Multi-way conditional (variable arity)
    sigs.insert(
        "cond".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string()),
            StackType::RowVar("b".to_string()),
        ),
    );

    // strand.spawn: ( a Quotation -- a Int ) - spawn a concurrent strand
    // The quotation can have any stack effect - it runs independently
    sigs.insert(
        "strand.spawn".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string()).push(Type::Quotation(Box::new(Effect::new(
                StackType::RowVar("spawn_in".to_string()),
                StackType::RowVar("spawn_out".to_string()),
            )))),
            StackType::RowVar("a".to_string()).push(Type::Int),
        ),
    );

    // strand.weave: ( a Quotation -- a handle ) - create a woven strand (generator)
    // The quotation receives (WeaveCtx, first_resume_value) and must thread WeaveCtx through.
    // Returns a handle (WeaveCtx) for use with strand.resume.
    sigs.insert(
        "strand.weave".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string()).push(Type::Quotation(Box::new(Effect::new(
                StackType::RowVar("weave_in".to_string()),
                StackType::RowVar("weave_out".to_string()),
            )))),
            StackType::RowVar("a".to_string()).push(Type::Var("handle".to_string())),
        ),
    );

    // strand.resume: ( a handle b -- a handle b Bool ) - resume weave with value
    // Takes handle and value to send, returns (handle, yielded_value, has_more)
    sigs.insert(
        "strand.resume".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string())
                .push(Type::Var("handle".to_string()))
                .push(Type::Var("b".to_string())),
            StackType::RowVar("a".to_string())
                .push(Type::Var("handle".to_string()))
                .push(Type::Var("b".to_string()))
                .push(Type::Bool),
        ),
    );

    // yield: ( a ctx b -- a ctx b | Yield b ) - yield value and receive resume value
    // The WeaveCtx must be passed explicitly and threaded through.
    // The Yield effect indicates this word produces yield semantics.
    sigs.insert(
        "yield".to_string(),
        Effect::with_effects(
            StackType::RowVar("a".to_string())
                .push(Type::Var("ctx".to_string()))
                .push(Type::Var("b".to_string())),
            StackType::RowVar("a".to_string())
                .push(Type::Var("ctx".to_string()))
                .push(Type::Var("b".to_string())),
            vec![SideEffect::Yield(Box::new(Type::Var("b".to_string())))],
        ),
    );

    // strand.weave-cancel: ( a handle -- a ) - cancel a weave and release its resources
    // Use this to clean up a weave that won't be resumed to completion.
    // This prevents resource leaks from abandoned weaves.
    sigs.insert(
        "strand.weave-cancel".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string()).push(Type::Var("handle".to_string())),
            StackType::RowVar("a".to_string()),
        ),
    );

    // =========================================================================
    // TCP Operations
    // =========================================================================

    // TCP operations return Bool for error handling
    builtin!(sigs, "tcp.listen", (a Int -- a Int Bool));
    builtin!(sigs, "tcp.accept", (a Int -- a Int Bool));
    builtin!(sigs, "tcp.read", (a Int -- a String Bool));
    builtin!(sigs, "tcp.write", (a String Int -- a Bool));
    builtin!(sigs, "tcp.close", (a Int -- a Bool));

    // =========================================================================
    // OS Operations
    // =========================================================================

    builtin!(sigs, "os.getenv", (a String -- a String Bool));
    builtin!(sigs, "os.home-dir", (a -- a String Bool));
    builtin!(sigs, "os.current-dir", (a -- a String Bool));
    builtin!(sigs, "os.path-exists", (a String -- a Bool));
    builtin!(sigs, "os.path-is-file", (a String -- a Bool));
    builtin!(sigs, "os.path-is-dir", (a String -- a Bool));
    builtin!(sigs, "os.path-join", (a String String -- a String));
    builtin!(sigs, "os.path-parent", (a String -- a String Bool));
    builtin!(sigs, "os.path-filename", (a String -- a String Bool));
    builtin!(sigs, "os.exit", (a Int -- a)); // Never returns, but typed as identity
    builtin!(sigs, "os.name", (a -- a String));
    builtin!(sigs, "os.arch", (a -- a String));

    // =========================================================================
    // Signal Handling (Unix signals)
    // =========================================================================

    builtin!(sigs, "signal.trap", (a Int -- a));
    builtin!(sigs, "signal.received?", (a Int -- a Bool));
    builtin!(sigs, "signal.pending?", (a Int -- a Bool));
    builtin!(sigs, "signal.default", (a Int -- a));
    builtin!(sigs, "signal.ignore", (a Int -- a));
    builtin!(sigs, "signal.clear", (a Int -- a));
    // Signal constants (platform-correct values)
    builtin!(sigs, "signal.SIGINT", (a -- a Int));
    builtin!(sigs, "signal.SIGTERM", (a -- a Int));
    builtin!(sigs, "signal.SIGHUP", (a -- a Int));
    builtin!(sigs, "signal.SIGPIPE", (a -- a Int));
    builtin!(sigs, "signal.SIGUSR1", (a -- a Int));
    builtin!(sigs, "signal.SIGUSR2", (a -- a Int));
    builtin!(sigs, "signal.SIGCHLD", (a -- a Int));
    builtin!(sigs, "signal.SIGALRM", (a -- a Int));
    builtin!(sigs, "signal.SIGCONT", (a -- a Int));

    // =========================================================================
    // Terminal Operations (raw mode, character I/O, dimensions)
    // =========================================================================

    builtin!(sigs, "terminal.raw-mode", (a Bool -- a));
    builtin!(sigs, "terminal.read-char", (a -- a Int));
    builtin!(sigs, "terminal.read-char?", (a -- a Int));
    builtin!(sigs, "terminal.width", (a -- a Int));
    builtin!(sigs, "terminal.height", (a -- a Int));
    builtin!(sigs, "terminal.flush", (a - -a));

    // =========================================================================
    // String Operations
    // =========================================================================

    builtin!(sigs, "string.concat", (a String String -- a String));
    builtin!(sigs, "string.length", (a String -- a Int));
    builtin!(sigs, "string.byte-length", (a String -- a Int));
    builtin!(sigs, "string.char-at", (a String Int -- a Int));
    builtin!(sigs, "string.substring", (a String Int Int -- a String));
    builtin!(sigs, "string.find", (a String String -- a Int));
    builtin!(sigs, "string.split", (a String String -- a V)); // Returns Variant (list)
    builtin!(sigs, "string.contains", (a String String -- a Bool));
    builtin!(sigs, "string.starts-with", (a String String -- a Bool));
    builtin!(sigs, "string.empty?", (a String -- a Bool));
    builtin!(sigs, "string.equal?", (a String String -- a Bool));
    builtin!(sigs, "string.join", (a V String -- a String)); // ( list separator -- joined )

    // Symbol operations
    builtin!(sigs, "symbol.=", (a Symbol Symbol -- a Bool));

    // String transformations
    builtins_string_to_string!(
        sigs,
        "string.trim",
        "string.chomp",
        "string.to-upper",
        "string.to-lower",
        "string.json-escape"
    );

    // =========================================================================
    // Encoding Operations
    // =========================================================================

    builtin!(sigs, "encoding.base64-encode", (a String -- a String));
    builtin!(sigs, "encoding.base64-decode", (a String -- a String Bool));
    builtin!(sigs, "encoding.base64url-encode", (a String -- a String));
    builtin!(sigs, "encoding.base64url-decode", (a String -- a String Bool));
    builtin!(sigs, "encoding.hex-encode", (a String -- a String));
    builtin!(sigs, "encoding.hex-decode", (a String -- a String Bool));

    // =========================================================================
    // Crypto Operations
    // =========================================================================

    builtin!(sigs, "crypto.sha256", (a String -- a String));
    builtin!(sigs, "crypto.hmac-sha256", (a String String -- a String));
    builtin!(sigs, "crypto.constant-time-eq", (a String String -- a Bool));
    builtin!(sigs, "crypto.random-bytes", (a Int -- a String));
    builtin!(sigs, "crypto.random-int", (a Int Int -- a Int));
    builtin!(sigs, "crypto.uuid4", (a -- a String));
    builtin!(sigs, "crypto.aes-gcm-encrypt", (a String String -- a String Bool));
    builtin!(sigs, "crypto.aes-gcm-decrypt", (a String String -- a String Bool));
    builtin!(sigs, "crypto.pbkdf2-sha256", (a String String Int -- a String Bool));
    builtin!(sigs, "crypto.ed25519-keypair", (a -- a String String));
    builtin!(sigs, "crypto.ed25519-sign", (a String String -- a String Bool));
    builtin!(sigs, "crypto.ed25519-verify", (a String String String -- a Bool));

    // =========================================================================
    // HTTP Client Operations
    // =========================================================================

    builtin!(sigs, "http.get", (a String -- a M));
    builtin!(sigs, "http.post", (a String String String -- a M));
    builtin!(sigs, "http.put", (a String String String -- a M));
    builtin!(sigs, "http.delete", (a String -- a M));

    // =========================================================================
    // Regular Expression Operations
    // =========================================================================

    // Regex operations return Bool for error handling (invalid regex)
    builtin!(sigs, "regex.match?", (a String String -- a Bool));
    builtin!(sigs, "regex.find", (a String String -- a String Bool));
    builtin!(sigs, "regex.find-all", (a String String -- a V Bool));
    builtin!(sigs, "regex.replace", (a String String String -- a String Bool));
    builtin!(sigs, "regex.replace-all", (a String String String -- a String Bool));
    builtin!(sigs, "regex.captures", (a String String -- a V Bool));
    builtin!(sigs, "regex.split", (a String String -- a V Bool));
    builtin!(sigs, "regex.valid?", (a String -- a Bool));

    // =========================================================================
    // Compression Operations
    // =========================================================================

    builtin!(sigs, "compress.gzip", (a String -- a String Bool));
    builtin!(sigs, "compress.gzip-level", (a String Int -- a String Bool));
    builtin!(sigs, "compress.gunzip", (a String -- a String Bool));
    builtin!(sigs, "compress.zstd", (a String -- a String Bool));
    builtin!(sigs, "compress.zstd-level", (a String Int -- a String Bool));
    builtin!(sigs, "compress.unzstd", (a String -- a String Bool));

    // =========================================================================
    // Variant Operations
    // =========================================================================

    builtin!(sigs, "variant.field-count", (a V -- a Int));
    builtin!(sigs, "variant.tag", (a V -- a Symbol));
    builtin!(sigs, "variant.field-at", (a V Int -- a T));
    builtin!(sigs, "variant.append", (a V T -- a V2));
    builtin!(sigs, "variant.last", (a V -- a T));
    builtin!(sigs, "variant.init", (a V -- a V2));

    // Type-safe variant constructors with fixed arity (symbol tags for SON support)
    builtin!(sigs, "variant.make-0", (a Symbol -- a V));
    builtin!(sigs, "variant.make-1", (a T1 Symbol -- a V));
    builtin!(sigs, "variant.make-2", (a T1 T2 Symbol -- a V));
    builtin!(sigs, "variant.make-3", (a T1 T2 T3 Symbol -- a V));
    builtin!(sigs, "variant.make-4", (a T1 T2 T3 T4 Symbol -- a V));
    // variant.make-5 through variant.make-12 defined manually (macro only supports up to 5 inputs)
    for n in 5..=12 {
        let mut input = StackType::RowVar("a".to_string());
        for i in 1..=n {
            input = input.push(Type::Var(format!("T{}", i)));
        }
        input = input.push(Type::Symbol);
        let output = StackType::RowVar("a".to_string()).push(Type::Var("V".to_string()));
        sigs.insert(format!("variant.make-{}", n), Effect::new(input, output));
    }

    // Aliases for dynamic variant construction (SON-friendly names)
    builtin!(sigs, "wrap-0", (a Symbol -- a V));
    builtin!(sigs, "wrap-1", (a T1 Symbol -- a V));
    builtin!(sigs, "wrap-2", (a T1 T2 Symbol -- a V));
    builtin!(sigs, "wrap-3", (a T1 T2 T3 Symbol -- a V));
    builtin!(sigs, "wrap-4", (a T1 T2 T3 T4 Symbol -- a V));
    // wrap-5 through wrap-12 defined manually
    for n in 5..=12 {
        let mut input = StackType::RowVar("a".to_string());
        for i in 1..=n {
            input = input.push(Type::Var(format!("T{}", i)));
        }
        input = input.push(Type::Symbol);
        let output = StackType::RowVar("a".to_string()).push(Type::Var("V".to_string()));
        sigs.insert(format!("wrap-{}", n), Effect::new(input, output));
    }

    // =========================================================================
    // List Operations (Higher-order combinators for Variants)
    // =========================================================================

    // List construction and access
    builtin!(sigs, "list.make", (a -- a V));
    builtin!(sigs, "list.push", (a V T -- a V));
    builtin!(sigs, "list.get", (a V Int -- a T Bool));
    builtin!(sigs, "list.set", (a V Int T -- a V Bool));

    builtin!(sigs, "list.length", (a V -- a Int));
    builtin!(sigs, "list.empty?", (a V -- a Bool));
    builtin!(sigs, "list.reverse", (a V -- a V));

    // list.map: ( a Variant Quotation -- a Variant )
    // Quotation: ( b T -- b U )
    sigs.insert(
        "list.map".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string())
                .push(Type::Var("V".to_string()))
                .push(Type::Quotation(Box::new(Effect::new(
                    StackType::RowVar("b".to_string()).push(Type::Var("T".to_string())),
                    StackType::RowVar("b".to_string()).push(Type::Var("U".to_string())),
                )))),
            StackType::RowVar("a".to_string()).push(Type::Var("V2".to_string())),
        ),
    );

    // list.filter: ( a Variant Quotation -- a Variant )
    // Quotation: ( b T -- b Bool )
    sigs.insert(
        "list.filter".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string())
                .push(Type::Var("V".to_string()))
                .push(Type::Quotation(Box::new(Effect::new(
                    StackType::RowVar("b".to_string()).push(Type::Var("T".to_string())),
                    StackType::RowVar("b".to_string()).push(Type::Bool),
                )))),
            StackType::RowVar("a".to_string()).push(Type::Var("V2".to_string())),
        ),
    );

    // list.fold: ( a Variant init Quotation -- a result )
    // Quotation: ( b Acc T -- b Acc )
    sigs.insert(
        "list.fold".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string())
                .push(Type::Var("V".to_string()))
                .push(Type::Var("Acc".to_string()))
                .push(Type::Quotation(Box::new(Effect::new(
                    StackType::RowVar("b".to_string())
                        .push(Type::Var("Acc".to_string()))
                        .push(Type::Var("T".to_string())),
                    StackType::RowVar("b".to_string()).push(Type::Var("Acc".to_string())),
                )))),
            StackType::RowVar("a".to_string()).push(Type::Var("Acc".to_string())),
        ),
    );

    // list.each: ( a Variant Quotation -- a )
    // Quotation: ( b T -- b )
    sigs.insert(
        "list.each".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string())
                .push(Type::Var("V".to_string()))
                .push(Type::Quotation(Box::new(Effect::new(
                    StackType::RowVar("b".to_string()).push(Type::Var("T".to_string())),
                    StackType::RowVar("b".to_string()),
                )))),
            StackType::RowVar("a".to_string()),
        ),
    );

    // =========================================================================
    // Map Operations (Dictionary with O(1) lookup)
    // =========================================================================

    builtin!(sigs, "map.make", (a -- a M));
    builtin!(sigs, "map.get", (a M K -- a V Bool)); // returns (value success) - errors are values, not crashes
    builtin!(sigs, "map.set", (a M K V -- a M2));
    builtin!(sigs, "map.has?", (a M K -- a Bool));
    builtin!(sigs, "map.remove", (a M K -- a M2));
    builtin!(sigs, "map.keys", (a M -- a V));
    builtin!(sigs, "map.values", (a M -- a V));
    builtin!(sigs, "map.size", (a M -- a Int));
    builtin!(sigs, "map.empty?", (a M -- a Bool));

    // map.each: ( a Map Quotation -- a )
    // Quotation: ( b K V -- b )
    sigs.insert(
        "map.each".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string())
                .push(Type::Var("M".to_string()))
                .push(Type::Quotation(Box::new(Effect::new(
                    StackType::RowVar("b".to_string())
                        .push(Type::Var("K".to_string()))
                        .push(Type::Var("V".to_string())),
                    StackType::RowVar("b".to_string()),
                )))),
            StackType::RowVar("a".to_string()),
        ),
    );

    // map.fold: ( a Map Acc Quotation -- a Acc )
    // Quotation: ( b Acc K V -- b Acc )
    sigs.insert(
        "map.fold".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string())
                .push(Type::Var("M".to_string()))
                .push(Type::Var("Acc".to_string()))
                .push(Type::Quotation(Box::new(Effect::new(
                    StackType::RowVar("b".to_string())
                        .push(Type::Var("Acc".to_string()))
                        .push(Type::Var("K".to_string()))
                        .push(Type::Var("V".to_string())),
                    StackType::RowVar("b".to_string()).push(Type::Var("Acc".to_string())),
                )))),
            StackType::RowVar("a".to_string()).push(Type::Var("Acc".to_string())),
        ),
    );

    // =========================================================================
    // Float Arithmetic ( a Float Float -- a Float )
    // =========================================================================

    builtins_float_float_to_float!(sigs, "f.add", "f.subtract", "f.multiply", "f.divide");
    builtins_float_float_to_float!(sigs, "f.+", "f.-", "f.*", "f./");

    // =========================================================================
    // Float Comparison ( a Float Float -- a Bool )
    // =========================================================================

    builtins_float_float_to_bool!(sigs, "f.=", "f.<", "f.>", "f.<=", "f.>=", "f.<>");
    builtins_float_float_to_bool!(sigs, "f.eq", "f.lt", "f.gt", "f.lte", "f.gte", "f.neq");

    // =========================================================================
    // Test Framework
    // =========================================================================

    builtin!(sigs, "test.init", (a String -- a));
    builtin!(sigs, "test.finish", (a - -a));
    builtin!(sigs, "test.has-failures", (a -- a Bool));
    builtin!(sigs, "test.assert", (a Bool -- a));
    builtin!(sigs, "test.assert-not", (a Bool -- a));
    builtin!(sigs, "test.assert-eq", (a Int Int -- a));
    builtin!(sigs, "test.assert-eq-str", (a String String -- a));
    builtin!(sigs, "test.fail", (a String -- a));
    builtin!(sigs, "test.pass-count", (a -- a Int));
    builtin!(sigs, "test.fail-count", (a -- a Int));

    // Time operations
    builtin!(sigs, "time.now", (a -- a Int));
    builtin!(sigs, "time.nanos", (a -- a Int));
    builtin!(sigs, "time.sleep-ms", (a Int -- a));

    // SON serialization
    builtin!(sigs, "son.dump", (a T -- a String));
    builtin!(sigs, "son.dump-pretty", (a T -- a String));

    // Stack introspection (for REPL)
    // stack.dump prints all values and clears the stack
    sigs.insert(
        "stack.dump".to_string(),
        Effect::new(
            StackType::RowVar("a".to_string()), // Consumes any stack
            StackType::RowVar("b".to_string()), // Returns empty stack (different row var)
        ),
    );

    sigs
}

mod docs;

#[cfg(test)]
mod tests;

pub use docs::{builtin_doc, builtin_docs};
