//! Error Flag Detection (Phase 2b)
//!
//! Abstract stack simulation that tracks Bool values produced by fallible
//! operations. Warns when these "error flags" are dropped without being
//! checked via `if` or `cond`.
//!
//! This catches patterns that the TOML-based pattern linter misses:
//! - `file.slurp swap nip` (Bool moved by swap, then dropped by nip)
//! - `i./ >aux ... aux> drop` (Bool stashed on aux stack, dropped later)
//!
//! # Architecture
//!
//! Modeled on `resource_lint.rs`:
//! 1. Tag Bools from fallible ops with their origin
//! 2. Simulate stack operations to track tag movement
//! 3. When a tagged Bool is consumed by `if`/`cond`, mark checked
//! 4. When consumed by `drop`/`nip`/other, emit warning
//!
//! # Conservative Design
//!
//! - Only tracks Bools from known fallible builtins (not all Bools)
//! - If a tagged Bool flows into an unknown user word, assume checked
//!   (avoids false positives from cross-word analysis)
//! - Bools remaining on the stack at word end are assumed returned
//!   (escape analysis, same as resource_lint)

use crate::ast::{Program, Span, Statement, WordDef};
use crate::lint::{LintDiagnostic, Severity};
use std::path::{Path, PathBuf};

/// A tracked error flag with its origin
#[derive(Debug, Clone)]
struct ErrorFlag {
    /// Line where the fallible operation was called (0-indexed)
    created_line: usize,
    /// The operation that produced this flag
    operation: String,
    /// Human-readable description of what failure the Bool indicates
    description: String,
}

/// A value on the abstract stack
#[derive(Debug, Clone)]
enum StackVal {
    /// A tracked error flag that hasn't been checked yet
    Flag(ErrorFlag),
    /// Any other value (not tracked)
    Other,
}

/// Abstract stack state for error flag tracking
#[derive(Debug, Clone)]
struct FlagStack {
    stack: Vec<StackVal>,
    aux: Vec<StackVal>,
}

impl FlagStack {
    fn new() -> Self {
        FlagStack {
            stack: Vec::new(),
            aux: Vec::new(),
        }
    }

    fn push_other(&mut self) {
        self.stack.push(StackVal::Other);
    }

    fn push_flag(&mut self, line: usize, operation: &str, description: &str) {
        let flag = ErrorFlag {
            created_line: line,
            operation: operation.to_string(),
            description: description.to_string(),
        };
        self.stack.push(StackVal::Flag(flag));
    }

    fn pop(&mut self) -> Option<StackVal> {
        self.stack.pop()
    }

    fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Join two states after branching (conservative: keep flags from either)
    fn join(&self, other: &FlagStack) -> FlagStack {
        // Use the longer stack, preserving flags from either branch
        let len = self.stack.len().max(other.stack.len());
        let mut joined = Vec::with_capacity(len);

        for i in 0..len {
            let a = self.stack.get(i);
            let b = other.stack.get(i);
            // If either branch has a flag at this position, keep it
            let val = match (a, b) {
                (Some(StackVal::Flag(f)), _) => StackVal::Flag(f.clone()),
                (_, Some(StackVal::Flag(f))) => StackVal::Flag(f.clone()),
                _ => StackVal::Other,
            };
            joined.push(val);
        }

        // Join aux stacks similarly
        let aux_len = self.aux.len().max(other.aux.len());
        let mut joined_aux = Vec::with_capacity(aux_len);
        for i in 0..aux_len {
            let a = self.aux.get(i);
            let b = other.aux.get(i);
            let val = match (a, b) {
                (Some(StackVal::Flag(f)), _) => StackVal::Flag(f.clone()),
                (_, Some(StackVal::Flag(f))) => StackVal::Flag(f.clone()),
                _ => StackVal::Other,
            };
            joined_aux.push(val);
        }

        FlagStack {
            stack: joined,
            aux: joined_aux,
        }
    }
}

/// Information about a fallible operation.
struct FallibleOpInfo {
    /// Number of values the operation consumes from the stack
    inputs: usize,
    /// Number of values pushed BEFORE the Bool (e.g., 1 for `( -- String Bool )`)
    values_before_bool: usize,
    /// Human-readable description of what failure the Bool indicates
    description: &'static str,
}

