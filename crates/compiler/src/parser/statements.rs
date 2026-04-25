//! Statement parsing: literals, word calls, if/else, quotations, match.
use crate::ast::{MatchArm, Pattern, Span, Statement};

use super::{Parser, is_float_literal, unescape_string};

impl Parser {
    pub(super) fn parse_statement(&mut self) -> Result<Statement, String> {
        use crate::ast::Span;
        let tok = self.advance_token().ok_or("Unexpected end of file")?;
        let token = &tok.text;
        let tok_line = tok.line;
        let tok_column = tok.column;
        let tok_len = tok.text.len();

        // Check if it looks like a float literal (contains . or scientific notation)
        // Must check this BEFORE integer parsing
        if let Some(f) = is_float_literal(token)
            .then(|| token.parse::<f64>().ok())
            .flatten()
        {
            return Ok(Statement::FloatLiteral(f));
        }

        // Try to parse as hex literal (0x or 0X prefix)
        if let Some(hex) = token
            .strip_prefix("0x")
            .or_else(|| token.strip_prefix("0X"))
        {
            return i64::from_str_radix(hex, 16)
                .map(Statement::IntLiteral)
                .map_err(|_| format!("Invalid hex literal: {}", token));
        }

        // Try to parse as binary literal (0b or 0B prefix)
        if let Some(bin) = token
            .strip_prefix("0b")
            .or_else(|| token.strip_prefix("0B"))
        {
            return i64::from_str_radix(bin, 2)
                .map(Statement::IntLiteral)
                .map_err(|_| format!("Invalid binary literal: {}", token));
        }

        // Try to parse as decimal integer literal
        if let Ok(n) = token.parse::<i64>() {
            return Ok(Statement::IntLiteral(n));
        }

        // Try to parse as boolean literal
        if token == "true" {
            return Ok(Statement::BoolLiteral(true));
        }
        if token == "false" {
            return Ok(Statement::BoolLiteral(false));
        }

        // Try to parse as symbol literal (:foo, :some-name)
        if token == ":" {
            // Get the next token as the symbol name
            let name_tok = self
                .advance_token()
                .ok_or("Expected symbol name after ':', got end of input")?;
            let name = &name_tok.text;
            // Validate symbol name (identifier-like, kebab-case allowed)
            if name.is_empty() {
                return Err("Symbol name cannot be empty".to_string());
            }
            if name.starts_with(|c: char| c.is_ascii_digit()) {
                return Err(format!(
                    "Symbol name cannot start with a digit: ':{}'\n  Hint: Symbol names must start with a letter",
                    name
                ));
            }
            if let Some(bad_char) = name.chars().find(|c| {
                !c.is_alphanumeric()
                    && *c != '-'
                    && *c != '_'
                    && *c != '.'
                    && *c != '?'
                    && *c != '!'
            }) {
                return Err(format!(
                    "Symbol name contains invalid character '{}': ':{}'\n  Hint: Allowed: letters, digits, - _ . ? !",
                    bad_char, name
                ));
            }
            return Ok(Statement::Symbol(name.clone()));
        }

        // Try to parse as string literal
        if token.starts_with('"') {
            // Validate token has at least opening and closing quotes
            if token.len() < 2 || !token.ends_with('"') {
                return Err(format!("Malformed string literal: {}", token));
            }
            // Strip exactly one quote from each end (not all quotes, which would
            // incorrectly handle escaped quotes at string boundaries like "hello\"")
            let raw = &token[1..token.len() - 1];
            let unescaped = unescape_string(raw)?;
            return Ok(Statement::StringLiteral(unescaped));
        }

        // `if` / `else` / `then` are no longer parser keywords (Seq 6.0).
        // Conditional control flow is now expressed via the `__if__`
        // combinator (renamed to `if` once the migration is complete) and
        // its `when` / `unless` library variants. Reject the old syntax
        // with an explicit pointer at the migration doc.
        if token == "if" || token == "else" || token == "then" {
            return Err(format!(
                "at line {}: '{}' is no longer a parser keyword in Seq 6.0.\n  \
                 Conditionals are now expressed with the `__if__` combinator:\n  \
                   `cond if A else B then`  →  `cond [ A ] [ B ] __if__`\n  \
                   `cond if A then`         →  `cond [ A ] when`\n  \
                 See docs/MIGRATION_6_0.md for the full transformation rules.",
                tok_line + 1,
                token
            ));
        }

        // Check for quotation
        if token == "[" {
            return self.parse_quotation(tok_line, tok_column);
        }

        // Check for match expression
        if token == "match" {
            return self.parse_match(tok_line, tok_column);
        }

        // Otherwise it's a word call - preserve source span for precise diagnostics
        Ok(Statement::WordCall {
            name: token.to_string(),
            span: Some(Span::new(tok_line, tok_column, tok_len)),
        })
    }

