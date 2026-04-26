//! Token cursor: peek/advance/consume/match helpers over the token stream.

use super::{Parser, Token};

impl Parser {
    pub(super) fn skip_comments(&mut self) {
        loop {
            // The tokenizer splits `#` as a standalone token (alongside
            // `()[]{},;:`), so any `#`-introduced line comment — with or
            // without a space, including `#!shebang` lines — appears here
            // as a `"#"` token followed by zero-or-more tokens until the
            // next newline. We consume them all.
            let is_comment = if self.is_at_end() {
                false
            } else {
                self.current() == "#"
            };

            if is_comment {
                self.advance(); // consume the `#` token

                // Collect all tokens until newline to reconstruct the comment text
                let mut comment_parts: Vec<String> = Vec::new();
                while !self.is_at_end() && self.current() != "\n" {
                    comment_parts.push(self.current().to_string());
                    self.advance();
                }
                if !self.is_at_end() {
                    self.advance(); // skip newline
                }

                // Join parts and check for seq:allow annotation
                // Format: # seq:allow(lint-id) -> parts = ["seq", ":", "allow", "(", "lint-id", ")"]
                let comment = comment_parts.join("");
                if let Some(lint_id) = comment
                    .strip_prefix("seq:allow(")
                    .and_then(|s| s.strip_suffix(")"))
                {
                    self.pending_allowed_lints.push(lint_id.to_string());
                }
            } else if self.check("\n") {
                // Skip blank lines
                self.advance();
            } else {
                break;
            }
        }
    }

    pub(super) fn check(&self, expected: &str) -> bool {
        if self.is_at_end() {
            return false;
        }
        self.current() == expected
    }

    pub(super) fn consume(&mut self, expected: &str) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Get the text of the current token
    pub(super) fn current(&self) -> &str {
        if self.is_at_end() {
            ""
        } else {
            &self.tokens[self.pos].text
        }
    }

    /// Get the full current token with position info
    pub(super) fn current_token(&self) -> Option<&Token> {
        if self.is_at_end() {
            None
        } else {
            Some(&self.tokens[self.pos])
        }
    }

    /// Peek at a token N positions ahead without consuming
    pub(super) fn peek_at(&self, n: usize) -> Option<&str> {
        let idx = self.pos + n;
        if idx < self.tokens.len() {
            Some(&self.tokens[idx].text)
        } else {
            None
        }
    }

    /// Advance and return the token text (for compatibility with existing code)
    pub(super) fn advance(&mut self) -> Option<&String> {
        if self.is_at_end() {
            None
        } else {
            let token = &self.tokens[self.pos];
            self.pos += 1;
            Some(&token.text)
        }
    }

    /// Advance and return the full token with position info
    pub(super) fn advance_token(&mut self) -> Option<&Token> {
        if self.is_at_end() {
            None
        } else {
            let token = &self.tokens[self.pos];
            self.pos += 1;
            Some(token)
        }
    }

    pub(super) fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }
}
