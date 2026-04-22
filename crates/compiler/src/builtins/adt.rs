//! Variant (algebraic data type) operations.

use std::collections::HashMap;

use crate::types::{Effect, StackType, Type};

use super::macros::*;

pub(super) fn add_signatures(sigs: &mut HashMap<String, Effect>) {
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
}

pub(super) fn add_docs(docs: &mut HashMap<&'static str, &'static str>) {
    // Variant Operations
    docs.insert(
        "variant.field-count",
        "Get the number of fields in a variant.",
    );
    docs.insert(
        "variant.tag",
        "Get the tag (constructor name) of a variant.",
    );
    docs.insert("variant.field-at", "Get the field at index N.");
    docs.insert(
        "variant.append",
        "Append a value to a variant (creates new).",
    );
    docs.insert("variant.last", "Get the last field of a variant.");
    docs.insert("variant.init", "Get all fields except the last.");
    docs.insert("variant.make-0", "Create a variant with 0 fields.");
    docs.insert("variant.make-1", "Create a variant with 1 field.");
    docs.insert("variant.make-2", "Create a variant with 2 fields.");
    docs.insert("variant.make-3", "Create a variant with 3 fields.");
    docs.insert("variant.make-4", "Create a variant with 4 fields.");
    docs.insert("variant.make-5", "Create a variant with 5 fields.");
    docs.insert("variant.make-6", "Create a variant with 6 fields.");
    docs.insert("variant.make-7", "Create a variant with 7 fields.");
    docs.insert("variant.make-8", "Create a variant with 8 fields.");
    docs.insert("variant.make-9", "Create a variant with 9 fields.");
    docs.insert("variant.make-10", "Create a variant with 10 fields.");
    docs.insert("variant.make-11", "Create a variant with 11 fields.");
    docs.insert("variant.make-12", "Create a variant with 12 fields.");
    docs.insert("wrap-0", "Create a variant with 0 fields (alias).");
    docs.insert("wrap-1", "Create a variant with 1 field (alias).");
    docs.insert("wrap-2", "Create a variant with 2 fields (alias).");
    docs.insert("wrap-3", "Create a variant with 3 fields (alias).");
    docs.insert("wrap-4", "Create a variant with 4 fields (alias).");
    docs.insert("wrap-5", "Create a variant with 5 fields (alias).");
    docs.insert("wrap-6", "Create a variant with 6 fields (alias).");
    docs.insert("wrap-7", "Create a variant with 7 fields (alias).");
    docs.insert("wrap-8", "Create a variant with 8 fields (alias).");
    docs.insert("wrap-9", "Create a variant with 9 fields (alias).");
    docs.insert("wrap-10", "Create a variant with 10 fields (alias).");
    docs.insert("wrap-11", "Create a variant with 11 fields (alias).");
    docs.insert("wrap-12", "Create a variant with 12 fields (alias).");
}
