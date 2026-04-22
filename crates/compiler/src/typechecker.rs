//! Enhanced type checker for Seq with full type tracking
//!
//! Uses row polymorphism and unification to verify stack effects.
//! Based on cem2's type checker but simplified for Phase 8.5.

use crate::call_graph::CallGraph;
use crate::types::{Effect, StackType, Type, UnionTypeInfo};
use std::collections::HashMap;

/// Format a line number as an error message prefix (e.g., "at line 42: ").
/// Line numbers are 0-indexed internally, so we add 1 for display.
fn format_line_prefix(line: usize) -> String {
    format!("at line {}: ", line + 1)
}

/// Validate that `main` has an allowed signature (Issue #355).
///
/// Only `( -- )` and `( -- Int )` are accepted. The first is "void main"
/// (process exits with 0). The second is "int main" (return value is the
/// process exit code).
///
/// Any other shape — extra inputs, multiple outputs, non-Int output —
/// is rejected with an actionable error.
fn validate_main_effect(effect: &Effect) -> Result<(), String> {
    // Inputs must be empty (just the row var, no concrete types)
    let inputs_ok = matches!(&effect.inputs, StackType::Empty | StackType::RowVar(_));

    // Outputs: either empty (void main) or exactly one Int (int main)
    let outputs_ok = match &effect.outputs {
        StackType::Empty | StackType::RowVar(_) => true,
        StackType::Cons {
            rest,
            top: Type::Int,
        } if matches!(**rest, StackType::Empty | StackType::RowVar(_)) => true,
        _ => false,
    };

    if inputs_ok && outputs_ok {
        return Ok(());
    }

    Err(format!(
        "Word 'main' has an invalid stack effect: ( {} -- {} ).\n\
         `main` must be declared with one of:\n\
           ( -- )       — void main, process exits with code 0\n\
           ( -- Int )   — exit code is the returned Int\n\
         Other shapes are not allowed.",
        effect.inputs, effect.outputs
    ))
}

pub struct TypeChecker {
    /// Environment mapping word names to their effects
    env: HashMap<String, Effect>,
    /// Union type registry - maps union names to their type information
    /// Contains variant names and field types for each union
    unions: HashMap<String, UnionTypeInfo>,
    /// Counter for generating fresh type variables
    fresh_counter: std::cell::Cell<usize>,
    /// Quotation types tracked during type checking
    /// Maps quotation ID (from AST) to inferred type (Quotation or Closure)
    /// This type map is used by codegen to generate appropriate code
    quotation_types: std::cell::RefCell<HashMap<usize, Type>>,
    /// Expected quotation/closure type (from word signature, if any)
    /// Used during type-driven capture inference
    expected_quotation_type: std::cell::RefCell<Option<Type>>,
    /// Current word being type-checked (for detecting recursive tail calls)
    /// Used to identify divergent branches in if/else expressions
    /// Stores (name, line_number) for better error messages
    current_word: std::cell::RefCell<Option<(String, Option<usize>)>>,
    /// Per-statement type info for codegen optimization (Issue #186)
    /// Maps (word_name, statement_index) -> concrete top-of-stack type before statement
    /// Only stores trivially-copyable types (Int, Float, Bool) to enable optimizations
    statement_top_types: std::cell::RefCell<HashMap<(String, usize), Type>>,
    /// Call graph for detecting mutual recursion (Issue #229)
    /// Used to improve divergent branch detection beyond direct recursion
    call_graph: Option<CallGraph>,
    /// Current aux stack type during word body checking (Issue #350)
    /// Tracked per-word; reset to Empty at each word boundary.
    current_aux_stack: std::cell::RefCell<StackType>,
    /// Maximum aux stack depth per word, for codegen alloca sizing (Issue #350)
    /// Maps word_name -> max_depth (number of %Value allocas needed)
    aux_max_depths: std::cell::RefCell<HashMap<String, usize>>,
    /// Maximum aux stack depth per quotation, for codegen alloca sizing (Issue #393)
    /// Maps quotation_id -> max_depth (number of %Value allocas needed)
    /// Quotation IDs are program-wide unique, assigned by the parser.
    quotation_aux_depths: std::cell::RefCell<HashMap<usize, usize>>,
    /// Stack of currently-active quotation IDs during type checking (Issue #393).
    /// Pushed when entering `infer_quotation`, popped on exit. The top of the
    /// stack is the innermost quotation. Empty means we're in word body scope.
    /// Replaces the old `in_quotation_scope` boolean.
    quotation_id_stack: std::cell::RefCell<Vec<usize>>,
    /// Resolved arithmetic sugar: maps (line, column) -> concrete op name.
    /// Keyed by source location, which is unique per occurrence and available
    /// to both the typechecker and codegen via the AST span.
    resolved_sugar: std::cell::RefCell<HashMap<(usize, usize), String>>,
}

mod combinators;
mod control_flow;
mod driver;
mod freshen;
mod pick_roll;
mod quotations;
mod stack_utils;
mod state;
mod validation;
mod words;

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
