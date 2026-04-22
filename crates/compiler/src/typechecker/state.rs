//! Constructors, accessors, and bookkeeping for TypeChecker.
use std::collections::HashMap;

use crate::call_graph::CallGraph;
use crate::types::{Effect, StackType, Type, UnionTypeInfo, VariantInfo};

use super::{TypeChecker, format_line_prefix};

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
    pub(super) fn line_prefix(&self) -> String {
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
    pub(super) fn find_variant(&self, variant_name: &str) -> Option<(&str, &VariantInfo)> {
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
    pub(super) fn capture_statement_type(
        &self,
        word_name: &str,
        stmt_index: usize,
        stack: &StackType,
    ) {
        if let Some(top_type) = Self::get_trivially_copyable_top(stack) {
            self.statement_top_types
                .borrow_mut()
                .insert((word_name.to_string(), stmt_index), top_type);
        }
    }
}
