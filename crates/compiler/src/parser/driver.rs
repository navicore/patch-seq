//! Public entry points: construction, external-union registration, top-level parse.
use crate::ast::Program;

use super::{Parser, annotate_error_with_line, tokenize};

impl Parser {
    pub fn new(source: &str) -> Self {
        let tokens = tokenize(source);
        Parser {
            tokens,
            pos: 0,
            next_quotation_id: 0,
            pending_allowed_lints: Vec::new(),
            known_unions: std::collections::HashSet::new(),
        }
    }

    /// Register external union names (e.g., from included modules)
    /// These union types will be recognized in stack effect declarations.
    pub fn register_external_unions(&mut self, union_names: &[&str]) {
        for name in union_names {
            self.known_unions.insert(name.to_string());
        }
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut program = Program::new();

        // Check for unclosed string error from tokenizer
        if let Some(error_token) = self.tokens.iter().find(|t| *t == "<<<UNCLOSED_STRING>>>") {
            return Err(format!(
                "Unclosed string literal at line {}, column {} - missing closing quote",
                error_token.line + 1, // 1-indexed for user display
                error_token.column + 1
            ));
        }

        while !self.is_at_end() {
            self.skip_comments();
            if self.is_at_end() {
                break;
            }

            // Dispatch to the appropriate sub-parser. If the sub-parser returns
            // an error, annotate it with the current token's line so the LSP
            // can surface the diagnostic at the offending location rather than
            // defaulting to line 1.
            let result = if self.check("include") {
                self.parse_include().map(|inc| program.includes.push(inc))
            } else if self.check("union") {
                self.parse_union_def().map(|u| program.unions.push(u))
            } else {
                self.parse_word_def().map(|w| program.words.push(w))
            };

            if let Err(msg) = result {
                // Prefer the token we were looking at when the error fired.
                // If we were already at EOF, fall back to the final token's line
                // so the diagnostic lands near the unterminated construct
                // instead of on line 1.
                let loc_token = self.current_token().or_else(|| self.tokens.last());
                return Err(annotate_error_with_line(msg, loc_token));
            }
        }

        Ok(program)
    }
}
