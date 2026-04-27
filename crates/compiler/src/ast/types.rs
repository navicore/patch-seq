//! Abstract Syntax Tree for Seq
//!
//! Minimal AST sufficient for hello-world and basic programs.
//! Will be extended as we add more language features.

use crate::types::Effect;
use std::path::PathBuf;

/// Source location for error reporting and tooling
#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    pub file: PathBuf,
    /// Start line (0-indexed for LSP compatibility)
    pub start_line: usize,
    /// End line (0-indexed, inclusive)
    pub end_line: usize,
}

impl SourceLocation {
    /// Create a new source location with just a single line (for backward compatibility)
    pub fn new(file: PathBuf, line: usize) -> Self {
        SourceLocation {
            file,
            start_line: line,
            end_line: line,
        }
    }

    /// Create a source location spanning multiple lines
    pub fn span(file: PathBuf, start_line: usize, end_line: usize) -> Self {
        debug_assert!(
            start_line <= end_line,
            "SourceLocation: start_line ({}) must be <= end_line ({})",
            start_line,
            end_line
        );
        SourceLocation {
            file,
            start_line,
            end_line,
        }
    }
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.start_line == self.end_line {
            write!(f, "{}:{}", self.file.display(), self.start_line + 1)
        } else {
            write!(
                f,
                "{}:{}-{}",
                self.file.display(),
                self.start_line + 1,
                self.end_line + 1
            )
        }
    }
}

/// Include statement
#[derive(Debug, Clone, PartialEq)]
pub enum Include {
    /// Standard library include: `include std:http`
    Std(String),
    /// Relative path include: `include "my-utils"`
    Relative(String),
    /// FFI library include: `include ffi:readline`
    Ffi(String),
}

// ============================================================================
//                     ALGEBRAIC DATA TYPES (ADTs)
// ============================================================================

/// A field in a union variant
/// Example: `response-chan: Int`
#[derive(Debug, Clone, PartialEq)]
pub struct UnionField {
    pub name: String,
    pub type_name: String, // For now, just store the type name as string
}

/// A variant in a union type
/// Example: `Get { response-chan: Int }`
#[derive(Debug, Clone, PartialEq)]
pub struct UnionVariant {
    pub name: String,
    pub fields: Vec<UnionField>,
    pub source: Option<SourceLocation>,
}

/// A union type definition
/// Example:
/// ```seq
/// union Message {
///   Get { response-chan: Int }
///   Increment { response-chan: Int }
///   Report { op: Int, delta: Int, total: Int }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct UnionDef {
    pub name: String,
    pub variants: Vec<UnionVariant>,
    pub source: Option<SourceLocation>,
}

/// A pattern in a match expression
/// For Phase 1: just the variant name (stack-based matching)
/// Later phases will add field bindings: `Get { chan }`
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Match a variant by name, pushing all fields to stack
    /// Example: `Get ->` pushes response-chan to stack
    Variant(String),

    /// Match a variant with named field bindings (Phase 5)
    /// Example: `Get { chan } ->` binds chan to the response-chan field
    VariantWithBindings { name: String, bindings: Vec<String> },
}

/// A single arm in a match expression
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Vec<Statement>,
    /// Source span for error reporting (points to variant name)
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub includes: Vec<Include>,
    pub unions: Vec<UnionDef>,
    pub words: Vec<WordDef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WordDef {
    pub name: String,
    /// Optional stack effect declaration
    /// Example: ( ..a Int -- ..a Bool )
    pub effect: Option<Effect>,
    pub body: Vec<Statement>,
    /// Source location for error reporting (collision detection)
    pub source: Option<SourceLocation>,
    /// Lint IDs that are allowed (suppressed) for this word
    /// Set via `# seq:allow(lint-id)` annotation before the word definition
    pub allowed_lints: Vec<String>,
}

/// Source span for a single token or expression
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Span {
    /// Line number (0-indexed)
    pub line: usize,
    /// Start column (0-indexed)
    pub column: usize,
    /// Length of the span in characters
    pub length: usize,
}

impl Span {
    pub fn new(line: usize, column: usize, length: usize) -> Self {
        Span {
            line,
            column,
            length,
        }
    }
}

/// Source span for a quotation, supporting multi-line ranges
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QuotationSpan {
    /// Start line (0-indexed)
    pub start_line: usize,
    /// Start column (0-indexed)
    pub start_column: usize,
    /// End line (0-indexed)
    pub end_line: usize,
    /// End column (0-indexed, exclusive)
    pub end_column: usize,
}

impl QuotationSpan {
    pub fn new(start_line: usize, start_column: usize, end_line: usize, end_column: usize) -> Self {
        QuotationSpan {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    /// Check if a position (line, column) falls within this span
    pub fn contains(&self, line: usize, column: usize) -> bool {
        if line < self.start_line || line > self.end_line {
            return false;
        }
        if line == self.start_line && column < self.start_column {
            return false;
        }
        if line == self.end_line && column >= self.end_column {
            return false;
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Integer literal: pushes value onto stack
    IntLiteral(i64),

    /// Floating-point literal: pushes IEEE 754 double onto stack
    FloatLiteral(f64),

    /// Boolean literal: pushes true/false onto stack
    BoolLiteral(bool),

    /// String literal: pushes a byte-clean string onto the stack.
    ///
    /// The payload is `Vec<u8>` because Seq strings are arbitrary byte
    /// sequences (`\xNN` escapes produce literal bytes, embedded NULs are
    /// legal). Most literals happen to be UTF-8 text; the type stays
    /// general so binary-protocol authors can write magic numbers,
    /// alignment NULs, and IEEE-754 byte patterns inline.
    StringLiteral(Vec<u8>),

    /// Symbol literal: pushes symbol onto stack
    /// Syntax: :foo, :some-name, :ok
    /// Used for dynamic variant construction and SON.
    /// Note: Symbols are not currently interned (future optimization).
    Symbol(String),

    /// Word call: calls another word or built-in
    /// Contains the word name and optional source span for precise diagnostics
    WordCall { name: String, span: Option<Span> },

    /// Conditional: if/else/then
    ///
    /// Pops an integer from the stack (0 = zero, non-zero = non-zero)
    /// and executes the appropriate branch
    If {
        /// Statements to execute when condition is non-zero (the 'then' clause)
        then_branch: Vec<Statement>,
        /// Optional statements to execute when condition is zero (the 'else' clause)
        else_branch: Option<Vec<Statement>>,
        /// Source span for error reporting (points to 'if' keyword)
        span: Option<Span>,
    },

    /// Quotation: [ ... ]
    ///
    /// A block of deferred code (quotation/lambda)
    /// Quotations are first-class values that can be pushed onto the stack
    /// and executed later with combinators like `call`, `times`, or `while`
    ///
    /// The id field is used by the typechecker to track the inferred type
    /// (Quotation vs Closure) for this quotation. The id is assigned during parsing.
    /// The span field records the source location for LSP hover support.
    Quotation {
        id: usize,
        body: Vec<Statement>,
        span: Option<QuotationSpan>,
    },

    /// Match expression: pattern matching on union types
    ///
    /// Pops a union value from the stack and dispatches to the
    /// appropriate arm based on the variant tag.
    ///
    /// Example:
    /// ```seq
    /// match
    ///   Get -> send-response
    ///   Increment -> do-increment send-response
    ///   Report -> aggregate-add
    /// end
    /// ```
    Match {
        /// The match arms in order
        arms: Vec<MatchArm>,
        /// Source span for error reporting (points to 'match' keyword)
        span: Option<Span>,
    },
}
