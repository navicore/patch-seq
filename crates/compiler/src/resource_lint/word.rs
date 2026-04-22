//! Single-word resource analyzer — simulates one word body at a time
//! without cross-word knowledge. Used as a fallback or for isolated
//! analysis passes.

use std::path::Path;

use crate::ast::{MatchArm, Span, Statement, WordDef};
use crate::lint::{LintDiagnostic, Severity};

use super::state::{InconsistentResource, ResourceKind, StackState, StackValue, TrackedResource};

/// The resource leak analyzer (single-word analysis)
pub struct ResourceAnalyzer {
    /// Diagnostics collected during analysis
    diagnostics: Vec<LintDiagnostic>,
    /// File being analyzed
    file: std::path::PathBuf,
}

impl ResourceAnalyzer {
    pub fn new(file: &Path) -> Self {
        ResourceAnalyzer {
            diagnostics: Vec::new(),
            file: file.to_path_buf(),
        }
    }

    /// Analyze a word definition for resource leaks
    pub fn analyze_word(&mut self, word: &WordDef) -> Vec<LintDiagnostic> {
        self.diagnostics.clear();

        let mut state = StackState::new();

        // Analyze the word body
        self.analyze_statements(&word.body, &mut state, word);

        // Check for leaked resources at end of word
        // Note: Resources still on stack at word end could be:
        // 1. Intentionally returned (escape) - caller's responsibility
        // 2. Leaked - forgot to clean up
        //
        // For Phase 2a, we apply escape analysis: if a resource is still
        // on the stack at word end, it's being returned to the caller.
        // This is valid - the caller becomes responsible for cleanup.
        // We only warn about resources that are explicitly dropped without
        // cleanup, or handled inconsistently across branches.
        //
        // Phase 2b could add cross-word analysis to track if callers
        // properly handle returned resources.
        let _ = state.remaining_resources(); // Intentional: escape = no warning

        std::mem::take(&mut self.diagnostics)
    }

    /// Analyze a sequence of statements
    fn analyze_statements(
        &mut self,
        statements: &[Statement],
        state: &mut StackState,
        word: &WordDef,
    ) {
        for stmt in statements {
            self.analyze_statement(stmt, state, word);
        }
    }

    /// Analyze a single statement
    fn analyze_statement(&mut self, stmt: &Statement, state: &mut StackState, word: &WordDef) {
        match stmt {
            Statement::IntLiteral(_)
            | Statement::FloatLiteral(_)
            | Statement::BoolLiteral(_)
            | Statement::StringLiteral(_)
            | Statement::Symbol(_) => {
                state.push_unknown();
            }

            Statement::WordCall { name, span } => {
                self.analyze_word_call(name, span.as_ref(), state, word);
            }

            Statement::Quotation { body, .. } => {
                // Quotations capture the current stack conceptually but don't
                // execute immediately. For now, just push an unknown value
                // (the quotation itself). We could analyze the body when
                // we see `call`, but that's Phase 2b.
                let _ = body; // Acknowledge we're not analyzing the body yet
                state.push_unknown();
            }

            Statement::If {
                then_branch,
                else_branch,
                span: _,
            } => {
                self.analyze_if(then_branch, else_branch.as_ref(), state, word);
            }

            Statement::Match { arms, span: _ } => {
                self.analyze_match(arms, state, word);
            }
        }
    }

