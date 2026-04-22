//! Enhanced type checker for Seq with full type tracking
//!
//! Uses row polymorphism and unification to verify stack effects.
//! Based on cem2's type checker but simplified for Phase 8.5.

use crate::ast::{Program, Statement, WordDef};
use crate::builtins::builtin_signature;
use crate::call_graph::CallGraph;
use crate::capture_analysis::{calculate_captures, extract_concrete_types};
use crate::types::{
    Effect, SideEffect, StackType, Type, UnionTypeInfo, VariantFieldInfo, VariantInfo,
};
use crate::unification::{Subst, unify_stacks, unify_types};
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

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            env: HashMap::new(),
            unions: HashMap::new(),
            fresh_counter: std::cell::Cell::new(0),
            quotation_types: std::cell::RefCell::new(HashMap::new()),
            expected_quotation_type: std::cell::RefCell::new(None),
            current_word: std::cell::RefCell::new(None),
            statement_top_types: std::cell::RefCell::new(HashMap::new()),
            call_graph: None,
            current_aux_stack: std::cell::RefCell::new(StackType::Empty),
            aux_max_depths: std::cell::RefCell::new(HashMap::new()),
            quotation_aux_depths: std::cell::RefCell::new(HashMap::new()),
            quotation_id_stack: std::cell::RefCell::new(Vec::new()),
            resolved_sugar: std::cell::RefCell::new(HashMap::new()),
        }
    }

    /// Set the call graph for mutual recursion detection.
    ///
    /// When set, the type checker can detect divergent branches caused by
    /// mutual recursion (e.g., even/odd pattern) in addition to direct recursion.
    pub fn set_call_graph(&mut self, call_graph: CallGraph) {
        self.call_graph = Some(call_graph);
    }

    /// Get line info prefix for error messages (e.g., "at line 42: " or "")
    fn line_prefix(&self) -> String {
        self.current_word
            .borrow()
            .as_ref()
            .and_then(|(_, line)| line.map(format_line_prefix))
            .unwrap_or_default()
    }

    /// Look up a union type by name
    pub fn get_union(&self, name: &str) -> Option<&UnionTypeInfo> {
        self.unions.get(name)
    }

    /// Get all registered union types
    pub fn get_unions(&self) -> &HashMap<String, UnionTypeInfo> {
        &self.unions
    }

    /// Find variant info by name across all unions
    ///
    /// Returns (union_name, variant_info) for the variant
    fn find_variant(&self, variant_name: &str) -> Option<(&str, &VariantInfo)> {
        for (union_name, union_info) in &self.unions {
            for variant in &union_info.variants {
                if variant.name == variant_name {
                    return Some((union_name.as_str(), variant));
                }
            }
        }
        None
    }

    /// Register external word effects (e.g., from included modules or FFI).
    ///
    /// All external words must have explicit stack effects for type safety.
    pub fn register_external_words(&mut self, words: &[(&str, &Effect)]) {
        for (name, effect) in words {
            self.env.insert(name.to_string(), (*effect).clone());
        }
    }

    /// Register external union type names (e.g., from included modules).
    ///
    /// This allows field types in union definitions to reference types from includes.
    /// We only register the name as a valid type; we don't need full variant info
    /// since the actual union definition lives in the included file.
    pub fn register_external_unions(&mut self, union_names: &[&str]) {
        for name in union_names {
            // Insert a placeholder union with no variants
            // This makes is_valid_type_name() return true for this type
            self.unions.insert(
                name.to_string(),
                UnionTypeInfo {
                    name: name.to_string(),
                    variants: vec![],
                },
            );
        }
    }

    /// Extract the type map (quotation ID -> inferred type)
    ///
    /// This should be called after check_program() to get the inferred types
    /// for all quotations in the program. The map is used by codegen to generate
    /// appropriate code for Quotations vs Closures.
    pub fn take_quotation_types(&self) -> HashMap<usize, Type> {
        self.quotation_types.replace(HashMap::new())
    }

    /// Extract per-statement type info for codegen optimization (Issue #186)
    /// Returns map of (word_name, statement_index) -> top-of-stack type
    pub fn take_statement_top_types(&self) -> HashMap<(String, usize), Type> {
        self.statement_top_types.replace(HashMap::new())
    }

    /// Extract resolved arithmetic sugar for codegen
    /// Maps (line, column) -> concrete operation name
    pub fn take_resolved_sugar(&self) -> HashMap<(usize, usize), String> {
        self.resolved_sugar.replace(HashMap::new())
    }

    /// Extract per-word aux stack max depths for codegen alloca sizing (Issue #350)
    pub fn take_aux_max_depths(&self) -> HashMap<String, usize> {
        self.aux_max_depths.replace(HashMap::new())
    }

    /// Extract per-quotation aux stack max depths for codegen alloca sizing (Issue #393)
    /// Maps quotation_id -> max_depth
    pub fn take_quotation_aux_depths(&self) -> HashMap<usize, usize> {
        self.quotation_aux_depths.replace(HashMap::new())
    }

    /// Count the number of concrete types in a StackType (for aux depth tracking)
    fn stack_depth(stack: &StackType) -> usize {
        let mut depth = 0;
        let mut current = stack;
        while let StackType::Cons { rest, .. } = current {
            depth += 1;
            current = rest;
        }
        depth
    }

    /// Check if the top of the stack is a trivially-copyable type (Int, Float, Bool)
    /// These types have no heap references and can be memcpy'd in codegen.
    fn get_trivially_copyable_top(stack: &StackType) -> Option<Type> {
        match stack {
            StackType::Cons { top, .. } => match top {
                Type::Int | Type::Float | Type::Bool => Some(top.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    /// Record the top-of-stack type for a statement if it's trivially copyable (Issue #186)
    fn capture_statement_type(&self, word_name: &str, stmt_index: usize, stack: &StackType) {
        if let Some(top_type) = Self::get_trivially_copyable_top(stack) {
            self.statement_top_types
                .borrow_mut()
                .insert((word_name.to_string(), stmt_index), top_type);
        }
    }

    /// Generate a fresh variable name
    fn fresh_var(&self, prefix: &str) -> String {
        let n = self.fresh_counter.get();
        self.fresh_counter.set(n + 1);
        format!("{}${}", prefix, n)
    }

    /// Freshen all type and row variables in an effect
    fn freshen_effect(&self, effect: &Effect) -> Effect {
        let mut type_map = HashMap::new();
        let mut row_map = HashMap::new();

        let fresh_inputs = self.freshen_stack(&effect.inputs, &mut type_map, &mut row_map);
        let fresh_outputs = self.freshen_stack(&effect.outputs, &mut type_map, &mut row_map);

        // Freshen the side effects too
        let fresh_effects = effect
            .effects
            .iter()
            .map(|e| self.freshen_side_effect(e, &mut type_map, &mut row_map))
            .collect();

        Effect::with_effects(fresh_inputs, fresh_outputs, fresh_effects)
    }

    fn freshen_side_effect(
        &self,
        effect: &SideEffect,
        type_map: &mut HashMap<String, String>,
        row_map: &mut HashMap<String, String>,
    ) -> SideEffect {
        match effect {
            SideEffect::Yield(ty) => {
                SideEffect::Yield(Box::new(self.freshen_type(ty, type_map, row_map)))
            }
        }
    }

    fn freshen_stack(
        &self,
        stack: &StackType,
        type_map: &mut HashMap<String, String>,
        row_map: &mut HashMap<String, String>,
    ) -> StackType {
        match stack {
            StackType::Empty => StackType::Empty,
            StackType::RowVar(name) => {
                let fresh_name = row_map
                    .entry(name.clone())
                    .or_insert_with(|| self.fresh_var(name));
                StackType::RowVar(fresh_name.clone())
            }
            StackType::Cons { rest, top } => {
                let fresh_rest = self.freshen_stack(rest, type_map, row_map);
                let fresh_top = self.freshen_type(top, type_map, row_map);
                StackType::Cons {
                    rest: Box::new(fresh_rest),
                    top: fresh_top,
                }
            }
        }
    }

    fn freshen_type(
        &self,
        ty: &Type,
        type_map: &mut HashMap<String, String>,
        row_map: &mut HashMap<String, String>,
    ) -> Type {
        match ty {
            Type::Int | Type::Float | Type::Bool | Type::String | Type::Symbol | Type::Channel => {
                ty.clone()
            }
            Type::Var(name) => {
                let fresh_name = type_map
                    .entry(name.clone())
                    .or_insert_with(|| self.fresh_var(name));
                Type::Var(fresh_name.clone())
            }
            Type::Quotation(effect) => {
                let fresh_inputs = self.freshen_stack(&effect.inputs, type_map, row_map);
                let fresh_outputs = self.freshen_stack(&effect.outputs, type_map, row_map);
                Type::Quotation(Box::new(Effect::new(fresh_inputs, fresh_outputs)))
            }
            Type::Closure { effect, captures } => {
                let fresh_inputs = self.freshen_stack(&effect.inputs, type_map, row_map);
                let fresh_outputs = self.freshen_stack(&effect.outputs, type_map, row_map);
                let fresh_captures = captures
                    .iter()
                    .map(|t| self.freshen_type(t, type_map, row_map))
                    .collect();
                Type::Closure {
                    effect: Box::new(Effect::new(fresh_inputs, fresh_outputs)),
                    captures: fresh_captures,
                }
            }
            // Union types are concrete named types - no freshening needed
            Type::Union(name) => Type::Union(name.clone()),
        }
    }

    /// Parse a type name string into a Type
    ///
    /// Supports: Int, Float, Bool, String, Channel, and union types
    fn parse_type_name(&self, name: &str) -> Type {
        match name {
            "Int" => Type::Int,
            "Float" => Type::Float,
            "Bool" => Type::Bool,
            "String" => Type::String,
            "Channel" => Type::Channel,
            // Any other name is assumed to be a union type reference
            other => Type::Union(other.to_string()),
        }
    }

    /// Check if a type name is a known valid type
    ///
    /// Returns true for built-in types (Int, Float, Bool, String, Channel) and
    /// registered union type names
    fn is_valid_type_name(&self, name: &str) -> bool {
        matches!(name, "Int" | "Float" | "Bool" | "String" | "Channel")
            || self.unions.contains_key(name)
    }

    /// Validate that all field types in union definitions reference known types
    ///
    /// Note: Field count validation happens earlier in generate_constructors()
    fn validate_union_field_types(&self, program: &Program) -> Result<(), String> {
        for union_def in &program.unions {
            for variant in &union_def.variants {
                for field in &variant.fields {
                    if !self.is_valid_type_name(&field.type_name) {
                        return Err(format!(
                            "Unknown type '{}' in field '{}' of variant '{}' in union '{}'. \
                             Valid types are: Int, Float, Bool, String, Channel, or a defined union name.",
                            field.type_name, field.name, variant.name, union_def.name
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate that all types in a stack effect are known types
    ///
    /// RFC #345: This catches cases where an uppercase identifier was parsed as a type
    /// variable but should have been a union type (e.g., from an include that wasn't
    /// available when parsing). Type variables should be single uppercase letters (T, U, V).
    fn validate_effect_types(&self, effect: &Effect, word_name: &str) -> Result<(), String> {
        self.validate_stack_types(&effect.inputs, word_name)?;
        self.validate_stack_types(&effect.outputs, word_name)?;
        Ok(())
    }

    /// Validate types in a stack type
    fn validate_stack_types(&self, stack: &StackType, word_name: &str) -> Result<(), String> {
        match stack {
            StackType::Empty | StackType::RowVar(_) => Ok(()),
            StackType::Cons { rest, top } => {
                self.validate_type(top, word_name)?;
                self.validate_stack_types(rest, word_name)
            }
        }
    }

    /// Validate a single type
    ///
    /// Type variables are allowed - they're used for polymorphism.
    /// Only Type::Union types are validated to ensure they're registered.
    fn validate_type(&self, ty: &Type, word_name: &str) -> Result<(), String> {
        match ty {
            Type::Var(_) => {
                // Type variables are always valid - they represent polymorphic types
                // Examples: T, U, V, Ctx, Handle, Acc, etc.
                // After fixup_union_types(), any union name that was mistakenly parsed
                // as a type variable will have been converted to Type::Union
                Ok(())
            }
            Type::Quotation(effect) => self.validate_effect_types(effect, word_name),
            Type::Closure { effect, captures } => {
                self.validate_effect_types(effect, word_name)?;
                for cap in captures {
                    self.validate_type(cap, word_name)?;
                }
                Ok(())
            }
            // Concrete types are always valid
            Type::Int | Type::Float | Type::Bool | Type::String | Type::Symbol | Type::Channel => {
                Ok(())
            }
            // Union types are valid if they're registered
            Type::Union(name) => {
                if !self.unions.contains_key(name) {
                    return Err(format!(
                        "In word '{}': Unknown union type '{}' in stack effect.\n\
                         Make sure this union is defined before the word that uses it.",
                        word_name, name
                    ));
                }
                Ok(())
            }
        }
    }

    /// Type check a complete program
    pub fn check_program(&mut self, program: &Program) -> Result<(), String> {
        // First pass: register all union definitions
        for union_def in &program.unions {
            let variants = union_def
                .variants
                .iter()
                .map(|v| VariantInfo {
                    name: v.name.clone(),
                    fields: v
                        .fields
                        .iter()
                        .map(|f| VariantFieldInfo {
                            name: f.name.clone(),
                            field_type: self.parse_type_name(&f.type_name),
                        })
                        .collect(),
                })
                .collect();

            self.unions.insert(
                union_def.name.clone(),
                UnionTypeInfo {
                    name: union_def.name.clone(),
                    variants,
                },
            );
        }

        // Validate field types in unions reference known types
        self.validate_union_field_types(program)?;

        // Second pass: collect all word signatures
        // All words must have explicit stack effect declarations (v2.0 requirement)
        for word in &program.words {
            if let Some(effect) = &word.effect {
                // RFC #345: Validate that all types in the effect are known
                // This catches cases where an uppercase identifier was parsed as a type variable
                // but should have been a union type (e.g., from an include)
                self.validate_effect_types(effect, &word.name)?;
                self.env.insert(word.name.clone(), effect.clone());
            } else {
                return Err(format!(
                    "Word '{}' is missing a stack effect declaration.\n\
                     All words must declare their stack effect, e.g.: : {} ( -- ) ... ;",
                    word.name, word.name
                ));
            }
        }

        // Validate main's signature (Issue #355).
        // Only `( -- )` and `( -- Int )` are allowed.
        if let Some(main_effect) = self.env.get("main") {
            validate_main_effect(main_effect)?;
        }

        // Third pass: type check each word body
        for word in &program.words {
            self.check_word(word)?;
        }

        Ok(())
    }

    /// Type check a word definition
    fn check_word(&self, word: &WordDef) -> Result<(), String> {
        // Track current word for detecting recursive tail calls (divergent branches)
        let line = word.source.as_ref().map(|s| s.start_line);
        *self.current_word.borrow_mut() = Some((word.name.clone(), line));

        // Reset aux stack for this word (Issue #350)
        *self.current_aux_stack.borrow_mut() = StackType::Empty;

        // All words must have declared effects (enforced in check_program)
        let declared_effect = word.effect.as_ref().expect("word must have effect");

        // Check if the word's output type is a quotation or closure
        // If so, store it as the expected type for capture inference
        if let Some((_rest, top_type)) = declared_effect.outputs.clone().pop()
            && matches!(top_type, Type::Quotation(_) | Type::Closure { .. })
        {
            *self.expected_quotation_type.borrow_mut() = Some(top_type);
        }

        // Infer the result stack and effects starting from declared input
        let (result_stack, _subst, inferred_effects) =
            self.infer_statements_from(&word.body, &declared_effect.inputs, true)?;

        // Clear expected type after checking
        *self.expected_quotation_type.borrow_mut() = None;

        // Verify result matches declared output
        let line_info = line.map(format_line_prefix).unwrap_or_default();
        unify_stacks(&declared_effect.outputs, &result_stack).map_err(|e| {
            format!(
                "{}Word '{}': declared output stack ({}) doesn't match inferred ({}): {}",
                line_info, word.name, declared_effect.outputs, result_stack, e
            )
        })?;

        // Verify computational effects match (bidirectional)
        // 1. Check that each inferred effect has a matching declared effect (by kind)
        // Type variables in effects are matched by kind (Yield matches Yield)
        for inferred in &inferred_effects {
            if !self.effect_matches_any(inferred, &declared_effect.effects) {
                return Err(format!(
                    "{}Word '{}': body produces effect '{}' but no matching effect is declared.\n\
                     Hint: Add '| Yield <type>' to the word's stack effect declaration.",
                    line_info, word.name, inferred
                ));
            }
        }

        // 2. Check that each declared effect is actually produced (effect soundness)
        // This prevents declaring effects that don't occur
        for declared in &declared_effect.effects {
            if !self.effect_matches_any(declared, &inferred_effects) {
                return Err(format!(
                    "{}Word '{}': declares effect '{}' but body doesn't produce it.\n\
                     Hint: Remove the effect declaration or ensure the body uses yield.",
                    line_info, word.name, declared
                ));
            }
        }

        // Verify aux stack is empty at word boundary (Issue #350)
        let aux_stack = self.current_aux_stack.borrow().clone();
        if aux_stack != StackType::Empty {
            return Err(format!(
                "{}Word '{}': aux stack is not empty at word return.\n\
                 Remaining aux stack: {}\n\
                 Every >aux must be matched by a corresponding aux> before the word returns.",
                line_info, word.name, aux_stack
            ));
        }

        // Clear current word
        *self.current_word.borrow_mut() = None;

        Ok(())
    }

    /// Infer the resulting stack type from a sequence of statements
    /// starting from a given input stack
    /// Returns (final_stack, substitution, accumulated_effects)
    ///
    /// `capture_stmt_types`: If true, capture statement type info for codegen optimization.
    /// Should only be true for top-level word bodies, not for nested branches/loops.
    fn infer_statements_from(
        &self,
        statements: &[Statement],
        start_stack: &StackType,
        capture_stmt_types: bool,
    ) -> Result<(StackType, Subst, Vec<SideEffect>), String> {
        let mut current_stack = start_stack.clone();
        let mut accumulated_subst = Subst::empty();
        let mut accumulated_effects: Vec<SideEffect> = Vec::new();
        let mut skip_next = false;

        for (i, stmt) in statements.iter().enumerate() {
            // Skip this statement if we already handled it (e.g., pick/roll after literal)
            if skip_next {
                skip_next = false;
                continue;
            }

            // Special case: IntLiteral followed by pick or roll
            // Handle them as a fused operation with correct type semantics
            if let Statement::IntLiteral(n) = stmt
                && let Some(Statement::WordCall {
                    name: next_word, ..
                }) = statements.get(i + 1)
            {
                if next_word == "pick" {
                    let (new_stack, subst) = self.handle_literal_pick(*n, current_stack.clone())?;
                    current_stack = new_stack;
                    accumulated_subst = accumulated_subst.compose(&subst);
                    skip_next = true; // Skip the "pick" word
                    continue;
                } else if next_word == "roll" {
                    let (new_stack, subst) = self.handle_literal_roll(*n, current_stack.clone())?;
                    current_stack = new_stack;
                    accumulated_subst = accumulated_subst.compose(&subst);
                    skip_next = true; // Skip the "roll" word
                    continue;
                }
            }

            // Look ahead: if this is a quotation followed by a word that expects specific quotation type,
            // set the expected type before checking the quotation
            let saved_expected_type = if matches!(stmt, Statement::Quotation { .. }) {
                // Save the current expected type
                let saved = self.expected_quotation_type.borrow().clone();

                // Try to set expected type based on lookahead
                if let Some(Statement::WordCall {
                    name: next_word, ..
                }) = statements.get(i + 1)
                {
                    // Check if the next word expects a specific quotation type
                    if let Some(next_effect) = self.lookup_word_effect(next_word) {
                        // Extract the quotation type expected by the next word
                        // For operations like spawn: ( ..a Quotation(-- ) -- ..a Int )
                        if let Some((_rest, quot_type)) = next_effect.inputs.clone().pop()
                            && matches!(quot_type, Type::Quotation(_))
                        {
                            *self.expected_quotation_type.borrow_mut() = Some(quot_type);
                        }
                    }
                }
                Some(saved)
            } else {
                None
            };

            // Capture statement type info for codegen optimization (Issue #186)
            // Record the top-of-stack type BEFORE this statement for operations like dup
            // Only capture for top-level word bodies, not nested branches/loops
            if capture_stmt_types && let Some((word_name, _)) = self.current_word.borrow().as_ref()
            {
                self.capture_statement_type(word_name, i, &current_stack);
            }

            let (new_stack, subst, effects) = self.infer_statement(stmt, current_stack)?;
            current_stack = new_stack;
            accumulated_subst = accumulated_subst.compose(&subst);

            // Accumulate side effects from this statement
            for effect in effects {
                if !accumulated_effects.contains(&effect) {
                    accumulated_effects.push(effect);
                }
            }

            // Restore expected type after checking quotation
            if let Some(saved) = saved_expected_type {
                *self.expected_quotation_type.borrow_mut() = saved;
            }
        }

        Ok((current_stack, accumulated_subst, accumulated_effects))
    }

    /// Handle `n pick` where n is a literal integer
    ///
    /// pick(n) copies the value at position n to the top of the stack.
    /// Position 0 is the top, 1 is below top, etc.
    ///
    /// Example: `2 pick` on stack ( A B C ) produces ( A B C A )
    /// - Position 0: C (top)
    /// - Position 1: B
    /// - Position 2: A
    /// - Result: copy A to top
    fn handle_literal_pick(
        &self,
        n: i64,
        current_stack: StackType,
    ) -> Result<(StackType, Subst), String> {
        if n < 0 {
            return Err(format!("pick: index must be non-negative, got {}", n));
        }

        // Get the type at position n
        let type_at_n = self.get_type_at_position(&current_stack, n as usize, "pick")?;

        // Push a copy of that type onto the stack
        Ok((current_stack.push(type_at_n), Subst::empty()))
    }

    /// Handle `n roll` where n is a literal integer
    ///
    /// roll(n) moves the value at position n to the top of the stack,
    /// shifting all items above it down by one position.
    ///
    /// Example: `2 roll` on stack ( A B C ) produces ( B C A )
    /// - Position 0: C (top)
    /// - Position 1: B
    /// - Position 2: A
    /// - Result: move A to top, B and C shift down
    fn handle_literal_roll(
        &self,
        n: i64,
        current_stack: StackType,
    ) -> Result<(StackType, Subst), String> {
        if n < 0 {
            return Err(format!("roll: index must be non-negative, got {}", n));
        }

        // For roll, we need to:
        // 1. Extract the type at position n
        // 2. Remove it from that position
        // 3. Push it on top
        self.rotate_type_to_top(current_stack, n as usize)
    }

    /// Get the type at position n in the stack (0 = top)
    fn get_type_at_position(&self, stack: &StackType, n: usize, op: &str) -> Result<Type, String> {
        let mut current = stack;
        let mut pos = 0;

        loop {
            match current {
                StackType::Cons { rest, top } => {
                    if pos == n {
                        return Ok(top.clone());
                    }
                    pos += 1;
                    current = rest;
                }
                StackType::RowVar(name) => {
                    // We've hit a row variable before reaching position n
                    // This means the type at position n is unknown statically.
                    // Generate a fresh type variable to represent it.
                    // This allows the code to type-check, with the actual type
                    // determined by unification with how the value is used.
                    //
                    // Note: This works correctly even in conditional branches because
                    // branches are now inferred from the actual stack (not abstractly),
                    // so row variables only appear when the word itself has polymorphic inputs.
                    let fresh_type = Type::Var(self.fresh_var(&format!("{}_{}", op, name)));
                    return Ok(fresh_type);
                }
                StackType::Empty => {
                    return Err(format!(
                        "{}{}: stack underflow - position {} requested but stack has only {} concrete items",
                        self.line_prefix(),
                        op,
                        n,
                        pos
                    ));
                }
            }
        }
    }

    /// Remove the type at position n and push it on top (for roll)
    fn rotate_type_to_top(&self, stack: StackType, n: usize) -> Result<(StackType, Subst), String> {
        if n == 0 {
            // roll(0) is a no-op
            return Ok((stack, Subst::empty()));
        }

        // Collect all types from top to the target position
        let mut types_above: Vec<Type> = Vec::new();
        let mut current = stack;
        let mut pos = 0;

        // Pop items until we reach position n
        loop {
            match current {
                StackType::Cons { rest, top } => {
                    if pos == n {
                        // Found the target - 'top' is what we want to move to the top
                        // Rebuild the stack: rest, then types_above (reversed), then top
                        let mut result = *rest;
                        // Push types_above back in reverse order (bottom to top)
                        for ty in types_above.into_iter().rev() {
                            result = result.push(ty);
                        }
                        // Push the rotated type on top
                        result = result.push(top);
                        return Ok((result, Subst::empty()));
                    }
                    types_above.push(top);
                    pos += 1;
                    current = *rest;
                }
                StackType::RowVar(name) => {
                    // Reached a row variable before position n
                    // The type at position n is in the row variable.
                    // Generate a fresh type variable to represent the moved value.
                    //
                    // Note: This preserves stack size correctly because we're moving
                    // (not copying) a value. The row variable conceptually "loses"
                    // an item which appears on top. Since we can't express "row minus one",
                    // we generate a fresh type and trust unification to constrain it.
                    //
                    // This works correctly in conditional branches because branches are
                    // now inferred from the actual stack (not abstractly), so row variables
                    // only appear when the word itself has polymorphic inputs.
                    let fresh_type = Type::Var(self.fresh_var(&format!("roll_{}", name)));

                    // Reconstruct the stack with the rolled type on top
                    let mut result = StackType::RowVar(name.clone());
                    for ty in types_above.into_iter().rev() {
                        result = result.push(ty);
                    }
                    result = result.push(fresh_type);
                    return Ok((result, Subst::empty()));
                }
                StackType::Empty => {
                    return Err(format!(
                        "{}roll: stack underflow - position {} requested but stack has only {} items",
                        self.line_prefix(),
                        n,
                        pos
                    ));
                }
            }
        }
    }

    /// Infer the stack effect of a sequence of statements
    /// Returns an Effect with both inputs and outputs normalized by applying discovered substitutions
    /// Also includes any computational side effects (Yield, etc.)
    fn infer_statements(&self, statements: &[Statement]) -> Result<Effect, String> {
        let start = StackType::RowVar("input".to_string());
        // Don't capture statement types for quotation bodies - only top-level word bodies
        let (result, subst, effects) = self.infer_statements_from(statements, &start, false)?;

        // Apply the accumulated substitution to both start and result
        // This ensures row variables are consistently named
        let normalized_start = subst.apply_stack(&start);
        let normalized_result = subst.apply_stack(&result);

        Ok(Effect::with_effects(
            normalized_start,
            normalized_result,
            effects,
        ))
    }

    /// Infer the stack effect of a match expression
    fn infer_match(
        &self,
        arms: &[crate::ast::MatchArm],
        match_span: &Option<crate::ast::Span>,
        current_stack: StackType,
    ) -> Result<(StackType, Subst, Vec<SideEffect>), String> {
        if arms.is_empty() {
            return Err("match expression must have at least one arm".to_string());
        }

        // Pop the matched value from the stack
        let (stack_after_match, _matched_type) =
            self.pop_type(&current_stack, "match expression")?;

        // Track all arm results for unification
        let mut arm_results: Vec<StackType> = Vec::new();
        let mut combined_subst = Subst::empty();
        let mut merged_effects: Vec<SideEffect> = Vec::new();

        // Save aux stack before match arms (Issue #350)
        let aux_before_match = self.current_aux_stack.borrow().clone();
        let mut aux_after_arms: Vec<StackType> = Vec::new();

        for arm in arms {
            // Restore aux stack before each arm (Issue #350)
            *self.current_aux_stack.borrow_mut() = aux_before_match.clone();

            // Get variant name from pattern
            let variant_name = match &arm.pattern {
                crate::ast::Pattern::Variant(name) => name.as_str(),
                crate::ast::Pattern::VariantWithBindings { name, .. } => name.as_str(),
            };

            // Look up variant info
            let (_union_name, variant_info) = self
                .find_variant(variant_name)
                .ok_or_else(|| format!("Unknown variant '{}' in match pattern", variant_name))?;

            // Push fields onto the stack based on pattern type
            let arm_stack = self.push_variant_fields(
                &stack_after_match,
                &arm.pattern,
                variant_info,
                variant_name,
            )?;

            // Type check the arm body directly from the actual stack
            // Don't capture statement types for match arms - only top-level word bodies
            let (arm_result, arm_subst, arm_effects) =
                self.infer_statements_from(&arm.body, &arm_stack, false)?;

            combined_subst = combined_subst.compose(&arm_subst);
            arm_results.push(arm_result);
            aux_after_arms.push(self.current_aux_stack.borrow().clone());

            // Merge effects from this arm
            for effect in arm_effects {
                if !merged_effects.contains(&effect) {
                    merged_effects.push(effect);
                }
            }
        }

        // Verify all arms produce the same aux stack (Issue #350)
        if aux_after_arms.len() > 1 {
            let first_aux = &aux_after_arms[0];
            for (i, arm_aux) in aux_after_arms.iter().enumerate().skip(1) {
                if arm_aux != first_aux {
                    let match_line = match_span.as_ref().map(|s| s.line + 1).unwrap_or(0);
                    return Err(format!(
                        "at line {}: match arms have incompatible aux stack effects:\n\
                         \x20 arm 0 aux: {}\n\
                         \x20 arm {} aux: {}\n\
                         \x20 All match arms must leave the aux stack in the same state.",
                        match_line, first_aux, i, arm_aux
                    ));
                }
            }
        }
        // Set aux to the first arm's result (all are verified equal)
        if let Some(aux) = aux_after_arms.into_iter().next() {
            *self.current_aux_stack.borrow_mut() = aux;
        }

        // Unify all arm results to ensure they're compatible
        let mut final_result = arm_results[0].clone();
        for (i, arm_result) in arm_results.iter().enumerate().skip(1) {
            // Get line info for error reporting
            let match_line = match_span.as_ref().map(|s| s.line + 1).unwrap_or(0);
            let arm0_line = arms[0].span.as_ref().map(|s| s.line + 1).unwrap_or(0);
            let arm_i_line = arms[i].span.as_ref().map(|s| s.line + 1).unwrap_or(0);

            let arm_subst = unify_stacks(&final_result, arm_result).map_err(|e| {
                if match_line > 0 && arm0_line > 0 && arm_i_line > 0 {
                    format!(
                        "at line {}: match arms have incompatible stack effects:\n\
                         \x20 arm 0 (line {}) produces: {}\n\
                         \x20 arm {} (line {}) produces: {}\n\
                         \x20 All match arms must produce the same stack shape.\n\
                         \x20 Error: {}",
                        match_line, arm0_line, final_result, i, arm_i_line, arm_result, e
                    )
                } else {
                    format!(
                        "match arms have incompatible stack effects:\n\
                         \x20 arm 0 produces: {}\n\
                         \x20 arm {} produces: {}\n\
                         \x20 All match arms must produce the same stack shape.\n\
                         \x20 Error: {}",
                        final_result, i, arm_result, e
                    )
                }
            })?;
            combined_subst = combined_subst.compose(&arm_subst);
            final_result = arm_subst.apply_stack(&final_result);
        }

        Ok((final_result, combined_subst, merged_effects))
    }

    /// Push variant fields onto the stack based on the match pattern
    fn push_variant_fields(
        &self,
        stack: &StackType,
        pattern: &crate::ast::Pattern,
        variant_info: &VariantInfo,
        variant_name: &str,
    ) -> Result<StackType, String> {
        let mut arm_stack = stack.clone();
        match pattern {
            crate::ast::Pattern::Variant(_) => {
                // Stack-based: push all fields in declaration order
                for field in &variant_info.fields {
                    arm_stack = arm_stack.push(field.field_type.clone());
                }
            }
            crate::ast::Pattern::VariantWithBindings { bindings, .. } => {
                // Named bindings: validate and push only bound fields
                for binding in bindings {
                    let field = variant_info
                        .fields
                        .iter()
                        .find(|f| &f.name == binding)
                        .ok_or_else(|| {
                            let available: Vec<_> = variant_info
                                .fields
                                .iter()
                                .map(|f| f.name.as_str())
                                .collect();
                            format!(
                                "Unknown field '{}' in pattern for variant '{}'.\n\
                                 Available fields: {}",
                                binding,
                                variant_name,
                                available.join(", ")
                            )
                        })?;
                    arm_stack = arm_stack.push(field.field_type.clone());
                }
            }
        }
        Ok(arm_stack)
    }

    /// Check if a branch ends with a recursive tail call to the current word
    /// or to a mutually recursive word.
    ///
    /// Such branches are "divergent" - they never return to the if/else,
    /// so their stack effect shouldn't constrain the other branch.
    ///
    /// # Detection Capabilities
    ///
    /// - Direct recursion: word calls itself
    /// - Mutual recursion: word calls another word in the same SCC (when call graph is available)
    ///
    /// # Limitations
    ///
    /// This detection does NOT detect:
    /// - Calls to known non-returning functions (panic, exit, infinite loops)
    /// - Nested control flow with tail calls (if ... if ... recurse then then)
    ///
    /// These patterns will still require branch unification. Future enhancements
    /// could track known non-returning functions or support explicit divergence
    /// annotations (similar to Rust's `!` type).
    fn is_divergent_branch(&self, statements: &[Statement]) -> bool {
        let Some((current_word_name, _)) = self.current_word.borrow().as_ref().cloned() else {
            return false;
        };
        let Some(Statement::WordCall { name, .. }) = statements.last() else {
            return false;
        };

        // Direct recursion: word calls itself
        if name == &current_word_name {
            return true;
        }

        // Mutual recursion: word calls another word in the same SCC
        if let Some(ref graph) = self.call_graph
            && graph.are_mutually_recursive(&current_word_name, name)
        {
            return true;
        }

        false
    }

    /// Infer the stack effect of an if/else expression
    fn infer_if(
        &self,
        then_branch: &[Statement],
        else_branch: &Option<Vec<Statement>>,
        if_span: &Option<crate::ast::Span>,
        current_stack: StackType,
    ) -> Result<(StackType, Subst, Vec<SideEffect>), String> {
        // Pop condition (must be Bool)
        let (stack_after_cond, cond_type) = self.pop_type(&current_stack, "if condition")?;

        // Condition must be Bool
        let cond_subst = unify_stacks(
            &StackType::singleton(Type::Bool),
            &StackType::singleton(cond_type),
        )
        .map_err(|e| format!("if condition must be Bool: {}", e))?;

        let stack_after_cond = cond_subst.apply_stack(&stack_after_cond);

        // Check for divergent branches (recursive tail calls)
        let then_diverges = self.is_divergent_branch(then_branch);
        let else_diverges = else_branch
            .as_ref()
            .map(|stmts| self.is_divergent_branch(stmts))
            .unwrap_or(false);

        // Save aux stack before branching (Issue #350)
        let aux_before_branches = self.current_aux_stack.borrow().clone();

        // Infer branches directly from the actual stack
        // Don't capture statement types for if branches - only top-level word bodies
        let (then_result, then_subst, then_effects) =
            self.infer_statements_from(then_branch, &stack_after_cond, false)?;
        let aux_after_then = self.current_aux_stack.borrow().clone();

        // Restore aux stack before checking else branch (Issue #350)
        *self.current_aux_stack.borrow_mut() = aux_before_branches.clone();

        // Infer else branch (or use stack_after_cond if no else)
        let (else_result, else_subst, else_effects) = if let Some(else_stmts) = else_branch {
            self.infer_statements_from(else_stmts, &stack_after_cond, false)?
        } else {
            (stack_after_cond.clone(), Subst::empty(), vec![])
        };
        let aux_after_else = self.current_aux_stack.borrow().clone();

        // Verify aux stacks match between branches (Issue #350)
        // Skip check if one branch diverges (never returns)
        if !then_diverges && !else_diverges && aux_after_then != aux_after_else {
            let if_line = if_span.as_ref().map(|s| s.line + 1).unwrap_or(0);
            return Err(format!(
                "at line {}: if/else branches have incompatible aux stack effects:\n\
                 \x20 then branch aux: {}\n\
                 \x20 else branch aux: {}\n\
                 \x20 Both branches must leave the aux stack in the same state.",
                if_line, aux_after_then, aux_after_else
            ));
        }

        // Set aux to the non-divergent branch's result (or then if neither diverges)
        if then_diverges && !else_diverges {
            *self.current_aux_stack.borrow_mut() = aux_after_else;
        } else {
            *self.current_aux_stack.borrow_mut() = aux_after_then;
        }

        // Merge effects from both branches (if either yields, the whole if yields)
        let mut merged_effects = then_effects;
        for effect in else_effects {
            if !merged_effects.contains(&effect) {
                merged_effects.push(effect);
            }
        }

        // Handle divergent branches: if one branch diverges (never returns),
        // use the other branch's stack type without requiring unification.
        // This supports patterns like:
        //   chan.receive not if drop store-loop then
        // where the then branch recurses and the else branch continues.
        let (result, branch_subst) = if then_diverges && !else_diverges {
            // Then branch diverges, use else branch's type
            (else_result, Subst::empty())
        } else if else_diverges && !then_diverges {
            // Else branch diverges, use then branch's type
            (then_result, Subst::empty())
        } else {
            // Both branches must produce compatible stacks (normal case)
            let if_line = if_span.as_ref().map(|s| s.line + 1).unwrap_or(0);
            let branch_subst = unify_stacks(&then_result, &else_result).map_err(|e| {
                if if_line > 0 {
                    format!(
                        "at line {}: if/else branches have incompatible stack effects:\n\
                         \x20 then branch produces: {}\n\
                         \x20 else branch produces: {}\n\
                         \x20 Both branches of an if/else must produce the same stack shape.\n\
                         \x20 Hint: Make sure both branches push/pop the same number of values.\n\
                         \x20 Error: {}",
                        if_line, then_result, else_result, e
                    )
                } else {
                    format!(
                        "if/else branches have incompatible stack effects:\n\
                         \x20 then branch produces: {}\n\
                         \x20 else branch produces: {}\n\
                         \x20 Both branches of an if/else must produce the same stack shape.\n\
                         \x20 Hint: Make sure both branches push/pop the same number of values.\n\
                         \x20 Error: {}",
                        then_result, else_result, e
                    )
                }
            })?;
            (branch_subst.apply_stack(&then_result), branch_subst)
        };

        // Propagate all substitutions
        let total_subst = cond_subst
            .compose(&then_subst)
            .compose(&else_subst)
            .compose(&branch_subst);
        Ok((result, total_subst, merged_effects))
    }

    /// Infer the stack effect of a quotation
    /// Quotations capture effects in their type - they don't propagate effects to the outer scope
    fn infer_quotation(
        &self,
        id: usize,
        body: &[Statement],
        current_stack: StackType,
    ) -> Result<(StackType, Subst, Vec<SideEffect>), String> {
        // Save and clear expected type so nested quotations don't inherit it.
        // The expected type applies only to THIS quotation, not inner ones.
        let expected_for_this_quotation = self.expected_quotation_type.borrow().clone();
        *self.expected_quotation_type.borrow_mut() = None;

        // Save enclosing aux stack and enter quotation scope (Issue #350, #393).
        // Quotations are compiled as separate LLVM functions; each gets its own
        // aux slot table. The save/restore here means the enclosing word's aux
        // state is undisturbed by the quotation, and the quotation's aux usage
        // is tracked independently in `quotation_aux_depths` (Issue #393).
        let saved_aux = self.current_aux_stack.borrow().clone();
        *self.current_aux_stack.borrow_mut() = StackType::Empty;
        self.quotation_id_stack.borrow_mut().push(id);

        // Run the body inference and balance check inside an immediately-invoked
        // closure so we can restore scope state on every exit path — including
        // errors. Without this, an error in body inference or the balance check
        // would leave the typechecker with a corrupt scope stack and a polluted
        // aux stack, which matters for callers that inspect errors and continue.
        let body_result: Result<Effect, String> = (|| {
            // Infer the effect of the quotation body.
            //
            // If we have an expected quotation type from a combinator's signature
            // (e.g., list.fold expects [..b Acc T -- ..b Acc]), seed the body
            // inference with that input stack. Without this, the body inference
            // starts from a polymorphic row variable, and operations like >aux
            // can't pop because they don't know the type. Issue #393.
            let body_effect = if let Some(expected) = &expected_for_this_quotation {
                let expected_effect = match expected {
                    Type::Quotation(eff) => Some((**eff).clone()),
                    Type::Closure { effect, .. } => Some((**effect).clone()),
                    _ => None,
                };
                if let Some(eff) = expected_effect {
                    // Freshen to avoid row-variable name clashes with the
                    // enclosing scope.
                    let fresh = self.freshen_effect(&eff);
                    let (result, subst, effects) =
                        self.infer_statements_from(body, &fresh.inputs, false)?;
                    let normalized_start = subst.apply_stack(&fresh.inputs);
                    let normalized_result = subst.apply_stack(&result);
                    Effect::with_effects(normalized_start, normalized_result, effects)
                } else {
                    self.infer_statements(body)?
                }
            } else {
                self.infer_statements(body)?
            };

            // Verify quotation's aux stack is balanced (Issue #350).
            // Lexical scoping: every >aux inside the quotation must have a
            // matching aux> inside the same quotation.
            let quot_aux = self.current_aux_stack.borrow().clone();
            if quot_aux != StackType::Empty {
                return Err(format!(
                    "Quotation has unbalanced aux stack.\n\
                     Remaining aux stack: {}\n\
                     Every >aux must be matched by a corresponding aux> within the quotation.",
                    quot_aux
                ));
            }

            Ok(body_effect)
        })();

        // Always restore scope state, regardless of whether the body inference
        // succeeded or failed.
        *self.current_aux_stack.borrow_mut() = saved_aux;
        self.quotation_id_stack.borrow_mut().pop();
        *self.expected_quotation_type.borrow_mut() = expected_for_this_quotation.clone();

        let body_effect = body_result?;

        // Perform capture analysis
        let quot_type = self.analyze_captures(&body_effect, &current_stack)?;

        // If this is a closure, we need to pop the captured values from the stack
        // and correct the capture types from the caller's actual stack.
        let result_stack = match &quot_type {
            Type::Quotation(_) => {
                // Stateless - no captures. Record in type map for codegen.
                self.quotation_types
                    .borrow_mut()
                    .insert(id, quot_type.clone());
                current_stack.push(quot_type)
            }
            Type::Closure {
                captures, effect, ..
            } => {
                // Pop captured values from the caller's stack.
                // The capture COUNT comes from analyze_captures (based on
                // body vs expected input comparison), but the capture TYPES
                // come from the caller's stack — not from the body's inference.
                //
                // We intentionally do NOT call unify_types on the popped types.
                // The body's inference may have constrained a type variable to
                // Int/Float via its operations (e.g., i.+), even when the actual
                // stack value is a Variant. unify_types(Var("V$nn"), Int) would
                // succeed and propagate the wrong type to codegen, which would
                // then emit env_get_int for a Variant value — a runtime crash.
                // Using the caller's actual types directly ensures codegen emits
                // the correct getter for the runtime Value type.
                let mut stack = current_stack.clone();
                let mut actual_captures: Vec<Type> = Vec::new();
                for _ in (0..captures.len()).rev() {
                    let (new_stack, actual_type) = self.pop_type(&stack, "closure capture")?;
                    actual_captures.push(actual_type);
                    stack = new_stack;
                }
                // actual_captures is in pop order (top-down), reverse to
                // get bottom-to-top (matching calculate_captures convention)
                actual_captures.reverse();

                // Rebuild the closure type with the actual capture types
                let corrected_quot_type = Type::Closure {
                    effect: effect.clone(),
                    captures: actual_captures,
                };

                // Update the type map so codegen sees the corrected types
                self.quotation_types
                    .borrow_mut()
                    .insert(id, corrected_quot_type.clone());

                stack.push(corrected_quot_type)
            }
            _ => unreachable!("analyze_captures only returns Quotation or Closure"),
        };

        // Quotations don't propagate effects - they capture them in the quotation type
        // The effect annotation on the quotation type (e.g., [ ..a -- ..b | Yield Int ])
        // indicates what effects the quotation may produce when called
        Ok((result_stack, Subst::empty(), vec![]))
    }

    /// Infer the stack effect of a word call
    fn infer_word_call(
        &self,
        name: &str,
        span: &Option<crate::ast::Span>,
        current_stack: StackType,
    ) -> Result<(StackType, Subst, Vec<SideEffect>), String> {
        // Arithmetic sugar resolution: resolve +, -, *, / etc. to concrete ops
        // based on the types currently on the stack.
        let is_sugar = matches!(
            name,
            "+" | "-" | "*" | "/" | "%" | "=" | "<" | ">" | "<=" | ">=" | "<>"
        );
        if is_sugar {
            if let Some(resolved) = self.resolve_arithmetic_sugar(name, &current_stack) {
                // Record the resolution for codegen, keyed by source location (line, column)
                if let Some(s) = span {
                    self.resolved_sugar
                        .borrow_mut()
                        .insert((s.line, s.column), resolved.clone());
                }
                // Proceed as if the user wrote the resolved name
                return self.infer_word_call(&resolved, span, current_stack);
            }
            // Sugar op but types don't match — give a helpful error
            let line_prefix = self.line_prefix();
            let (top_desc, second_desc) = {
                let top = current_stack.clone().pop().map(|(_, t)| format!("{}", t));
                let second = current_stack
                    .clone()
                    .pop()
                    .and_then(|(r, _)| r.pop().map(|(_, t)| format!("{}", t)));
                (
                    top.unwrap_or_else(|| "empty".to_string()),
                    second.unwrap_or_else(|| "empty".to_string()),
                )
            };
            let (type_options, suggestion) = match name {
                "+" => (
                    "Int+Int, Float+Float, or String+String",
                    "Use `i.+`, `f.+`, or `string.concat`.",
                ),
                "=" => (
                    "Int+Int, Float+Float, or String+String (equality)",
                    "Use `i.=`, `f.=`, or `string.equal?`.",
                ),
                "%" => (
                    "Int+Int only — float modulo is not supported",
                    "Use `i.%` for integer modulo.",
                ),
                _ => (
                    "Int+Int or Float+Float",
                    "Use the `i.` or `f.` prefixed variant.",
                ),
            };
            return Err(format!(
                "{}`{}` requires matching types ({}), got ({}, {}). {}",
                line_prefix, name, type_options, second_desc, top_desc, suggestion,
            ));
        }

        // Special handling for aux stack operations (Issue #350)
        if name == ">aux" {
            return self.infer_to_aux(span, current_stack);
        }
        if name == "aux>" {
            return self.infer_from_aux(span, current_stack);
        }

        // Special handling for `call`: extract and apply the quotation's actual effect
        // This ensures stack pollution through quotations is caught (Issue #228)
        if name == "call" {
            return self.infer_call(span, current_stack);
        }

        // Special handling for dataflow combinators
        if name == "dip" {
            return self.infer_dip(span, current_stack);
        }
        if name == "keep" {
            return self.infer_keep(span, current_stack);
        }
        if name == "bi" {
            return self.infer_bi(span, current_stack);
        }

        // Look up word's effect
        let effect = self
            .lookup_word_effect(name)
            .ok_or_else(|| format!("Unknown word: '{}'", name))?;

        // Freshen the effect to avoid variable name clashes
        let fresh_effect = self.freshen_effect(&effect);

        // Special handling for strand.spawn: auto-convert Quotation to Closure if needed
        let adjusted_stack = if name == "strand.spawn" {
            self.adjust_stack_for_spawn(current_stack, &fresh_effect)?
        } else {
            current_stack
        };

        // Apply the freshened effect to current stack
        let (result_stack, subst) = self.apply_effect(&fresh_effect, adjusted_stack, name, span)?;

        // Propagate side effects from the called word
        // Note: strand.weave "handles" Yield effects (consumes them from the quotation)
        // strand.spawn requires pure quotations (checked separately)
        let propagated_effects = fresh_effect.effects.clone();

        Ok((result_stack, subst, propagated_effects))
    }

    /// Handle >aux: pop from main stack, push onto scope-local aux stack
    /// (Issue #350, Issue #393).
    ///
    /// In word-body scope, depth is tracked per word in `aux_max_depths`.
    /// In quotation-body scope, depth is tracked per quotation ID in
    /// `quotation_aux_depths`. Each quotation gets its own slot table at
    /// codegen time.
    fn infer_to_aux(
        &self,
        _span: &Option<crate::ast::Span>,
        current_stack: StackType,
    ) -> Result<(StackType, Subst, Vec<SideEffect>), String> {
        let (rest, top_type) = self.pop_type(&current_stack, ">aux")?;

        // Push onto aux stack
        let mut aux = self.current_aux_stack.borrow_mut();
        *aux = aux.clone().push(top_type);

        // Track max depth for codegen alloca sizing.
        // If we're inside a quotation, key the depth by quotation ID.
        // Otherwise, key by the enclosing word name.
        let depth = Self::stack_depth(&aux);
        let quot_stack = self.quotation_id_stack.borrow();
        if let Some(&quot_id) = quot_stack.last() {
            let mut depths = self.quotation_aux_depths.borrow_mut();
            let entry = depths.entry(quot_id).or_insert(0);
            if depth > *entry {
                *entry = depth;
            }
        } else if let Some((word_name, _)) = self.current_word.borrow().as_ref() {
            let mut depths = self.aux_max_depths.borrow_mut();
            let entry = depths.entry(word_name.clone()).or_insert(0);
            if depth > *entry {
                *entry = depth;
            }
        }

        Ok((rest, Subst::empty(), vec![]))
    }

    /// Handle aux>: pop from aux stack, push onto main stack (Issue #350, #393).
    fn infer_from_aux(
        &self,
        _span: &Option<crate::ast::Span>,
        current_stack: StackType,
    ) -> Result<(StackType, Subst, Vec<SideEffect>), String> {
        let mut aux = self.current_aux_stack.borrow_mut();
        match aux.clone().pop() {
            Some((rest, top_type)) => {
                *aux = rest;
                Ok((current_stack.push(top_type), Subst::empty(), vec![]))
            }
            None => {
                let line_info = self.line_prefix();
                Err(format!(
                    "{}aux>: aux stack is empty. Every aux> must be paired with a preceding >aux.",
                    line_info
                ))
            }
        }
    }

    /// Special handling for `call` to properly propagate quotation effects (Issue #228)
    ///
    /// The generic `call` signature `( ..a Q -- ..b )` has independent row variables,
    /// which doesn't constrain the output based on the quotation's actual effect.
    /// This function extracts the quotation's effect and applies it properly.
    fn infer_call(
        &self,
        span: &Option<crate::ast::Span>,
        current_stack: StackType,
    ) -> Result<(StackType, Subst, Vec<SideEffect>), String> {
        // Pop the quotation from the stack
        let line_prefix = self.line_prefix();
        let (remaining_stack, quot_type) = current_stack.clone().pop().ok_or_else(|| {
            format!(
                "{}call: stack underflow - expected quotation on stack",
                line_prefix
            )
        })?;

        // Extract the quotation's effect
        let quot_effect = match &quot_type {
            Type::Quotation(effect) => (**effect).clone(),
            Type::Closure { effect, .. } => (**effect).clone(),
            Type::Var(_) => {
                // Type variable - fall back to polymorphic behavior
                // This happens when the quotation type isn't known yet
                let effect = self
                    .lookup_word_effect("call")
                    .ok_or_else(|| "Unknown word: 'call'".to_string())?;
                let fresh_effect = self.freshen_effect(&effect);
                let (result_stack, subst) =
                    self.apply_effect(&fresh_effect, current_stack, "call", span)?;
                return Ok((result_stack, subst, vec![]));
            }
            _ => {
                return Err(format!(
                    "call: expected quotation or closure on stack, got {}",
                    quot_type
                ));
            }
        };

        // Check for Yield effects - quotations with Yield must use strand.weave
        if quot_effect.has_yield() {
            return Err("Cannot call quotation with Yield effect directly.\n\
                 Quotations that yield values must be wrapped with `strand.weave`.\n\
                 Example: `[ yielding-code ] strand.weave` instead of `[ yielding-code ] call`"
                .to_string());
        }

        // Freshen the quotation's effect to avoid variable clashes
        let fresh_effect = self.freshen_effect(&quot_effect);

        // Apply the quotation's effect to the remaining stack
        let (result_stack, subst) =
            self.apply_effect(&fresh_effect, remaining_stack, "call", span)?;

        // Propagate side effects from the quotation
        let propagated_effects = fresh_effect.effects.clone();

        Ok((result_stack, subst, propagated_effects))
    }

    /// Resolve arithmetic sugar operators to concrete operations based on
    /// the types on the stack. Returns `None` if the name is not a sugar op.
    fn resolve_arithmetic_sugar(&self, name: &str, stack: &StackType) -> Option<String> {
        // Only handle known sugar operators
        let is_binary = matches!(
            name,
            "+" | "-" | "*" | "/" | "%" | "=" | "<" | ">" | "<=" | ">=" | "<>"
        );
        if !is_binary {
            return None;
        }

        // Peek at the top two types on the stack
        let (rest, top) = stack.clone().pop()?;
        let (_, second) = rest.pop()?;

        match (name, &second, &top) {
            // Int × Int operations
            ("+", Type::Int, Type::Int) => Some("i.+".to_string()),
            ("-", Type::Int, Type::Int) => Some("i.-".to_string()),
            ("*", Type::Int, Type::Int) => Some("i.*".to_string()),
            ("/", Type::Int, Type::Int) => Some("i./".to_string()),
            ("%", Type::Int, Type::Int) => Some("i.%".to_string()),
            ("=", Type::Int, Type::Int) => Some("i.=".to_string()),
            ("<", Type::Int, Type::Int) => Some("i.<".to_string()),
            (">", Type::Int, Type::Int) => Some("i.>".to_string()),
            ("<=", Type::Int, Type::Int) => Some("i.<=".to_string()),
            (">=", Type::Int, Type::Int) => Some("i.>=".to_string()),
            ("<>", Type::Int, Type::Int) => Some("i.<>".to_string()),

            // Float × Float operations
            ("+", Type::Float, Type::Float) => Some("f.+".to_string()),
            ("-", Type::Float, Type::Float) => Some("f.-".to_string()),
            ("*", Type::Float, Type::Float) => Some("f.*".to_string()),
            ("/", Type::Float, Type::Float) => Some("f./".to_string()),
            ("=", Type::Float, Type::Float) => Some("f.=".to_string()),
            ("<", Type::Float, Type::Float) => Some("f.<".to_string()),
            (">", Type::Float, Type::Float) => Some("f.>".to_string()),
            ("<=", Type::Float, Type::Float) => Some("f.<=".to_string()),
            (">=", Type::Float, Type::Float) => Some("f.>=".to_string()),
            ("<>", Type::Float, Type::Float) => Some("f.<>".to_string()),

            // String operations (only + for concat, = for equality)
            ("+", Type::String, Type::String) => Some("string.concat".to_string()),
            ("=", Type::String, Type::String) => Some("string.equal?".to_string()),

            // No match — not a sugar op for these types (will fall through
            // to normal lookup, which will fail with "Unknown word: '+'" —
            // giving the user a clear error that they need explicit types)
            _ => None,
        }
    }

    /// Infer the stack effect of `dip`: ( ..a x quot -- ..b x )
    ///
    /// Hide the value below the quotation, run the quotation on the rest
    /// of the stack, then restore the hidden value on top.
    fn infer_dip(
        &self,
        span: &Option<crate::ast::Span>,
        current_stack: StackType,
    ) -> Result<(StackType, Subst, Vec<SideEffect>), String> {
        let line_prefix = self.line_prefix();

        // Pop the quotation
        let (stack_after_quot, quot_type) = current_stack.clone().pop().ok_or_else(|| {
            format!(
                "{}dip: stack underflow - expected quotation on stack",
                line_prefix
            )
        })?;

        // Extract the quotation's effect
        let quot_effect = match &quot_type {
            Type::Quotation(effect) => (**effect).clone(),
            Type::Closure { effect, .. } => (**effect).clone(),
            Type::Var(_) => {
                // Unknown quotation type — fall back to generic builtin signature
                let effect = self
                    .lookup_word_effect("dip")
                    .ok_or_else(|| "Unknown word: 'dip'".to_string())?;
                let fresh_effect = self.freshen_effect(&effect);
                let (result_stack, subst) =
                    self.apply_effect(&fresh_effect, current_stack, "dip", span)?;
                return Ok((result_stack, subst, vec![]));
            }
            _ => {
                return Err(format!(
                    "{}dip: expected quotation or closure on top of stack, got {}",
                    line_prefix, quot_type
                ));
            }
        };

        if quot_effect.has_yield() {
            return Err("dip: quotation must not have Yield effects.\n\
                 Use strand.weave for quotations that yield."
                .to_string());
        }

        // Pop the preserved value (below the quotation)
        let (rest_stack, preserved_type) = stack_after_quot.clone().pop().ok_or_else(|| {
            format!(
                "{}dip: stack underflow - expected a value below the quotation",
                line_prefix
            )
        })?;

        // Freshen and apply the quotation's effect to the stack below the preserved value
        let fresh_effect = self.freshen_effect(&quot_effect);
        let (result_stack, subst) =
            self.apply_effect(&fresh_effect, rest_stack, "dip (quotation)", span)?;

        // Push the preserved value back on top, applying substitution in case
        // preserved_type contains type variables resolved during unification
        let resolved_preserved = subst.apply_type(&preserved_type);
        let result_stack = result_stack.push(resolved_preserved);

        let propagated_effects = fresh_effect.effects.clone();
        Ok((result_stack, subst, propagated_effects))
    }

    /// Infer the stack effect of `keep`: ( ..a x quot -- ..b x )
    ///
    /// Run the quotation on the value (quotation receives x), then
    /// restore the original value on top. Like `over >aux call aux>`.
    fn infer_keep(
        &self,
        span: &Option<crate::ast::Span>,
        current_stack: StackType,
    ) -> Result<(StackType, Subst, Vec<SideEffect>), String> {
        let line_prefix = self.line_prefix();

        // Pop the quotation
        let (stack_after_quot, quot_type) = current_stack.clone().pop().ok_or_else(|| {
            format!(
                "{}keep: stack underflow - expected quotation on stack",
                line_prefix
            )
        })?;

        // Extract the quotation's effect
        let quot_effect = match &quot_type {
            Type::Quotation(effect) => (**effect).clone(),
            Type::Closure { effect, .. } => (**effect).clone(),
            Type::Var(_) => {
                let effect = self
                    .lookup_word_effect("keep")
                    .ok_or_else(|| "Unknown word: 'keep'".to_string())?;
                let fresh_effect = self.freshen_effect(&effect);
                let (result_stack, subst) =
                    self.apply_effect(&fresh_effect, current_stack, "keep", span)?;
                return Ok((result_stack, subst, vec![]));
            }
            _ => {
                return Err(format!(
                    "{}keep: expected quotation or closure on top of stack, got {}",
                    line_prefix, quot_type
                ));
            }
        };

        if quot_effect.has_yield() {
            return Err("keep: quotation must not have Yield effects.\n\
                 Use strand.weave for quotations that yield."
                .to_string());
        }

        // Peek at the preserved value type (it stays, we just need its type)
        let (_rest_stack, preserved_type) = stack_after_quot.clone().pop().ok_or_else(|| {
            format!(
                "{}keep: stack underflow - expected a value below the quotation",
                line_prefix
            )
        })?;

        // The quotation receives x on the stack (stack_after_quot still has x on top).
        // Apply the quotation's effect to the stack INCLUDING x.
        let fresh_effect = self.freshen_effect(&quot_effect);
        let (result_stack, subst) =
            self.apply_effect(&fresh_effect, stack_after_quot, "keep (quotation)", span)?;

        // Push the preserved value back on top, applying substitution in case
        // preserved_type contains type variables resolved during unification
        let resolved_preserved = subst.apply_type(&preserved_type);
        let result_stack = result_stack.push(resolved_preserved);

        let propagated_effects = fresh_effect.effects.clone();
        Ok((result_stack, subst, propagated_effects))
    }

    /// Infer the stack effect of `bi`: ( ..a x quot1 quot2 -- ..c )
    ///
    /// Apply two quotations to the same value. First quotation receives x,
    /// then second quotation receives x on top of the first quotation's results.
    fn infer_bi(
        &self,
        span: &Option<crate::ast::Span>,
        current_stack: StackType,
    ) -> Result<(StackType, Subst, Vec<SideEffect>), String> {
        let line_prefix = self.line_prefix();

        // Pop quot2 (top)
        let (stack1, quot2_type) = current_stack.clone().pop().ok_or_else(|| {
            format!(
                "{}bi: stack underflow - expected second quotation on stack",
                line_prefix
            )
        })?;

        // Pop quot1
        let (stack2, quot1_type) = stack1.clone().pop().ok_or_else(|| {
            format!(
                "{}bi: stack underflow - expected first quotation on stack",
                line_prefix
            )
        })?;

        // Extract both quotation effects
        let quot1_effect = match &quot1_type {
            Type::Quotation(effect) => (**effect).clone(),
            Type::Closure { effect, .. } => (**effect).clone(),
            Type::Var(_) => {
                let effect = self
                    .lookup_word_effect("bi")
                    .ok_or_else(|| "Unknown word: 'bi'".to_string())?;
                let fresh_effect = self.freshen_effect(&effect);
                let (result_stack, subst) =
                    self.apply_effect(&fresh_effect, current_stack, "bi", span)?;
                return Ok((result_stack, subst, vec![]));
            }
            _ => {
                return Err(format!(
                    "{}bi: expected quotation or closure as first quotation, got {}",
                    line_prefix, quot1_type
                ));
            }
        };

        let quot2_effect = match &quot2_type {
            Type::Quotation(effect) => (**effect).clone(),
            Type::Closure { effect, .. } => (**effect).clone(),
            Type::Var(_) => {
                let effect = self
                    .lookup_word_effect("bi")
                    .ok_or_else(|| "Unknown word: 'bi'".to_string())?;
                let fresh_effect = self.freshen_effect(&effect);
                let (result_stack, subst) =
                    self.apply_effect(&fresh_effect, current_stack, "bi", span)?;
                return Ok((result_stack, subst, vec![]));
            }
            _ => {
                return Err(format!(
                    "{}bi: expected quotation or closure as second quotation, got {}",
                    line_prefix, quot2_type
                ));
            }
        };

        if quot1_effect.has_yield() || quot2_effect.has_yield() {
            return Err("bi: quotations must not have Yield effects.\n\
                 Use strand.weave for quotations that yield."
                .to_string());
        }

        // stack2 has x on top (the value both quotations operate on)
        // Peek at x's type for the second application
        let (_rest, preserved_type) = stack2.clone().pop().ok_or_else(|| {
            format!(
                "{}bi: stack underflow - expected a value below the quotations",
                line_prefix
            )
        })?;

        // Apply quot1 to stack including x
        let fresh_effect1 = self.freshen_effect(&quot1_effect);
        let (after_quot1, subst1) =
            self.apply_effect(&fresh_effect1, stack2, "bi (first quotation)", span)?;

        // Push x again for quot2, applying subst1 in case preserved_type
        // contains type variables that were resolved during quot1's unification
        let resolved_preserved = subst1.apply_type(&preserved_type);
        let with_x = after_quot1.push(resolved_preserved);

        // Apply quot2
        let fresh_effect2 = self.freshen_effect(&quot2_effect);
        let (result_stack, subst2) =
            self.apply_effect(&fresh_effect2, with_x, "bi (second quotation)", span)?;

        let subst = subst1.compose(&subst2);

        let mut effects = fresh_effect1.effects.clone();
        for e in fresh_effect2.effects.clone() {
            if !effects.contains(&e) {
                effects.push(e);
            }
        }

        Ok((result_stack, subst, effects))
    }

    /// Infer the resulting stack type after a statement
    /// Takes current stack, returns (new stack, substitution, side effects) after statement
    fn infer_statement(
        &self,
        statement: &Statement,
        current_stack: StackType,
    ) -> Result<(StackType, Subst, Vec<SideEffect>), String> {
        match statement {
            Statement::IntLiteral(_) => Ok((current_stack.push(Type::Int), Subst::empty(), vec![])),
            Statement::BoolLiteral(_) => {
                Ok((current_stack.push(Type::Bool), Subst::empty(), vec![]))
            }
            Statement::StringLiteral(_) => {
                Ok((current_stack.push(Type::String), Subst::empty(), vec![]))
            }
            Statement::FloatLiteral(_) => {
                Ok((current_stack.push(Type::Float), Subst::empty(), vec![]))
            }
            Statement::Symbol(_) => Ok((current_stack.push(Type::Symbol), Subst::empty(), vec![])),
            Statement::Match { arms, span } => self.infer_match(arms, span, current_stack),
            Statement::WordCall { name, span } => self.infer_word_call(name, span, current_stack),
            Statement::If {
                then_branch,
                else_branch,
                span,
            } => self.infer_if(then_branch, else_branch, span, current_stack),
            Statement::Quotation { id, body, .. } => self.infer_quotation(*id, body, current_stack),
        }
    }

    /// Look up the effect of a word (built-in or user-defined)
    fn lookup_word_effect(&self, name: &str) -> Option<Effect> {
        // First check built-ins
        if let Some(effect) = builtin_signature(name) {
            return Some(effect);
        }

        // Then check user-defined words
        self.env.get(name).cloned()
    }

    /// Apply an effect to a stack
    /// Effect: (inputs -- outputs)
    /// Current stack must match inputs, result is outputs
    /// Returns (result_stack, substitution)
    fn apply_effect(
        &self,
        effect: &Effect,
        current_stack: StackType,
        operation: &str,
        span: &Option<crate::ast::Span>,
    ) -> Result<(StackType, Subst), String> {
        // Check for stack underflow: if the effect needs more concrete values than
        // the current stack provides, and the stack has a "rigid" row variable at its base,
        // this would be unsound (the row var could be Empty at runtime).
        // Bug #169: "phantom stack entries"
        //
        // We only check for "rigid" row variables (named "rest" from declared effects).
        // Row variables named "input" are from inference and CAN grow to discover requirements.
        let effect_concrete = Self::count_concrete_types(&effect.inputs);
        let stack_concrete = Self::count_concrete_types(&current_stack);

        if let Some(row_var_name) = Self::get_row_var_base(&current_stack) {
            // Only check "rigid" row variables (from declared effects, not inference).
            //
            // Row variable naming convention (established in parser.rs:build_stack_type):
            // - "rest": Created by the parser for declared stack effects. When a word declares
            //   `( String Int -- String )`, the parser creates `( ..rest String Int -- ..rest String )`.
            //   This "rest" is rigid because the caller guarantees exactly these concrete types.
            // - "rest$N": Freshened versions created during type checking when calling other words.
            //   These represent the callee's stack context and can grow during unification.
            // - "input": Created for words without declared effects during inference.
            //   These are flexible and grow to discover the word's actual requirements.
            //
            // Only the original "rest" (exact match) should trigger underflow checking.
            let is_rigid = row_var_name == "rest";

            if is_rigid && effect_concrete > stack_concrete {
                let word_name = self
                    .current_word
                    .borrow()
                    .as_ref()
                    .map(|(n, _)| n.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                return Err(format!(
                    "{}In '{}': {}: stack underflow - requires {} value(s), only {} provided",
                    self.line_prefix(),
                    word_name,
                    operation,
                    effect_concrete,
                    stack_concrete
                ));
            }
        }

        // Unify current stack with effect's input
        let subst = unify_stacks(&effect.inputs, &current_stack).map_err(|e| {
            let line_info = span
                .as_ref()
                .map(|s| format_line_prefix(s.line))
                .unwrap_or_default();
            format!(
                "{}{}: stack type mismatch. Expected {}, got {}: {}",
                line_info, operation, effect.inputs, current_stack, e
            )
        })?;

        // Apply substitution to output
        let result_stack = subst.apply_stack(&effect.outputs);

        Ok((result_stack, subst))
    }

    /// Count the number of concrete (non-row-variable) types in a stack
    fn count_concrete_types(stack: &StackType) -> usize {
        let mut count = 0;
        let mut current = stack;
        while let StackType::Cons { rest, top: _ } = current {
            count += 1;
            current = rest;
        }
        count
    }

    /// Get the row variable name at the base of a stack, if any
    fn get_row_var_base(stack: &StackType) -> Option<String> {
        let mut current = stack;
        while let StackType::Cons { rest, top: _ } = current {
            current = rest;
        }
        match current {
            StackType::RowVar(name) => Some(name.clone()),
            _ => None,
        }
    }

    /// Adjust stack for strand.spawn operation by converting Quotation to Closure if needed
    ///
    /// strand.spawn expects Quotation(Empty -- Empty), but if we have Quotation(T... -- U...)
    /// with non-empty inputs, we auto-convert it to a Closure that captures those inputs.
    fn adjust_stack_for_spawn(
        &self,
        current_stack: StackType,
        spawn_effect: &Effect,
    ) -> Result<StackType, String> {
        // strand.spawn expects: ( ..a Quotation(Empty -- Empty) -- ..a Int )
        // Extract the expected quotation type from strand.spawn's effect
        let expected_quot_type = match &spawn_effect.inputs {
            StackType::Cons { top, rest: _ } => {
                if !matches!(top, Type::Quotation(_)) {
                    return Ok(current_stack); // Not a quotation, don't adjust
                }
                top
            }
            _ => return Ok(current_stack),
        };

        // Check what's actually on the stack
        let (rest_stack, actual_type) = match &current_stack {
            StackType::Cons { rest, top } => (rest.as_ref().clone(), top),
            _ => return Ok(current_stack), // Empty stack, nothing to adjust
        };

        // If top of stack is a Quotation with non-empty inputs, convert to Closure
        if let Type::Quotation(actual_effect) = actual_type {
            // Check if quotation needs inputs
            if !matches!(actual_effect.inputs, StackType::Empty) {
                // Extract expected effect from spawn's signature
                let expected_effect = match expected_quot_type {
                    Type::Quotation(eff) => eff.as_ref(),
                    _ => return Ok(current_stack),
                };

                // Calculate what needs to be captured
                let captures = calculate_captures(actual_effect, expected_effect)?;

                // Create a Closure type
                let closure_type = Type::Closure {
                    effect: Box::new(expected_effect.clone()),
                    captures: captures.clone(),
                };

                // Pop the captured values from the stack
                // The values to capture are BELOW the quotation on the stack
                let mut adjusted_stack = rest_stack;
                for _ in &captures {
                    adjusted_stack = match adjusted_stack {
                        StackType::Cons { rest, .. } => rest.as_ref().clone(),
                        _ => {
                            return Err(format!(
                                "strand.spawn: not enough values on stack to capture. Need {} values",
                                captures.len()
                            ));
                        }
                    };
                }

                // Push the Closure onto the adjusted stack
                return Ok(adjusted_stack.push(closure_type));
            }
        }

        Ok(current_stack)
    }

    /// Analyze quotation captures
    ///
    /// Determines whether a quotation should be stateless (Type::Quotation)
    /// or a closure (Type::Closure) based on the expected type from the word signature.
    ///
    /// Type-driven inference with automatic closure creation:
    ///   - If expected type is Closure[effect], calculate what to capture
    ///   - If expected type is Quotation[effect]:
    ///     - If body needs more inputs than expected effect, auto-create Closure
    ///     - Otherwise return stateless Quotation
    ///   - If no expected type, default to stateless (conservative)
    ///
    /// Example 1 (auto-create closure):
    ///   Expected: Quotation[-- ]          [spawn expects ( -- )]
    ///   Body: [ handle-connection ]       [needs ( Int -- )]
    ///   Body effect: ( Int -- )           [needs 1 Int]
    ///   Expected effect: ( -- )           [provides 0 inputs]
    ///   Result: Closure { effect: ( -- ), captures: [Int] }
    ///
    /// Example 2 (explicit closure):
    ///   Signature: ( Int -- Closure[Int -- Int] )
    ///   Body: [ add ]
    ///   Body effect: ( Int Int -- Int )  [add needs 2 Ints]
    ///   Expected effect: [Int -- Int]    [call site provides 1 Int]
    ///   Result: Closure { effect: [Int -- Int], captures: [Int] }
    fn analyze_captures(
        &self,
        body_effect: &Effect,
        _current_stack: &StackType,
    ) -> Result<Type, String> {
        // Check if there's an expected type from the word signature
        let expected = self.expected_quotation_type.borrow().clone();

        match expected {
            Some(Type::Closure { effect, .. }) => {
                // User declared closure type - calculate captures
                let captures = calculate_captures(body_effect, &effect)?;
                Ok(Type::Closure { effect, captures })
            }
            Some(Type::Quotation(expected_effect)) => {
                // Check if we need to auto-create a closure by comparing the
                // body's concrete input count against what the combinator provides.
                let body_inputs = extract_concrete_types(&body_effect.inputs);
                let expected_inputs = extract_concrete_types(&expected_effect.inputs);

                // Auto-capture triggers when the body needs more concrete inputs
                // than the expected provides. Three branches:
                // (a) Expected is empty (strand.spawn): body needs any inputs → capture all.
                // (b) Expected has concrete inputs (list.fold): body has MORE → capture excess.
                // (c) Expected has ONLY a row variable and no concrete inputs
                //     (strand.weave): don't capture, fall through to unification.
                let expected_is_empty = matches!(expected_effect.inputs, StackType::Empty);
                let should_capture = if expected_is_empty {
                    !body_inputs.is_empty()
                } else if !expected_inputs.is_empty() {
                    body_inputs.len() > expected_inputs.len()
                } else {
                    false // row-variable-only expected — don't capture, unify instead
                };

                if should_capture {
                    // Body needs more inputs than the combinator provides.
                    // The excess (bottommost) become captures; the topmost must
                    // align with what the combinator provides.
                    //
                    // Example: list.fold expects ( ..b Acc T -- ..b Acc ).
                    // Body inferred as ( ..b X Acc T -- ..b Acc ).
                    // expected_inputs = [Acc, T], body_inputs = [X, Acc, T].
                    // Captures = [X]. Topmost 2 of body must match expected's 2.
                    //
                    // Issue #395: this extends the empty-input auto-capture
                    // (used by strand.spawn) to the non-empty case.
                    let captures = calculate_captures(body_effect, &expected_effect)?;
                    Ok(Type::Closure {
                        effect: expected_effect,
                        captures,
                    })
                } else {
                    // Body has same or fewer inputs — standard unification path.
                    // This catches:
                    // - Stack pollution: body pushes values when expected is stack-neutral
                    // - Stack underflow: body consumes values when expected is stack-neutral
                    // - Wrong return type: body returns Int when Bool expected
                    let body_quot = Type::Quotation(Box::new(body_effect.clone()));
                    let expected_quot = Type::Quotation(expected_effect.clone());
                    unify_types(&body_quot, &expected_quot).map_err(|e| {
                        format!(
                            "quotation effect mismatch: expected {}, got {}: {}",
                            expected_effect, body_effect, e
                        )
                    })?;

                    // Body is compatible with expected effect - stateless quotation
                    Ok(Type::Quotation(expected_effect))
                }
            }
            _ => {
                // No expected type - conservative default: stateless quotation
                Ok(Type::Quotation(Box::new(body_effect.clone())))
            }
        }
    }

    /// Check if an inferred effect matches any of the declared effects
    /// Effects match by kind (e.g., Yield matches Yield, regardless of type parameters)
    /// Type parameters should unify, but for now we just check the effect kind
    fn effect_matches_any(&self, inferred: &SideEffect, declared: &[SideEffect]) -> bool {
        declared.iter().any(|decl| match (inferred, decl) {
            (SideEffect::Yield(_), SideEffect::Yield(_)) => true,
        })
    }

    /// Pop a type from a stack type, returning (rest, top)
    fn pop_type(&self, stack: &StackType, context: &str) -> Result<(StackType, Type), String> {
        match stack {
            StackType::Cons { rest, top } => Ok(((**rest).clone(), top.clone())),
            StackType::Empty => Err(format!(
                "{}: stack underflow - expected value on stack but stack is empty",
                context
            )),
            StackType::RowVar(_) => {
                // Can't statically determine if row variable is empty
                // For now, assume it has at least one element
                // This is conservative - real implementation would track constraints
                Err(format!(
                    "{}: cannot pop from polymorphic stack without more type information",
                    context
                ))
            }
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