/// Single source of truth for all fallible operations.
/// Maps operation name → (inputs consumed, values before Bool, description).
fn fallible_op_info(name: &str) -> Option<FallibleOpInfo> {
    let (inputs, values_before_bool, description) = match name {
        // Division — ( Int Int -- Int Bool )
        "i./" | "i.divide" => (2, 1, "division by zero"),
        "i.%" | "i.modulo" => (2, 1, "modulo by zero"),

        // File I/O
        "file.slurp" => (1, 1, "file read failure"),
        "file.spit" => (2, 0, "file write failure"),
        "file.append" => (2, 0, "file append failure"),
        "file.delete" => (1, 0, "file delete failure"),
        "file.size" => (1, 1, "file size failure"),
        "dir.make" => (1, 0, "directory creation failure"),
        "dir.delete" => (1, 0, "directory delete failure"),
        "dir.list" => (1, 1, "directory list failure"),

        // I/O — ( -- String Bool )
        "io.read-line" => (0, 1, "read failure"),

        // Parsing — ( String -- value Bool )
        "string->int" => (1, 1, "parse failure"),
        "string->float" => (1, 1, "parse failure"),

        // Channels
        "chan.send" => (2, 0, "send failure"),
        "chan.receive" => (1, 1, "receive failure"),

        // Map/List lookups
        "map.get" => (2, 1, "key not found"),
        "list.get" => (2, 1, "index out of bounds"),
        "list.set" => (3, 1, "index out of bounds"),

        // TCP
        "tcp.listen" => (1, 1, "listen failure"),
        "tcp.accept" => (1, 1, "accept failure"),
        "tcp.read" => (1, 1, "read failure"),
        "tcp.write" => (2, 0, "write failure"),
        "tcp.close" => (1, 0, "close failure"),

        // OS
        "os.getenv" => (1, 1, "env var not set"),
        "os.home-dir" => (0, 1, "home dir not available"),
        "os.current-dir" => (0, 1, "current dir not available"),
        "os.path-parent" => (1, 1, "no parent path"),
        "os.path-filename" => (1, 1, "no filename"),

        // Regex
        "regex.find" => (2, 1, "no match or invalid regex"),
        "regex.find-all" => (2, 1, "invalid regex"),
        "regex.replace" => (3, 1, "invalid regex"),
        "regex.replace-all" => (3, 1, "invalid regex"),
        "regex.captures" => (2, 1, "invalid regex"),
        "regex.split" => (2, 1, "invalid regex"),

        // Encoding
        "encoding.base64-decode" => (1, 1, "invalid base64"),
        "encoding.base64url-decode" => (1, 1, "invalid base64url"),
        "encoding.hex-decode" => (1, 1, "invalid hex"),

        // Crypto
        "crypto.aes-gcm-encrypt" => (2, 1, "encryption failure"),
        "crypto.aes-gcm-decrypt" => (2, 1, "decryption failure"),
        "crypto.pbkdf2-sha256" => (3, 1, "key derivation failure"),
        "crypto.ed25519-sign" => (2, 1, "signing failure"),

        // Compression
        "compress.gzip" => (1, 1, "compression failure"),
        "compress.gzip-level" => (2, 1, "compression failure"),
        "compress.gunzip" => (1, 1, "decompression failure"),
        "compress.zstd" => (1, 1, "compression failure"),
        "compress.zstd-level" => (2, 1, "compression failure"),
        "compress.unzstd" => (1, 1, "decompression failure"),

        _ => return None,
    };
    Some(FallibleOpInfo {
        inputs,
        values_before_bool,
        description,
    })
}

/// Words that consume a Bool as an error-checking mechanism
fn is_checking_consumer(name: &str) -> bool {
    // `if` is handled structurally (it's a Statement::If, not a WordCall)
    // `cond` consumes Bools as conditions
    name == "cond"
}

/// Analyzer for unchecked error flags
pub struct ErrorFlagAnalyzer {
    file: PathBuf,
    diagnostics: Vec<LintDiagnostic>,
}

impl ErrorFlagAnalyzer {
    pub fn new(file: &Path) -> Self {
        ErrorFlagAnalyzer {
            file: file.to_path_buf(),
            diagnostics: Vec::new(),
        }
    }