    /// Analyze a word call
    fn analyze_word_call(
        &mut self,
        name: &str,
        span: Option<&Span>,
        state: &mut StackState,
        word: &WordDef,
    ) {
        let line = span.map(|s| s.line).unwrap_or(0);

        match name {
            // Resource-creating words
            "strand.weave" => {
                // Pops quotation, pushes WeaveHandle
                state.pop(); // quotation
                state.push_resource(ResourceKind::WeaveHandle, line, name);
            }

            "chan.make" => {
                // Pushes a new channel
                state.push_resource(ResourceKind::Channel, line, name);
            }

            // Resource-consuming words
            "strand.weave-cancel" => {
                // Pops and consumes WeaveHandle
                if let Some(StackValue::Resource(r)) = state.pop()
                    && r.kind == ResourceKind::WeaveHandle
                {
                    state.consume_resource(r);
                }
            }

            "chan.close" => {
                // Pops and consumes Channel
                if let Some(StackValue::Resource(r)) = state.pop()
                    && r.kind == ResourceKind::Channel
                {
                    state.consume_resource(r);
                }
            }

            // strand.resume is special - it returns (handle value bool)
            // If bool is false, the weave completed and handle is consumed
            // We can't know statically, so we just track that the handle
            // is still in play (on the stack after resume)
            "strand.resume" => {
                // Pops (handle value), pushes (handle value bool)
                let value = state.pop(); // value to send
                let handle = state.pop(); // handle

                // Push them back plus the bool result
                if let Some(h) = handle {
                    state.stack.push(h);
                } else {
                    state.push_unknown();
                }
                if let Some(v) = value {
                    state.stack.push(v);
                } else {
                    state.push_unknown();
                }
                state.push_unknown(); // bool result
            }

            // Stack operations
            "drop" => {
                let dropped = state.pop();
                // If we dropped a resource without consuming it properly, that's a leak
                // But check if it was already consumed (e.g., transferred via strand.spawn)
                if let Some(StackValue::Resource(r)) = dropped {
                    let already_consumed = state.consumed.iter().any(|c| c.id == r.id);
                    if !already_consumed {
                        self.emit_drop_warning(&r, span, word);
                    }
                }
            }

            "dup" => {
                if let Some(top) = state.peek().cloned() {
                    state.stack.push(top);
                } else {
                    state.push_unknown();
                }
            }

            "swap" => {
                let a = state.pop();
                let b = state.pop();
                if let Some(av) = a {
                    state.stack.push(av);
                }
                if let Some(bv) = b {
                    state.stack.push(bv);
                }
            }

            "over" => {
                // ( a b -- a b a ) - copy second element to top
                if state.depth() >= 2 {
                    let second = state.stack[state.depth() - 2].clone();
                    state.stack.push(second);
                } else {
                    state.push_unknown();
                }
            }

            "rot" => {
                // ( a b c -- b c a )
                let c = state.pop();
                let b = state.pop();
                let a = state.pop();
                if let Some(bv) = b {
                    state.stack.push(bv);
                }
                if let Some(cv) = c {
                    state.stack.push(cv);
                }
                if let Some(av) = a {
                    state.stack.push(av);
                }
            }

            "nip" => {
                // ( a b -- b ) - drop second
                let b = state.pop();
                let a = state.pop();
                if let Some(StackValue::Resource(r)) = a {
                    let already_consumed = state.consumed.iter().any(|c| c.id == r.id);
                    if !already_consumed {
                        self.emit_drop_warning(&r, span, word);
                    }
                }
                if let Some(bv) = b {
                    state.stack.push(bv);
                }
            }

            ">aux" => {
                // Move top of main stack to aux stack (Issue #350)
                if let Some(val) = state.pop() {
                    state.aux_stack.push(val);
                }
            }

            "aux>" => {
                // Move top of aux stack back to main stack (Issue #350)
                if let Some(val) = state.aux_stack.pop() {
                    state.stack.push(val);
                }
            }

            "tuck" => {
                // ( a b -- b a b )
                let b = state.pop();
                let a = state.pop();
                if let Some(bv) = b.clone() {
                    state.stack.push(bv);
                }
                if let Some(av) = a {
                    state.stack.push(av);
                }
                if let Some(bv) = b {
                    state.stack.push(bv);
                }
            }

            "2dup" => {
                // ( a b -- a b a b )
                if state.depth() >= 2 {
                    let b = state.stack[state.depth() - 1].clone();
                    let a = state.stack[state.depth() - 2].clone();
                    state.stack.push(a);
                    state.stack.push(b);
                } else {
                    state.push_unknown();
                    state.push_unknown();
                }
            }

            "3drop" => {
                for _ in 0..3 {
                    if let Some(StackValue::Resource(r)) = state.pop() {
                        let already_consumed = state.consumed.iter().any(|c| c.id == r.id);
                        if !already_consumed {
                            self.emit_drop_warning(&r, span, word);
                        }
                    }
                }
            }

            "pick" => {
                // ( ... n -- ... value_at_n )
                // We can't know n statically, so just push unknown
                state.pop(); // pop n
                state.push_unknown();
            }

            "roll" => {
                // Similar to pick but also removes the item
                state.pop(); // pop n
                state.push_unknown();
            }

            // Channel operations that don't consume
            "chan.send" | "chan.receive" => {
                // These use the channel but don't consume it
                // chan.send: ( chan value -- bool )
                // chan.receive: ( chan -- value bool )
                state.pop();
                state.pop();
                state.push_unknown();
                state.push_unknown();
            }

            // strand.spawn clones the stack to the child strand
            // Resources on the stack are transferred to child's responsibility
            "strand.spawn" => {
                // Pops quotation, pushes strand-id
                // All resources currently on stack are now shared with child
                // Mark them as consumed since child takes responsibility
                state.pop(); // quotation
                let resources_on_stack: Vec<TrackedResource> = state
                    .stack
                    .iter()
                    .filter_map(|v| match v {
                        StackValue::Resource(r) => Some(r.clone()),
                        StackValue::Unknown => None,
                    })
                    .collect();
                for r in resources_on_stack {
                    state.consume_resource(r);
                }
                state.push_unknown(); // strand-id
            }

            // For any other word, we don't know its stack effect
            // Conservatively, we could assume it consumes/produces unknown values
            // For now, we just leave the stack unchanged (may cause false positives)
            _ => {
                // Unknown word - could be user-defined
                // We'd need type info to know its stack effect
                // For Phase 2a, we'll be conservative and do nothing
            }
        }
    }

