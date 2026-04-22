//! Capture Analysis for Closures
//!
//! This module handles the analysis of closure captures - determining which values
//! from the creation site need to be captured in a closure's environment.
//!
//! The key insight is that closures bridge two stack effects:
//! - **Body effect**: what the quotation body actually needs to execute
//! - **Call effect**: what the call site will provide when the closure is invoked
//!
//! The difference between these determines what must be captured at creation time.
//!
//! ## Example
//!
//! ```text
//! : add-to ( Int -- [Int -- Int] )
//!   [ add ] ;
//! ```
//!
//! Here:
//! - Body needs: `(Int Int -- Int)` (add requires two integers)
//! - Call provides: `(Int -- Int)` (caller provides one integer)
//! - Captures: `[Int]` (one integer captured from creation site)

use crate::types::{Effect, StackType, Type};

/// Calculate capture types for a closure
///
/// Given:
/// - `body_effect`: what the quotation body needs (e.g., `Int Int -- Int`)
/// - `call_effect`: what the call site will provide (e.g., `Int -- Int`)
///
/// Returns:
/// - `captures`: types to capture from creation stack (e.g., `[Int]`)
///
/// # Capture Ordering
///
/// Captures are returned bottom-to-top (deepest value first), matching the
/// runtime's env layout and the order the closure body pushes them:
///
/// ```text
/// Stack at creation: ( ...rest bottom top )
/// push_closure pops top-down, then reverses:
///   env[0] = bottom (caller's deepest capture)
///   env[N-1] = top   (caller's shallowest capture)
/// Closure function pushes env[0], env[1], ..., env[N-1] in order, so
/// the body stack looks like ( ...rest bottom top ) — identical to the
/// caller's visual order.
/// ```
///
/// Single captures are unaffected (a one-element vector reversed is itself).
///
/// # Errors
///
/// Returns an error if the call site provides more values than the body needs.
pub fn calculate_captures(body_effect: &Effect, call_effect: &Effect) -> Result<Vec<Type>, String> {
    // Extract concrete types from stack types (bottom to top)
    let body_inputs = extract_concrete_types(&body_effect.inputs);
    let call_inputs = extract_concrete_types(&call_effect.inputs);

    // Validate: call site shouldn't provide MORE than body needs
    if call_inputs.len() > body_inputs.len() {
        return Err(format!(
            "Closure signature error: call site provides {} values but body only needs {}",
            call_inputs.len(),
            body_inputs.len()
        ));
    }

    // Calculate how many to capture (from bottom of stack)
    let capture_count = body_inputs.len() - call_inputs.len();

    // Verify the topmost body inputs (the non-captured ones) align with
    // what the call site provides. If they don't match, the body is
    // incompatible with the combinator regardless of captures.
    let body_provided = &body_inputs[capture_count..];
    for (i, (body_type, call_type)) in body_provided.iter().zip(call_inputs.iter()).enumerate() {
        if body_type != call_type {
            // Type variables (like Acc, T from row polymorphism) won't match
            // concrete types here — that's expected, because the body's types
            // are inferred from a seeded row-variable stack. Skip the check
            // for type variables; they'll be verified by downstream unification.
            let is_var = matches!(body_type, Type::Var(_)) || matches!(call_type, Type::Var(_));
            if !is_var {
                return Err(format!(
                    "Closure capture error: body input at position {} (from top) is {}, \
                     but combinator provides {}. The non-captured inputs must match.",
                    i, body_type, call_type
                ));
            }
        }
    }

    // Captures are the first N types (bottom of stack)
    // Example: body needs [Int, String] (bottom to top), call provides [String]
    // Captures: [Int] (the bottom type)
    Ok(body_inputs[0..capture_count].to_vec())
}

/// Extract concrete types from a stack type (bottom to top order)
///
/// This function traverses a `StackType` and returns a vector of concrete types
/// in bottom-to-top order (deepest stack element first).
///
/// # Example
///
/// ```text
/// Input: Cons { rest: Cons { rest: Empty, top: Int }, top: String }
/// Output: [Int, String]  (bottom to top)
/// ```
///
/// # Row Variables
///
/// Row variables (like `..a`) are skipped - this function only extracts
/// concrete types. This is appropriate for capture analysis where we need
/// to know the actual types being captured.
///
/// # Performance
///
/// Uses recursion to build the vector in the correct order without needing
/// to clone the entire stack structure or reverse the result.
pub(crate) fn extract_concrete_types(stack: &StackType) -> Vec<Type> {
    // Use recursion to build the vector in bottom-to-top order
    fn collect(stack: &StackType, result: &mut Vec<Type>) {
        match stack {
            StackType::Cons { rest, top } => {
                // First recurse to collect types below, then add this type
                collect(rest, result);
                result.push(top.clone());
            }
            StackType::Empty | StackType::RowVar(_) => {
                // Base case: nothing more to collect
            }
        }
    }

    let mut types = Vec::new();
    collect(stack, &mut types);
    types
}

#[cfg(test)]
mod tests;
