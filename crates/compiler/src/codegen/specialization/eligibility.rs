//! Eligibility analysis for register-based specialization.
//!
//! Decides whether a word is a candidate for the fast-path codegen: its
//! declared effect must use only primitive types, its body must stay within
//! a fixed allowlist of operations (`SPECIALIZABLE_OPS`) or call other
//! already-specialized words, and it must not take/return heap-allocated
//! values. No IR is emitted here — this is read-only analysis.

use super::CodeGen;
use super::types::{RegisterType, SpecSignature};
use crate::ast::{Statement, WordDef};
use crate::types::StackType;

/// Operations that can be emitted in specialized (register-based) mode.
///
/// These operations either:
/// - Map directly to LLVM instructions (arithmetic, comparisons, bitwise)
/// - Use LLVM intrinsics (popcount, clz, ctz)
/// - Are pure context manipulations (stack shuffles like dup, swap, rot)
///
/// Operations NOT in this list (like `print`, `read`, string ops) require
/// the full stack-based calling convention.
pub(super) const SPECIALIZABLE_OPS: &[&str] = &[
    // Integer arithmetic (i./ and i.% emit safe division with zero checks)
    "i.+",
    "i.add",
    "i.-",
    "i.subtract",
    "i.*",
    "i.multiply",
    "i./",
    "i.divide",
    "i.%",
    "i.mod",
    // Bitwise operations
    "band",
    "bor",
    "bxor",
    "bnot",
    "shl",
    "shr",
    // Bit counting operations
    "popcount",
    "clz",
    "ctz",
    // Type conversions
    "int->float",
    "float->int",
    // Boolean logical operations
    "and",
    "or",
    "not",
    // Integer comparisons
    "i.<",
    "i.lt",
    "i.>",
    "i.gt",
    "i.<=",
    "i.lte",
    "i.>=",
    "i.gte",
    "i.=",
    "i.eq",
    "i.<>",
    "i.neq",
    // Float arithmetic
    "f.+",
    "f.add",
    "f.-",
    "f.subtract",
    "f.*",
    "f.multiply",
    "f./",
    "f.divide",
    // Float comparisons
    "f.<",
    "f.lt",
    "f.>",
    "f.gt",
    "f.<=",
    "f.lte",
    "f.>=",
    "f.gte",
    "f.=",
    "f.eq",
    "f.<>",
    "f.neq",
    // Stack operations (handled as context shuffles)
    "dup",
    "drop",
    "swap",
    "over",
    "rot",
    "nip",
    "tuck",
    "pick",
    "roll",
];

impl CodeGen {
    /// Check if a word can be specialized and return its signature if so
    pub fn can_specialize(&self, word: &WordDef) -> Option<SpecSignature> {
        // Must have an effect declaration
        let effect = word.effect.as_ref()?;

        // Must not have side effects (like Yield)
        if !effect.is_pure() {
            return None;
        }

        // Extract input/output types from the effect
        let inputs = Self::extract_register_types(&effect.inputs)?;
        let outputs = Self::extract_register_types(&effect.outputs)?;

        // Must have at least one input or output to optimize
        if inputs.is_empty() && outputs.is_empty() {
            return None;
        }

        // Must have at least one output (zero outputs means side-effect only)
        if outputs.is_empty() {
            return None;
        }

        // Check that the body is specializable
        if !self.is_body_specializable(&word.body, &word.name) {
            return None;
        }

        Some(SpecSignature { inputs, outputs })
    }

    /// Extract register types from a stack type.
    ///
    /// The parser always adds a row variable for composability, so we accept
    /// stack types with a row variable at the base and extract the concrete
    /// types on top of it.
    fn extract_register_types(stack: &StackType) -> Option<Vec<RegisterType>> {
        let mut types = Vec::new();
        let mut current = stack;

        loop {
            match current {
                StackType::Empty => break,
                StackType::RowVar(_) => {
                    // Row variable at the base is OK - we can specialize the
                    // concrete types on top of it. The row variable just means
                    // "whatever else is on the stack stays there".
                    break;
                }
                StackType::Cons { rest, top } => {
                    let reg_ty = RegisterType::from_type(top)?;
                    types.push(reg_ty);
                    current = rest;
                }
            }
        }

        // Reverse to get bottom-to-top order
        types.reverse();
        Some(types)
    }

    /// Check if a word body can be specialized.
    ///
    /// Tracks whether the previous statement was an integer literal, which is
    /// required for `pick` and `roll` operations in specialized mode.
    pub(super) fn is_body_specializable(&self, body: &[Statement], word_name: &str) -> bool {
        let mut prev_was_int_literal = false;
        for stmt in body {
            if !self.is_statement_specializable(stmt, word_name, prev_was_int_literal) {
                return false;
            }
            prev_was_int_literal = matches!(stmt, Statement::IntLiteral(_));
        }
        true
    }

    /// Check if a single statement can be specialized.
    ///
    /// `prev_was_int_literal` indicates whether the preceding statement was an
    /// IntLiteral, which is required for `pick` and `roll` to be specializable.
    fn is_statement_specializable(
        &self,
        stmt: &Statement,
        word_name: &str,
        prev_was_int_literal: bool,
    ) -> bool {
        match stmt {
            Statement::IntLiteral(_) => true,
            Statement::FloatLiteral(_) => true,
            Statement::BoolLiteral(_) => true,
            // Heap-backed literals cannot flow through registers.
            Statement::StringLiteral(_) => false,
            Statement::Symbol(_) => false,
            Statement::Quotation { .. } => false,
            Statement::Match { .. } => false,

            Statement::WordCall { name, .. } => {
                // Recursive calls to self are OK (we'll call specialized version)
                if name == word_name {
                    return true;
                }

                // pick and roll require a compile-time constant N (previous int literal)
                if (name == "pick" || name == "roll") && !prev_was_int_literal {
                    return false;
                }

                if SPECIALIZABLE_OPS.contains(&name.as_str()) {
                    return true;
                }

                if self.specialized_words.contains_key(name) {
                    return true;
                }

                false
            }

            Statement::If {
                then_branch,
                else_branch,
                span: _,
            } => {
                if !self.is_body_specializable(then_branch, word_name) {
                    return false;
                }
                if let Some(else_stmts) = else_branch
                    && !self.is_body_specializable(else_stmts, word_name)
                {
                    return false;
                }
                true
            }
        }
    }
}
