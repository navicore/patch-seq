//! Token type and low-level tokenization/escape/float helpers.

/// A token with its source position (1-indexed).
#[derive(Debug, Clone)]
pub struct Token {
    pub text: String,
    /// Line number (0-indexed for LSP compatibility)
    pub line: usize,
    /// Column number (0-indexed)
    pub column: usize,
}

impl Token {
    fn new(text: String, line: usize, column: usize) -> Self {
        Token { text, line, column }
    }
}

impl PartialEq<&str> for Token {
    fn eq(&self, other: &&str) -> bool {
        self.text == *other
    }
}

impl PartialEq<str> for Token {
    fn eq(&self, other: &str) -> bool {
        self.text == other
    }
}

pub(super) fn annotate_error_with_line(msg: String, tok: Option<&Token>) -> String {
    if msg.starts_with("at line ") {
        return msg;
    }
    let line = tok.map(|t| t.line).unwrap_or(0);
    format!("at line {}: {}", line + 1, msg)
}

/// Check if a token looks like a float literal
///
/// Float literals contain either:
/// - A decimal point: `3.14`, `.5`, `5.`
/// - Scientific notation: `1e10`, `1E-5`, `1.5e3`
///
/// This check must happen BEFORE integer parsing to avoid
/// parsing "5" in "5.0" as an integer.
pub(super) fn is_float_literal(token: &str) -> bool {
    // Skip leading minus sign for negative numbers
    let s = token.strip_prefix('-').unwrap_or(token);

    // Must have at least one digit
    if s.is_empty() {
        return false;
    }

    // Check for decimal point or scientific notation
    s.contains('.') || s.contains('e') || s.contains('E')
}

/// Process escape sequences in a string literal
///
/// Supported escape sequences:
/// - `\"` -> `"`  (quote)
/// - `\\` -> `\`  (backslash)
/// - `\n` -> newline
/// - `\r` -> carriage return
/// - `\t` -> tab
/// - `\xNN` -> Unicode code point U+00NN (hex value 00-FF)
///
/// # Note on `\xNN` encoding
///
/// The `\xNN` escape creates a Unicode code point U+00NN, not a raw byte.
/// For values 0x00-0x7F (ASCII), this maps directly to the byte value.
/// For values 0x80-0xFF (Latin-1 Supplement), the character is stored as
/// a multi-byte UTF-8 sequence. For example:
/// - `\x41` -> 'A' (1 byte in UTF-8)
/// - `\x1b` -> ESC (1 byte in UTF-8, used for ANSI terminal codes)
/// - `\xFF` -> 'ÿ' (U+00FF, 2 bytes in UTF-8: 0xC3 0xBF)
///
/// This matches Python 3 and Rust string behavior. For terminal ANSI codes,
/// which are the primary use case, all values are in the ASCII range.
///
/// # Errors
/// Returns error if an unknown escape sequence is encountered
pub(super) fn unescape_string(s: &str) -> Result<String, String> {
    let mut result = String::new();
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('x') => {
                    // Hex escape: \xNN
                    let hex1 = chars.next().ok_or_else(|| {
                        "Incomplete hex escape sequence '\\x' - expected 2 hex digits".to_string()
                    })?;
                    let hex2 = chars.next().ok_or_else(|| {
                        format!(
                            "Incomplete hex escape sequence '\\x{}' - expected 2 hex digits",
                            hex1
                        )
                    })?;

                    let hex_str: String = [hex1, hex2].iter().collect();
                    let byte_val = u8::from_str_radix(&hex_str, 16).map_err(|_| {
                        format!(
                            "Invalid hex escape sequence '\\x{}' - expected 2 hex digits (00-FF)",
                            hex_str
                        )
                    })?;

                    result.push(byte_val as char);
                }
                Some(c) => {
                    return Err(format!(
                        "Unknown escape sequence '\\{}' in string literal. \
                         Supported: \\\" \\\\ \\n \\r \\t \\xNN",
                        c
                    ));
                }
                None => {
                    return Err("String ends with incomplete escape sequence '\\'".to_string());
                }
            }
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

pub(super) fn tokenize(source: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut current_start_line = 0;
    let mut current_start_col = 0;
    let mut in_string = false;
    let mut prev_was_backslash = false;

    // Track current position (0-indexed)
    let mut line = 0;
    let mut col = 0;

    for ch in source.chars() {
        if in_string {
            current.push(ch);
            if ch == '"' && !prev_was_backslash {
                // Unescaped quote ends the string
                in_string = false;
                tokens.push(Token::new(
                    current.clone(),
                    current_start_line,
                    current_start_col,
                ));
                current.clear();
                prev_was_backslash = false;
            } else if ch == '\\' && !prev_was_backslash {
                // Start of escape sequence
                prev_was_backslash = true;
            } else {
                // Regular character or escaped character
                prev_was_backslash = false;
            }
            // Track newlines inside strings
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        } else if ch == '"' {
            if !current.is_empty() {
                tokens.push(Token::new(
                    current.clone(),
                    current_start_line,
                    current_start_col,
                ));
                current.clear();
            }
            in_string = true;
            current_start_line = line;
            current_start_col = col;
            current.push(ch);
            prev_was_backslash = false;
            col += 1;
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(Token::new(
                    current.clone(),
                    current_start_line,
                    current_start_col,
                ));
                current.clear();
            }
            // Preserve newlines for comment handling
            if ch == '\n' {
                tokens.push(Token::new("\n".to_string(), line, col));
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        } else if "():;[]{},".contains(ch) {
            if !current.is_empty() {
                tokens.push(Token::new(
                    current.clone(),
                    current_start_line,
                    current_start_col,
                ));
                current.clear();
            }
            tokens.push(Token::new(ch.to_string(), line, col));
            col += 1;
        } else {
            if current.is_empty() {
                current_start_line = line;
                current_start_col = col;
            }
            current.push(ch);
            col += 1;
        }
    }

    // Check for unclosed string literal
    if in_string {
        // Return error by adding a special error token
        // The parser will handle this as a parse error
        tokens.push(Token::new(
            "<<<UNCLOSED_STRING>>>".to_string(),
            current_start_line,
            current_start_col,
        ));
    } else if !current.is_empty() {
        tokens.push(Token::new(current, current_start_line, current_start_col));
    }

    tokens
}
