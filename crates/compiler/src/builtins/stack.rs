//! Stack shuffles and aux-slot operations.

use std::collections::HashMap;

use crate::types::{Effect, StackType, Type};

use super::macros::*;

pub(super) fn add_signatures(sigs: &mut HashMap<String, Effect>) {
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
}

pub(super) fn add_docs(docs: &mut HashMap<&'static str, &'static str>) {
    // Stack Operations
    docs.insert("dup", "Duplicate the top stack value.");
    docs.insert("drop", "Remove the top stack value.");
    docs.insert("swap", "Swap the top two stack values.");
    docs.insert("over", "Copy the second value to the top.");
    docs.insert("rot", "Rotate the top three values (third to top).");
    docs.insert("nip", "Remove the second value from the stack.");
    docs.insert("tuck", "Copy the top value below the second.");
    docs.insert("2dup", "Duplicate the top two values.");
    docs.insert("3drop", "Remove the top three values.");
    docs.insert("pick", "Copy the value at depth N to the top.");
    docs.insert("roll", "Rotate N+1 items, bringing depth N to top.");

    // Aux Stack Operations
    docs.insert(
        ">aux",
        "Move top of stack to word-local aux stack. Must be balanced with aux> before word returns.",
    );
    docs.insert(
        "aux>",
        "Move top of aux stack back to main stack. Requires a matching >aux.",
    );
}
