use super::*;
use crate::ast::{Program, Statement, WordDef};
use std::path::PathBuf;

fn test_config() -> LintConfig {
    LintConfig::from_toml(
        r#"
[[lint]]
id = "redundant-dup-drop"
pattern = "dup drop"
replacement = ""
message = "`dup drop` has no effect"
severity = "warning"

[[lint]]
id = "prefer-nip"
pattern = "swap drop"
replacement = "nip"
message = "prefer `nip` over `swap drop`"
severity = "hint"

[[lint]]
id = "redundant-swap-swap"
pattern = "swap swap"
replacement = ""
message = "consecutive swaps cancel out"
severity = "warning"
"#,
    )
    .unwrap()
}

#[test]
fn test_parse_config() {
    let config = test_config();
    assert_eq!(config.rules.len(), 3);
    assert_eq!(config.rules[0].id, "redundant-dup-drop");
    assert_eq!(config.rules[1].severity, Severity::Hint);
}

#[test]
fn test_compile_pattern() {
    let rule = LintRule {
        id: "test".to_string(),
        pattern: "swap drop".to_string(),
        replacement: "nip".to_string(),
        message: "test".to_string(),
        severity: Severity::Warning,
    };
    let compiled = CompiledPattern::compile(rule).unwrap();
    assert_eq!(compiled.elements.len(), 2);
    assert_eq!(
        compiled.elements[0],
        PatternElement::Word("swap".to_string())
    );
    assert_eq!(
        compiled.elements[1],
        PatternElement::Word("drop".to_string())
    );
}

#[test]
fn test_compile_pattern_with_wildcards() {
    let rule = LintRule {
        id: "test".to_string(),
        pattern: "dup $X drop".to_string(),
        replacement: "".to_string(),
        message: "test".to_string(),
        severity: Severity::Warning,
    };
    let compiled = CompiledPattern::compile(rule).unwrap();
    assert_eq!(compiled.elements.len(), 3);
    assert_eq!(
        compiled.elements[1],
        PatternElement::SingleWildcard("$X".to_string())
    );
}

#[test]
fn test_simple_match() {
    let config = test_config();
    let linter = Linter::new(&config).unwrap();

    // Create a simple program with "swap drop"
    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![WordDef {
            name: "test".to_string(),
            effect: None,
            body: vec![
                Statement::IntLiteral(1),
                Statement::IntLiteral(2),
                Statement::WordCall {
                    name: "swap".to_string(),
                    span: None,
                },
                Statement::WordCall {
                    name: "drop".to_string(),
                    span: None,
                },
            ],
            source: None,
            allowed_lints: vec![],
        }],
    };

    let diagnostics = linter.lint_program(&program, &PathBuf::from("test.seq"));
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "prefer-nip");
    assert_eq!(diagnostics[0].replacement, "nip");
}

#[test]
fn test_no_false_positives() {
    let config = test_config();
    let linter = Linter::new(&config).unwrap();

    // "swap" followed by something other than "drop"
    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![WordDef {
            name: "test".to_string(),
            effect: None,
            body: vec![
                Statement::WordCall {
                    name: "swap".to_string(),
                    span: None,
                },
                Statement::WordCall {
                    name: "dup".to_string(),
                    span: None,
                },
            ],
            source: None,
            allowed_lints: vec![],
        }],
    };

    let diagnostics = linter.lint_program(&program, &PathBuf::from("test.seq"));
    assert!(diagnostics.is_empty());
}

#[test]
fn test_multiple_matches() {
    let config = test_config();
    let linter = Linter::new(&config).unwrap();

    // Two instances of "swap drop"
    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![WordDef {
            name: "test".to_string(),
            effect: None,
            body: vec![
                Statement::WordCall {
                    name: "swap".to_string(),
                    span: None,
                },
                Statement::WordCall {
                    name: "drop".to_string(),
                    span: None,
                },
                Statement::WordCall {
                    name: "dup".to_string(),
                    span: None,
                },
                Statement::WordCall {
                    name: "swap".to_string(),
                    span: None,
                },
                Statement::WordCall {
                    name: "drop".to_string(),
                    span: None,
                },
            ],
            source: None,
            allowed_lints: vec![],
        }],
    };

    let diagnostics = linter.lint_program(&program, &PathBuf::from("test.seq"));
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn test_multi_wildcard_validation() {
    // Pattern with two multi-wildcards should be rejected
    let rule = LintRule {
        id: "bad-pattern".to_string(),
        pattern: "$... foo $...".to_string(),
        replacement: "".to_string(),
        message: "test".to_string(),
        severity: Severity::Warning,
    };
    let result = CompiledPattern::compile(rule);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("multi-wildcards"));
}

#[test]
fn test_single_multi_wildcard_allowed() {
    // Pattern with one multi-wildcard should be accepted
    let rule = LintRule {
        id: "ok-pattern".to_string(),
        pattern: "$... foo".to_string(),
        replacement: "".to_string(),
        message: "test".to_string(),
        severity: Severity::Warning,
    };
    let result = CompiledPattern::compile(rule);
    assert!(result.is_ok());
}

#[test]
fn test_literal_breaks_pattern() {
    // "swap 0 swap" should NOT match "swap swap" because the literal breaks the pattern
    let config = test_config();
    let linter = Linter::new(&config).unwrap();

    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![WordDef {
            name: "test".to_string(),
            effect: None,
            body: vec![
                Statement::WordCall {
                    name: "swap".to_string(),
                    span: None,
                },
                Statement::IntLiteral(0), // This should break the pattern
                Statement::WordCall {
                    name: "swap".to_string(),
                    span: None,
                },
            ],
            source: None,
            allowed_lints: vec![],
        }],
    };

    let diagnostics = linter.lint_program(&program, &PathBuf::from("test.seq"));
    // Should NOT find "swap swap" because there's a literal in between
    assert!(
        diagnostics.is_empty(),
        "Expected no matches, but got: {:?}",
        diagnostics
    );
}

#[test]
fn test_consecutive_swap_swap_still_matches() {
    // Actual consecutive "swap swap" should still be detected
    let config = test_config();
    let linter = Linter::new(&config).unwrap();

    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![WordDef {
            name: "test".to_string(),
            effect: None,
            body: vec![
                Statement::WordCall {
                    name: "swap".to_string(),
                    span: None,
                },
                Statement::WordCall {
                    name: "swap".to_string(),
                    span: None,
                },
            ],
            source: None,
            allowed_lints: vec![],
        }],
    };

    let diagnostics = linter.lint_program(&program, &PathBuf::from("test.seq"));
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "redundant-swap-swap");
}
