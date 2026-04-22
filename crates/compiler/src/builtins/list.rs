//! List operations.

use std::collections::HashMap;

use crate::types::{Effect, StackType, Type};

use super::macros::*;

pub(super) fn add_signatures(sigs: &mut HashMap<String, Effect>) {
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
}

pub(super) fn add_docs(docs: &mut HashMap<&'static str, &'static str>) {
    // List Operations
    docs.insert("list.make", "Create an empty list.");
    docs.insert("list.push", "Push a value onto a list. Returns new list.");
    docs.insert(
        "list.get",
        "Get value at index. Returns (value Bool) -- Bool is false if index out of bounds.",
    );
    docs.insert(
        "list.set",
        "Set value at index. Returns (List Bool) -- Bool is false if index out of bounds.",
    );
    docs.insert("list.length", "Get the number of elements in a list.");
    docs.insert("list.empty?", "Check if a list is empty.");
    docs.insert("list.reverse", "Reverse the elements of a list.");
    docs.insert(
        "list.map",
        "Apply quotation to each element. Returns new list.",
    );
    docs.insert("list.filter", "Keep elements where quotation returns true.");
    docs.insert("list.fold", "Reduce list with accumulator and quotation.");
    docs.insert(
        "list.each",
        "Execute quotation for each element (side effects).",
    );
}
