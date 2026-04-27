use super::*;
use crate::ast::{Span, Statement, WordDef};
use crate::types::{Effect, StackType};
use std::path::Path;

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
            Statement::StringLiteral(b"foo".to_vec()),
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
            Statement::StringLiteral(b"foo".to_vec()),
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
            Statement::StringLiteral(b"foo".to_vec()),
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
            Statement::StringLiteral(b"foo".to_vec()),
            word_call("file.slurp", 1),
            word_call(">aux", 5),
            Statement::StringLiteral(b"other work".to_vec()),
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
            Statement::StringLiteral(b"42".to_vec()),
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
            Statement::StringLiteral(b"42".to_vec()),
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
            Statement::StringLiteral(b"foo".to_vec()),
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
            Statement::StringLiteral(b"foo".to_vec()),
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
            Statement::StringLiteral(b"foo".to_vec()),
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