    pub(super) fn parse_quotation(
        &mut self,
        start_line: usize,
        start_column: usize,
    ) -> Result<Statement, String> {
        use crate::ast::QuotationSpan;
        let mut body = Vec::new();

        // Parse statements until ']'
        loop {
            if self.is_at_end() {
                return Err("Unexpected end of file in quotation".to_string());
            }

            // Skip comments and newlines
            self.skip_comments();

            if self.check("]") {
                let end_tok = self.advance_token().unwrap();
                let end_line = end_tok.line;
                let end_column = end_tok.column + 1; // exclusive
                let id = self.next_quotation_id;
                self.next_quotation_id += 1;
                // Span from '[' to ']' inclusive
                let span = QuotationSpan::new(start_line, start_column, end_line, end_column);
                return Ok(Statement::Quotation {
                    id,
                    body,
                    span: Some(span),
                });
            }

            body.push(self.parse_statement()?);
        }
    }

    /// Parse a match expression:
    ///   match
    ///     Get -> send-response
    ///     Increment -> do-increment send-response
    ///     Report -> aggregate-add
    ///   end
    pub(super) fn parse_match(
        &mut self,
        start_line: usize,
        start_column: usize,
    ) -> Result<Statement, String> {
        let mut arms = Vec::new();

        loop {
            self.skip_comments();

            // Check for 'end' to terminate match
            if self.check("end") {
                self.advance();
                break;
            }

            if self.is_at_end() {
                return Err("Unexpected end of file in match expression".to_string());
            }

            arms.push(self.parse_match_arm()?);
        }

        if arms.is_empty() {
            return Err("Match expression must have at least one arm".to_string());
        }

        Ok(Statement::Match {
            arms,
            span: Some(Span::new(start_line, start_column, "match".len())),
        })
    }

    /// Parse a single match arm:
    ///   Get -> send-response
    ///   or with bindings:
    ///   Get { chan } -> chan send-response
    pub(super) fn parse_match_arm(&mut self) -> Result<MatchArm, String> {
        // Get variant name with position info
        let variant_token = self
            .advance_token()
            .ok_or("Expected variant name in match arm")?;
        let variant_name = variant_token.text.clone();
        let arm_line = variant_token.line;
        let arm_column = variant_token.column;
        let arm_length = variant_name.len();

        self.skip_comments();

        // Check for optional bindings: { field1 field2 }
        let pattern = if self.check("{") {
            self.consume("{");
            let mut bindings = Vec::new();

            loop {
                self.skip_comments();

                if self.check("}") {
                    break;
                }

                if self.is_at_end() {
                    return Err(format!(
                        "Unexpected end of file in match arm bindings for '{}'",
                        variant_name
                    ));
                }

                let token = self.advance().ok_or("Expected binding name")?.clone();

                // Require > prefix to make clear these are stack extractions, not variables
                if let Some(field_name) = token.strip_prefix('>') {
                    if field_name.is_empty() {
                        return Err(format!(
                            "Expected field name after '>' in match bindings for '{}'",
                            variant_name
                        ));
                    }
                    bindings.push(field_name.to_string());
                } else {
                    return Err(format!(
                        "Match bindings must use '>' prefix to indicate stack extraction. \
                         Use '>{}' instead of '{}' in pattern for '{}'",
                        token, token, variant_name
                    ));
                }
            }

            self.consume("}");
            Pattern::VariantWithBindings {
                name: variant_name,
                bindings,
            }
        } else {
            Pattern::Variant(variant_name.clone())
        };

        self.skip_comments();

        // Expect '->' arrow
        if !self.consume("->") {
            return Err(format!(
                "Expected '->' after pattern '{}', got '{}'",
                match &pattern {
                    Pattern::Variant(n) => n.clone(),
                    Pattern::VariantWithBindings { name, .. } => name.clone(),
                },
                self.current()
            ));
        }

        // Parse body until next pattern or 'end'
        let mut body = Vec::new();
        loop {
            self.skip_comments();

            // Check for end of arm (next pattern starts with uppercase, or 'end')
            if self.check("end") {
                break;
            }

            // Check if next token looks like a match pattern (not just any uppercase word).
            // A pattern is: UppercaseName followed by '->' or '{'
            // This prevents confusing 'Make-Get' (constructor call) with a pattern.
            if let Some(token) = self.current_token()
                && let Some(first_char) = token.text.chars().next()
                && first_char.is_uppercase()
            {
                // Peek at next token to see if this is a pattern (followed by -> or {)
                if let Some(next) = self.peek_at(1)
                    && (next == "->" || next == "{")
                {
                    // This is the next pattern
                    break;
                }
                // Otherwise it's just an uppercase word call (like Make-Get), continue parsing body
            }

            if self.is_at_end() {
                return Err("Unexpected end of file in match arm body".to_string());
            }

            body.push(self.parse_statement()?);
        }

        Ok(MatchArm {
            pattern,
            body,
            span: Some(Span::new(arm_line, arm_column, arm_length)),
        })
    }
}
