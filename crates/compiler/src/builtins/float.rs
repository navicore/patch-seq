//! Float arithmetic and comparison.

use std::collections::HashMap;

use crate::types::{Effect, StackType, Type};

use super::macros::*;

pub(super) fn add_signatures(sigs: &mut HashMap<String, Effect>) {
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
}

pub(super) fn add_docs(docs: &mut HashMap<&'static str, &'static str>) {
    // Float Arithmetic
    docs.insert("f.add", "Add two floats.");
    docs.insert("f.subtract", "Subtract second float from first.");
    docs.insert("f.multiply", "Multiply two floats.");
    docs.insert("f.divide", "Divide first float by second.");
    docs.insert("f.+", "Add two floats.");
    docs.insert("f.-", "Subtract second float from first.");
    docs.insert("f.*", "Multiply two floats.");
    docs.insert("f./", "Divide first float by second.");

    // Float Comparison
    docs.insert("f.=", "Test if two floats are equal.");
    docs.insert("f.<", "Test if first float is less than second.");
    docs.insert("f.>", "Test if first float is greater than second.");
    docs.insert("f.<=", "Test if first float is less than or equal.");
    docs.insert("f.>=", "Test if first float is greater than or equal.");
    docs.insert("f.<>", "Test if two floats are not equal.");
    docs.insert("f.eq", "Test if two floats are equal.");
    docs.insert("f.lt", "Test if first float is less than second.");
    docs.insert("f.gt", "Test if first float is greater than second.");
    docs.insert("f.lte", "Test if first float is less than or equal.");
    docs.insert("f.gte", "Test if first float is greater than or equal.");
    docs.insert("f.neq", "Test if two floats are not equal.");
}