    /// Analyze an if/else statement
    fn analyze_if(
        &mut self,
        then_branch: &[Statement],
        else_branch: Option<&Vec<Statement>>,
        state: &mut StackState,
        word: &WordDef,
    ) {
        // Pop the condition
        state.pop();

        // Clone state for each branch
        let mut then_state = state.clone();
        let mut else_state = state.clone();

        // Analyze then branch
        self.analyze_statements(then_branch, &mut then_state, word);

        // Analyze else branch if present
        if let Some(else_stmts) = else_branch {
            self.analyze_statements(else_stmts, &mut else_state, word);
        }

        // Check for inconsistent resource handling between branches
        let merge_result = then_state.merge(&else_state);
        for inconsistent in merge_result.inconsistent {
            self.emit_branch_inconsistency_warning(&inconsistent, word);
        }

        // Compute proper lattice join of both branch states
        // This ensures we track resources from either branch and only
        // consider resources consumed if consumed in BOTH branches
        *state = then_state.join(&else_state);
    }

    /// Analyze a match statement
    fn analyze_match(&mut self, arms: &[MatchArm], state: &mut StackState, word: &WordDef) {
        // Pop the matched value
        state.pop();

        if arms.is_empty() {
            return;
        }

        // Analyze each arm
        let mut arm_states: Vec<StackState> = Vec::new();

        for arm in arms {
            let mut arm_state = state.clone();

            // Match arms may push extracted fields - for now we push unknowns
            // based on the pattern (simplified)
            match &arm.pattern {
                crate::ast::Pattern::Variant(_) => {
                    // Variant match pushes all fields - we don't know how many
                    // so we just continue with current state
                }
                crate::ast::Pattern::VariantWithBindings { bindings, .. } => {
                    // Push unknowns for each binding
                    for _ in bindings {
                        arm_state.push_unknown();
                    }
                }
            }

            self.analyze_statements(&arm.body, &mut arm_state, word);
            arm_states.push(arm_state);
        }

        // Check consistency between all arms
        if arm_states.len() >= 2 {
            let first = &arm_states[0];
            for other in &arm_states[1..] {
                let merge_result = first.merge(other);
                for inconsistent in merge_result.inconsistent {
                    self.emit_branch_inconsistency_warning(&inconsistent, word);
                }
            }
        }

        // Compute proper lattice join of all arm states
        // Resources are only consumed if consumed in ALL arms
        if let Some(first) = arm_states.into_iter().reduce(|acc, s| acc.join(&s)) {
            *state = first;
        }
    }

    /// Emit a warning for a resource dropped without cleanup
    fn emit_drop_warning(
        &mut self,
        resource: &TrackedResource,
        span: Option<&Span>,
        word: &WordDef,
    ) {
        let line = span
            .map(|s| s.line)
            .unwrap_or_else(|| word.source.as_ref().map(|s| s.start_line).unwrap_or(0));
        let column = span.map(|s| s.column);

        self.diagnostics.push(LintDiagnostic {
            id: format!("resource-leak-{}", resource.kind.name().to_lowercase()),
            message: format!(
                "{} created at line {} dropped without cleanup - {}",
                resource.kind.name(),
                resource.created_line + 1,
                resource.kind.cleanup_suggestion()
            ),
            severity: Severity::Warning,
            replacement: String::new(),
            file: self.file.clone(),
            line,
            end_line: None,
            start_column: column,
            end_column: column.map(|c| c + 4), // approximate
            word_name: word.name.clone(),
            start_index: 0,
            end_index: 0,
        });
    }

    /// Emit a warning for inconsistent resource handling between branches
    fn emit_branch_inconsistency_warning(
        &mut self,
        inconsistent: &InconsistentResource,
        word: &WordDef,
    ) {
        let line = word.source.as_ref().map(|s| s.start_line).unwrap_or(0);
        let branch = if inconsistent.consumed_in_else {
            "else"
        } else {
            "then"
        };

        self.diagnostics.push(LintDiagnostic {
            id: "resource-branch-inconsistent".to_string(),
            message: format!(
                "{} created at line {} is consumed in {} branch but not the other - all branches must handle resources consistently",
                inconsistent.resource.kind.name(),
                inconsistent.resource.created_line + 1,
                branch
            ),
            severity: Severity::Warning,
            replacement: String::new(),
            file: self.file.clone(),
            line,
            end_line: None,
            start_column: None,
            end_column: None,
            word_name: word.name.clone(),
            start_index: 0,
            end_index: 0,
        });
    }
}