    pub fn analyze_program(&mut self, program: &Program) -> Vec<LintDiagnostic> {
        let mut all_diagnostics = Vec::new();
        for word in &program.words {
            // Skip words with seq:allow(unchecked-error-flag)
            if word
                .allowed_lints
                .iter()
                .any(|l| l == "unchecked-error-flag")
            {
                continue;
            }
            let diags = self.analyze_word(word);
            all_diagnostics.extend(diags);
        }
        all_diagnostics
    }

    fn analyze_word(&mut self, word: &WordDef) -> Vec<LintDiagnostic> {
        self.diagnostics.clear();
        let mut state = FlagStack::new();
        self.analyze_statements(&word.body, &mut state, word);
        // Flags remaining on stack at word end = returned to caller (escape)
        std::mem::take(&mut self.diagnostics)
    }

    fn analyze_statements(
        &mut self,
        statements: &[Statement],
        state: &mut FlagStack,
        word: &WordDef,
    ) {
        for stmt in statements {
            self.analyze_statement(stmt, state, word);
        }
    }

    fn analyze_statement(&mut self, stmt: &Statement, state: &mut FlagStack, word: &WordDef) {
        match stmt {
            Statement::IntLiteral(_)
            | Statement::FloatLiteral(_)
            | Statement::BoolLiteral(_)
            | Statement::StringLiteral(_)
            | Statement::Symbol(_) => {
                state.push_other();
            }

            Statement::Quotation { .. } => {
                state.push_other();
            }

            Statement::WordCall { name, span } => {
                self.analyze_word_call(name, span.as_ref(), state, word);
            }

            Statement::If {
                then_branch,
                else_branch,
                span: _,
            } => {
                // `if` consumes the Bool on top — this IS a check
                state.pop();

                let mut then_state = state.clone();
                let mut else_state = state.clone();
                self.analyze_statements(then_branch, &mut then_state, word);
                if let Some(else_stmts) = else_branch {
                    self.analyze_statements(else_stmts, &mut else_state, word);
                }
                *state = then_state.join(&else_state);
            }

            Statement::Match { arms, span: _ } => {
                state.pop(); // match value consumed
                let mut arm_states: Vec<FlagStack> = Vec::new();
                for arm in arms {
                    let mut arm_state = state.clone();
                    // Match arm bindings push values onto stack
                    match &arm.pattern {
                        crate::ast::Pattern::Variant(_) => {
                            // Variant without named bindings — field count unknown
                            // statically. Same limitation as resource_lint.
                        }
                        crate::ast::Pattern::VariantWithBindings { bindings, .. } => {
                            for _binding in bindings {
                                arm_state.push_other();
                            }
                        }
                    }
                    self.analyze_statements(&arm.body, &mut arm_state, word);
                    arm_states.push(arm_state);
                }
                if let Some(joined) = arm_states.into_iter().reduce(|acc, s| acc.join(&s)) {
                    *state = joined;
                }
            }
        }
    }

