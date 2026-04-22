//! Top-level item parsing: includes, union definitions, word definitions.
use crate::ast::{Include, SourceLocation, UnionDef, UnionField, UnionVariant, WordDef};

use super::Parser;

impl Parser {
    pub(super) fn parse_include(&mut self) -> Result<Include, String> {
        self.consume("include");

        let token = self
            .advance()
            .ok_or("Expected module name after 'include'")?
            .clone();

        // Check for std: prefix (tokenizer splits this into "std", ":", "name")
        if token == "std" {
            // Expect : token
            if !self.consume(":") {
                return Err("Expected ':' after 'std' in include statement".to_string());
            }
            // Get the module name
            let name = self
                .advance()
                .ok_or("Expected module name after 'std:'")?
                .clone();
            return Ok(Include::Std(name));
        }

        // Check for ffi: prefix
        if token == "ffi" {
            // Expect : token
            if !self.consume(":") {
                return Err("Expected ':' after 'ffi' in include statement".to_string());
            }
            // Get the library name
            let name = self
                .advance()
                .ok_or("Expected library name after 'ffi:'")?
                .clone();
            return Ok(Include::Ffi(name));
        }

        // Check for quoted string (relative path)
        if token.starts_with('"') && token.ends_with('"') {
            let path = token.trim_start_matches('"').trim_end_matches('"');
            return Ok(Include::Relative(path.to_string()));
        }

        Err(format!(
            "Invalid include syntax '{}'. Use 'include std:name', 'include ffi:lib', or 'include \"path\"'",
            token
        ))
    }

    /// Parse a union type definition:
    ///   union Message {
    ///     Get { response-chan: Int }
    ///     Increment { response-chan: Int }
    ///     Report { op: Int, delta: Int, total: Int }
    ///   }
    pub(super) fn parse_union_def(&mut self) -> Result<UnionDef, String> {
        // Capture start line from 'union' token
        let start_line = self.current_token().map(|t| t.line).unwrap_or(0);

        // Consume 'union' keyword
        self.consume("union");

        // Get union name (must start with uppercase)
        let name = self
            .advance()
            .ok_or("Expected union name after 'union'")?
            .clone();

        if !name
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            return Err(format!(
                "Union name '{}' must start with an uppercase letter",
                name
            ));
        }

        // RFC #345: Register this union name so it can be recognized in stack effects
        // This allows ( UnionName -- ) to parse as Union type, not a type variable
        self.known_unions.insert(name.clone());

        // Skip comments and newlines
        self.skip_comments();

        // Expect '{'
        if !self.consume("{") {
            return Err(format!(
                "Expected '{{' after union name '{}', got '{}'",
                name,
                self.current()
            ));
        }

        // Parse variants until '}'
        let mut variants = Vec::new();
        loop {
            self.skip_comments();

            if self.check("}") {
                break;
            }

            if self.is_at_end() {
                return Err(format!("Unexpected end of file in union '{}'", name));
            }

            variants.push(self.parse_union_variant()?);
        }

        // Capture end line from '}' token before consuming
        let end_line = self.current_token().map(|t| t.line).unwrap_or(start_line);

        // Consume '}'
        self.consume("}");

        if variants.is_empty() {
            return Err(format!("Union '{}' must have at least one variant", name));
        }

        // Check for duplicate variant names
        let mut seen_variants = std::collections::HashSet::new();
        for variant in &variants {
            if !seen_variants.insert(&variant.name) {
                return Err(format!(
                    "Duplicate variant name '{}' in union '{}'",
                    variant.name, name
                ));
            }
        }

        Ok(UnionDef {
            name,
            variants,
            source: Some(SourceLocation::span(
                std::path::PathBuf::new(),
                start_line,
                end_line,
            )),
        })
    }

    /// Parse a single union variant:
    ///   Get { response-chan: Int }
    ///   or just: Empty (no fields)
    pub(super) fn parse_union_variant(&mut self) -> Result<UnionVariant, String> {
        let start_line = self.current_token().map(|t| t.line).unwrap_or(0);

        // Get variant name (must start with uppercase)
        let name = self.advance().ok_or("Expected variant name")?.clone();

        if !name
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            return Err(format!(
                "Variant name '{}' must start with an uppercase letter",
                name
            ));
        }

        self.skip_comments();

        // Check for optional fields
        let fields = if self.check("{") {
            self.consume("{");
            let fields = self.parse_union_fields()?;
            if !self.consume("}") {
                return Err(format!("Expected '}}' after variant '{}' fields", name));
            }
            fields
        } else {
            Vec::new()
        };

        Ok(UnionVariant {
            name,
            fields,
            source: Some(SourceLocation::new(std::path::PathBuf::new(), start_line)),
        })
    }

    /// Parse union fields: name: Type, name: Type, ...
    pub(super) fn parse_union_fields(&mut self) -> Result<Vec<UnionField>, String> {
        let mut fields = Vec::new();

        loop {
            self.skip_comments();

            if self.check("}") {
                break;
            }

            // Get field name
            let field_name = self.advance().ok_or("Expected field name")?.clone();

            // Expect ':'
            if !self.consume(":") {
                return Err(format!(
                    "Expected ':' after field name '{}', got '{}'",
                    field_name,
                    self.current()
                ));
            }

            // Get type name
            let type_name = self
                .advance()
                .ok_or("Expected type name after ':'")?
                .clone();

            fields.push(UnionField {
                name: field_name,
                type_name,
            });

            // Optional comma separator
            self.skip_comments();
            self.consume(",");
        }

        // Check for duplicate field names
        let mut seen_fields = std::collections::HashSet::new();
        for field in &fields {
            if !seen_fields.insert(&field.name) {
                return Err(format!("Duplicate field name '{}' in variant", field.name));
            }
        }

        Ok(fields)
    }

    pub(super) fn parse_word_def(&mut self) -> Result<WordDef, String> {
        // Consume any pending lint annotations collected from comments before this word
        let allowed_lints = std::mem::take(&mut self.pending_allowed_lints);

        // Capture start line from ':' token
        let start_line = self.current_token().map(|t| t.line).unwrap_or(0);

        // Expect ':'
        if !self.consume(":") {
            return Err(format!(
                "Expected ':' to start word definition, got '{}'",
                self.current()
            ));
        }

        // Get word name
        let name = self
            .advance()
            .ok_or("Expected word name after ':'")?
            .clone();

        // Parse stack effect if present: ( ..a Int -- ..a Bool )
        let effect = if self.check("(") {
            Some(self.parse_stack_effect()?)
        } else {
            None
        };

        // Parse body until ';'
        let mut body = Vec::new();
        while !self.check(";") {
            if self.is_at_end() {
                return Err(format!("Unexpected end of file in word '{}'", name));
            }

            // Skip comments and newlines in body
            self.skip_comments();
            if self.check(";") {
                break;
            }

            body.push(self.parse_statement()?);
        }

        // Capture end line from ';' token before consuming
        let end_line = self.current_token().map(|t| t.line).unwrap_or(start_line);

        // Consume ';'
        self.consume(";");

        Ok(WordDef {
            name,
            effect,
            body,
            source: Some(crate::ast::SourceLocation::span(
                std::path::PathBuf::new(),
                start_line,
                end_line,
            )),
            allowed_lints,
        })
    }
}