    fn analyze_word_call(
        &mut self,
        name: &str,
        span: Option<&Span>,
        state: &mut FlagStack,
        word: &WordDef,
    ) {
        let line = span.map(|s| s.line).unwrap_or(0);

        // Check if this is a fallible operation
        if let Some(info) = fallible_op_info(name) {
            // Pop inputs consumed by the operation
            for _ in 0..info.inputs {
                state.pop();
            }
            // Push output values, then the error flag Bool
            for _ in 0..info.values_before_bool {
                state.push_other();
            }
            state.push_flag(line, name, info.description);
            return;
        }

        // Check if this is a checking consumer
        if is_checking_consumer(name) {
            // `cond` is a multi-way conditional that consumes quotation pairs
            // + a count from the stack. Its variable arity means we can't
            // precisely model what it consumes. Conservative: assume it
            // checks any flags it touches (no warning), but don't clear
            // the entire stack — flags below the cond args may still need checking.
            state.pop(); // at minimum, the count argument
            return;
        }

        // Stack operations — simulate movement
        match name {
            "drop" => {
                if let Some(StackVal::Flag(flag)) = state.pop() {
                    self.emit_warning(&flag, line, word);
                }
            }
            "nip" => {
                // ( a b -- b ) — drops a (second from top)
                let top = state.pop();
                if let Some(StackVal::Flag(flag)) = state.pop() {
                    self.emit_warning(&flag, line, word);
                }
                if let Some(v) = top {
                    state.stack.push(v);
                }
            }
            "3drop" => {
                for _ in 0..3 {
                    if let Some(StackVal::Flag(flag)) = state.pop() {
                        self.emit_warning(&flag, line, word);
                    }
                }
            }
            "2drop" => {
                for _ in 0..2 {
                    if let Some(StackVal::Flag(flag)) = state.pop() {
                        self.emit_warning(&flag, line, word);
                    }
                }
            }
            "dup" => {
                if let Some(top) = state.stack.last().cloned() {
                    state.stack.push(top);
                }
            }
            "swap" => {
                let a = state.pop();
                let b = state.pop();
                if let Some(v) = a {
                    state.stack.push(v);
                }
                if let Some(v) = b {
                    state.stack.push(v);
                }
            }
            "over" => {
                if state.depth() >= 2 {
                    let second = state.stack[state.depth() - 2].clone();
                    state.stack.push(second);
                }
            }
            "rot" => {
                let c = state.pop();
                let b = state.pop();
                let a = state.pop();
                if let Some(v) = b {
                    state.stack.push(v);
                }
                if let Some(v) = c {
                    state.stack.push(v);
                }
                if let Some(v) = a {
                    state.stack.push(v);
                }
            }
            "tuck" => {
                let b = state.pop();
                let a = state.pop();
                if let Some(v) = b.clone() {
                    state.stack.push(v);
                }
                if let Some(v) = a {
                    state.stack.push(v);
                }
                if let Some(v) = b {
                    state.stack.push(v);
                }
            }
            "2dup" => {
                if state.depth() >= 2 {
                    let a = state.stack[state.depth() - 2].clone();
                    let b = state.stack[state.depth() - 1].clone();
                    state.stack.push(a);
                    state.stack.push(b);
                }
            }
            ">aux" => {
                if let Some(v) = state.pop() {
                    state.aux.push(v);
                }
            }
            "aux>" => {
                if let Some(v) = state.aux.pop() {
                    state.stack.push(v);
                }
            }
            "pick" | "roll" => {
                // Conservative: push unknown (can't statically know depth)
                state.push_other();
            }

            // Combinators — dip hides top, runs quotation, restores
            "dip" => {
                // ( x quot -- ? x ) — pop quot, pop x, run quot (unknown effect), push x
                state.pop(); // quotation
                let preserved = state.pop();
                // Quotation effect unknown — conservatively clear flags from stack
                // (quotation might check them, might not)
                state.stack.retain(|v| !matches!(v, StackVal::Flag(_)));
                if let Some(v) = preserved {
                    state.stack.push(v);
                }
            }
            "keep" => {
                // ( x quot -- ? x ) — similar to dip but quotation gets x
                state.pop(); // quotation
                let preserved = state.pop();
                state.stack.retain(|v| !matches!(v, StackVal::Flag(_)));
                if let Some(v) = preserved {
                    state.stack.push(v);
                }
            }
            "bi" => {
                // ( x q1 q2 -- ? ) — two quotations consume x
                state.pop(); // q2
                state.pop(); // q1
                state.pop(); // x
                // Both quotations have unknown effects
                state.stack.retain(|v| !matches!(v, StackVal::Flag(_)));
            }

            // call — quotation effect unknown, conservatively assume it checks
            "call" => {
                state.pop(); // quotation
                // Conservative: clear tracked flags (quotation might do anything)
                state.stack.retain(|v| !matches!(v, StackVal::Flag(_)));
            }

            // Known type-conversion words that consume one value and push one
            "int->string" | "int->float" | "float->int" | "float->string" | "char->string"
            | "symbol->string" | "string->symbol" => {
                // These consume the top value. If it's a flag, that's suspicious
                // but not necessarily wrong (e.g., converting a Bool to string for display).
                // Conservative: don't warn, just remove tracking.
                state.pop();
                state.push_other();
            }

            // Boolean operations that legitimately consume Bools
            "and" | "or" | "not" => {
                // These consume Bool(s) and produce Bool — not a check per se,
                // but the user is clearly working with the Bool value.
                // Conservative: mark as consumed (no warning).
                state.pop();
                if name != "not" {
                    state.pop();
                }
                state.push_other();
            }

            // Test assertions that check Bools
            "test.assert" | "test.assert-not" => {
                state.pop(); // Bool consumed by assertion = checked
            }

            // All other words: conservative — assume they consume/produce
            // unknown values. Pop any flags without warning (might be checked
            // inside the word).
            _ => {
                // For unknown words, we don't know the stack effect.
                // Conservative: leave the stack as-is (don't warn, don't clear).
                // This avoids false positives from user-defined words that
                // properly handle the Bool internally.
            }
        }
    }

    fn emit_warning(&mut self, flag: &ErrorFlag, drop_line: usize, word: &WordDef) {
        // Don't warn if the drop is adjacent to the operation (within 2 lines).
        // Adjacent drops like `tcp.write drop` are covered by the pattern-based
        // linter with better precision (exact column info, replacement suggestions).
        // We only add value for non-adjacent drops (e.g., swap nip, aux round-trips).
        // Note: if spans are missing, both lines default to 0 and this suppresses
        // the warning — acceptable since span-less nodes are rare (synthetic AST only).
        if drop_line <= flag.created_line + 2 {
            return;
        }

        self.diagnostics.push(LintDiagnostic {
            id: "unchecked-error-flag".to_string(),
            message: format!(
                "`{}` returns a Bool error flag (indicates {}) — dropped without checking",
                flag.operation, flag.description,
            ),
            severity: Severity::Warning,
            replacement: String::new(),
            file: self.file.clone(),
            line: flag.created_line,
            end_line: Some(drop_line),
            start_column: None,
            end_column: None,
            word_name: word.name.clone(),
            start_index: 0,
            end_index: 0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Statement, WordDef};
    use crate::types::{Effect, StackType};

    fn make_word(name: &str, body: Vec<Statement>) -> WordDef {
        WordDef {
            name: name.to_string(),
            effect: Some(Effect::new(StackType::Empty, StackType::Empty)),
            body,
            source: None,
            allowed_lints: vec![],
        }
    }

    fn word_call(name: &str, line: usize) -> Statement {
        Statement::WordCall {
            name: name.to_string(),
            span: Some(Span {
                line,
                column: 0,
                length: 1,
            }),
        }
    }

    #[test]
    fn test_adjacent_drop_not_flagged() {
        // file.slurp drop — same line, pattern linter handles this
        let word = make_word(
            "test",
            vec![
                Statement::StringLiteral("foo".to_string()),
                word_call("file.slurp", 1),
                word_call("drop", 1),
            ],
        );
        let mut analyzer = ErrorFlagAnalyzer::new(Path::new("test.seq"));
        let diags = analyzer.analyze_word(&word);
        assert!(
            diags.is_empty(),
            "Adjacent drop should be left to pattern linter"
        );
    }

    #[test]
    fn test_non_adjacent_drop_flagged() {
        // file.slurp swap nip — swap puts Bool below String, nip drops Bool
        // Stack: (String Bool) → swap → (Bool String) → nip → (String)
        // Bool was nipped without checking (lines spread apart)
        let word = make_word(
            "test",
            vec![
                Statement::StringLiteral("foo".to_string()),
                word_call("file.slurp", 1),
                word_call("swap", 5),
                word_call("nip", 10),
            ],
        );
        let mut analyzer = ErrorFlagAnalyzer::new(Path::new("test.seq"));
        let diags = analyzer.analyze_word(&word);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].id, "unchecked-error-flag");
        assert!(diags[0].message.contains("file.slurp"));
    }

    #[test]
    fn test_checked_by_if() {
        // file.slurp if ... then — Bool checked
        let word = make_word(
            "test",
            vec![
                Statement::StringLiteral("foo".to_string()),
                word_call("file.slurp", 1),
                Statement::If {
                    then_branch: vec![word_call("io.write-line", 3)],
                    else_branch: Some(vec![word_call("drop", 5)]),
                    span: Some(Span {
                        line: 2,
                        column: 0,
                        length: 2,
                    }),
                },
            ],
        );
        let mut analyzer = ErrorFlagAnalyzer::new(Path::new("test.seq"));
        let diags = analyzer.analyze_word(&word);
        assert!(diags.is_empty(), "Bool checked by if should not warn");
    }

    #[test]
    fn test_aux_round_trip_drop() {
        // file.slurp >aux ... aux> drop — Bool stashed and dropped
        let word = make_word(
            "test",
            vec![
                Statement::StringLiteral("foo".to_string()),
                word_call("file.slurp", 1),
                word_call(">aux", 5),
                Statement::StringLiteral("other work".to_string()),
                word_call("drop", 8),
                word_call("aux>", 12),
                word_call("drop", 15),
            ],
        );
        let mut analyzer = ErrorFlagAnalyzer::new(Path::new("test.seq"));
        let diags = analyzer.analyze_word(&word);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("file.slurp"));
    }

    #[test]
    fn test_division_checked() {
        // 10 0 i./ if ... then — division result checked
        let word = make_word(
            "test",
            vec![
                Statement::IntLiteral(10),
                Statement::IntLiteral(0),
                word_call("i./", 1),
                Statement::If {
                    then_branch: vec![],
                    else_branch: Some(vec![word_call("drop", 3)]),
                    span: Some(Span {
                        line: 2,
                        column: 0,
                        length: 2,
                    }),
                },
            ],
        );
        let mut analyzer = ErrorFlagAnalyzer::new(Path::new("test.seq"));
        let diags = analyzer.analyze_word(&word);
        assert!(diags.is_empty());
    }

    #[test]
    fn test_nip_preserves_flag_on_top() {
        // string->int produces (Int Bool). nip drops Int, keeps Bool on top.
        // Bool is still on stack (returned = escape). No warning.
        let word = make_word(
            "test",
            vec![
                Statement::StringLiteral("42".to_string()),
                word_call("string->int", 1),
                word_call("nip", 2),
            ],
        );
        let mut analyzer = ErrorFlagAnalyzer::new(Path::new("test.seq"));
        let diags = analyzer.analyze_word(&word);
        assert!(diags.is_empty(), "nip keeps Bool on top — no warning");
    }

    #[test]
    fn test_swap_nip_drops_flag() {
        // string->int swap nip — swap puts Bool below Int, nip drops Bool
        let word = make_word(
            "test",
            vec![
                Statement::StringLiteral("42".to_string()),
                word_call("string->int", 1),
                word_call("swap", 5),
                word_call("nip", 10),
            ],
        );
        let mut analyzer = ErrorFlagAnalyzer::new(Path::new("test.seq"));
        let diags = analyzer.analyze_word(&word);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("string->int"));
    }

    #[test]
    fn test_allow_suppresses_warning() {
        // seq:allow(unchecked-error-flag) should suppress the warning
        let word = WordDef {
            name: "test".to_string(),
            effect: Some(Effect::new(StackType::Empty, StackType::Empty)),
            body: vec![
                Statement::StringLiteral("foo".to_string()),
                word_call("file.slurp", 1),
                word_call("swap", 5),
                word_call("nip", 10),
            ],
            source: None,
            allowed_lints: vec!["unchecked-error-flag".to_string()],
        };
        let mut analyzer = ErrorFlagAnalyzer::new(Path::new("test.seq"));
        let program = crate::ast::Program {
            includes: vec![],
            unions: vec![],
            words: vec![word],
        };
        let diags = analyzer.analyze_program(&program);
        assert!(diags.is_empty(), "seq:allow should suppress warning");
    }

    #[test]
    fn test_multiple_flags_both_dropped() {
        // Two fallible calls, both flags dropped non-adjacently
        let word = make_word(
            "test",
            vec![
                Statement::StringLiteral("foo".to_string()),
                word_call("file.slurp", 1),   // pushes (String, Flag)
                word_call("swap", 5),         // (Flag, String)
                word_call("nip", 10),         // drops Flag #1
                word_call("string->int", 15), // pushes (Int, Flag)
                word_call("swap", 20),
                word_call("nip", 25), // drops Flag #2
            ],
        );
        let mut analyzer = ErrorFlagAnalyzer::new(Path::new("test.seq"));
        let diags = analyzer.analyze_word(&word);
        assert_eq!(diags.len(), 2, "Both flags should produce warnings");
    }

    #[test]
    fn test_dip_clears_flags_no_false_positive() {
        // dip runs a quotation with unknown effects — flags on the
        // pre-dip stack are conservatively cleared (no false positive)
        let word = make_word(
            "test",
            vec![
                Statement::StringLiteral("foo".to_string()),
                word_call("file.slurp", 1), // (String, Flag)
                Statement::Quotation {
                    id: 0,
                    body: vec![word_call("drop", 5)],
                    span: None,
                },
                word_call("dip", 10),
            ],
        );
        let mut analyzer = ErrorFlagAnalyzer::new(Path::new("test.seq"));
        let diags = analyzer.analyze_word(&word);
        assert!(
            diags.is_empty(),
            "dip conservatively clears flags — no false positive"
        );
    }
}
