use super::*;
use crate::ast::{Span, Statement, WordDef};
use std::path::Path;

fn make_word_call(name: &str) -> Statement {
    Statement::WordCall {
        name: name.to_string(),
        span: Some(Span::new(0, 0, name.len())),
    }
}

#[test]
fn test_immediate_weave_drop() {
    // : bad ( -- ) [ gen ] strand.weave drop ;
    let word = WordDef {
        name: "bad".to_string(),
        effect: None,
        body: vec![
            Statement::Quotation {
                span: None,
                id: 0,
                body: vec![make_word_call("gen")],
            },
            make_word_call("strand.weave"),
            make_word_call("drop"),
        ],
        source: None,
        allowed_lints: vec![],
    };

    let mut analyzer = ResourceAnalyzer::new(Path::new("test.seq"));
    let diagnostics = analyzer.analyze_word(&word);

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].id.contains("weavehandle"));
    assert!(diagnostics[0].message.contains("dropped without cleanup"));
}

#[test]
fn test_weave_properly_cancelled() {
    // : good ( -- ) [ gen ] strand.weave strand.weave-cancel ;
    let word = WordDef {
        name: "good".to_string(),
        effect: None,
        body: vec![
            Statement::Quotation {
                span: None,
                id: 0,
                body: vec![make_word_call("gen")],
            },
            make_word_call("strand.weave"),
            make_word_call("strand.weave-cancel"),
        ],
        source: None,
        allowed_lints: vec![],
    };

    let mut analyzer = ResourceAnalyzer::new(Path::new("test.seq"));
    let diagnostics = analyzer.analyze_word(&word);

    assert!(
        diagnostics.is_empty(),
        "Expected no warnings for properly cancelled weave"
    );
}

#[test]
fn test_branch_inconsistent_handling() {
    // : bad ( -- )
    //   [ gen ] strand.weave
    //   true if strand.weave-cancel else drop then ;
    let word = WordDef {
        name: "bad".to_string(),
        effect: None,
        body: vec![
            Statement::Quotation {
                span: None,
                id: 0,
                body: vec![make_word_call("gen")],
            },
            make_word_call("strand.weave"),
            Statement::BoolLiteral(true),
            Statement::If {
                then_branch: vec![make_word_call("strand.weave-cancel")],
                else_branch: Some(vec![make_word_call("drop")]),
                span: None,
            },
        ],
        source: None,
        allowed_lints: vec![],
    };

    let mut analyzer = ResourceAnalyzer::new(Path::new("test.seq"));
    let diagnostics = analyzer.analyze_word(&word);

    // Should warn about drop in else branch
    assert!(!diagnostics.is_empty());
}

#[test]
fn test_both_branches_cancel() {
    // : good ( -- )
    //   [ gen ] strand.weave
    //   true if strand.weave-cancel else strand.weave-cancel then ;
    let word = WordDef {
        name: "good".to_string(),
        effect: None,
        body: vec![
            Statement::Quotation {
                span: None,
                id: 0,
                body: vec![make_word_call("gen")],
            },
            make_word_call("strand.weave"),
            Statement::BoolLiteral(true),
            Statement::If {
                then_branch: vec![make_word_call("strand.weave-cancel")],
                else_branch: Some(vec![make_word_call("strand.weave-cancel")]),
                span: None,
            },
        ],
        source: None,
        allowed_lints: vec![],
    };

    let mut analyzer = ResourceAnalyzer::new(Path::new("test.seq"));
    let diagnostics = analyzer.analyze_word(&word);

    assert!(
        diagnostics.is_empty(),
        "Expected no warnings when both branches cancel"
    );
}

#[test]
fn test_channel_leak() {
    // : bad ( -- ) chan.make drop ;
    let word = WordDef {
        name: "bad".to_string(),
        effect: None,
        body: vec![make_word_call("chan.make"), make_word_call("drop")],
        source: None,
        allowed_lints: vec![],
    };

    let mut analyzer = ResourceAnalyzer::new(Path::new("test.seq"));
    let diagnostics = analyzer.analyze_word(&word);

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].id.contains("channel"));
}

#[test]
fn test_channel_properly_closed() {
    // : good ( -- ) chan.make chan.close ;
    let word = WordDef {
        name: "good".to_string(),
        effect: None,
        body: vec![make_word_call("chan.make"), make_word_call("chan.close")],
        source: None,
        allowed_lints: vec![],
    };

    let mut analyzer = ResourceAnalyzer::new(Path::new("test.seq"));
    let diagnostics = analyzer.analyze_word(&word);

    assert!(
        diagnostics.is_empty(),
        "Expected no warnings for properly closed channel"
    );
}

#[test]
fn test_swap_resource_tracking() {
    // : test ( -- ) chan.make 1 swap drop drop ;
    // After swap: chan is on top, 1 is second
    // First drop removes chan (should warn), second drop removes 1
    let word = WordDef {
        name: "test".to_string(),
        effect: None,
        body: vec![
            make_word_call("chan.make"),
            Statement::IntLiteral(1),
            make_word_call("swap"),
            make_word_call("drop"), // drops chan - should warn
            make_word_call("drop"), // drops 1
        ],
        source: None,
        allowed_lints: vec![],
    };

    let mut analyzer = ResourceAnalyzer::new(Path::new("test.seq"));
    let diagnostics = analyzer.analyze_word(&word);

    assert_eq!(
        diagnostics.len(),
        1,
        "Expected warning for dropped channel: {:?}",
        diagnostics
    );
    assert!(diagnostics[0].id.contains("channel"));
}

#[test]
fn test_over_resource_tracking() {
    // : test ( -- ) chan.make 1 over drop drop drop ;
    // Stack after chan.make: (chan)
    // Stack after 1: (chan 1)
    // Stack after over: (chan 1 chan) - chan copied to top
    // Both chan references are dropped without cleanup - both warn
    let word = WordDef {
        name: "test".to_string(),
        effect: None,
        body: vec![
            make_word_call("chan.make"),
            Statement::IntLiteral(1),
            make_word_call("over"),
            make_word_call("drop"), // drops copied chan - warns
            make_word_call("drop"), // drops 1
            make_word_call("drop"), // drops original chan - also warns
        ],
        source: None,
        allowed_lints: vec![],
    };

    let mut analyzer = ResourceAnalyzer::new(Path::new("test.seq"));
    let diagnostics = analyzer.analyze_word(&word);

    // Both channel drops warn (they share ID but neither was properly consumed)
    assert_eq!(
        diagnostics.len(),
        2,
        "Expected 2 warnings for dropped channels: {:?}",
        diagnostics
    );
}

#[test]
fn test_channel_transferred_via_spawn() {
    // Pattern from shopping-cart: channel transferred to spawned worker
    // : accept-loop ( -- )
    //   chan.make                  # create channel
    //   dup [ worker ] strand.spawn  # transfer to worker
    //   drop drop                  # drop strand-id and dup'd chan
    //   chan.send                  # use remaining chan
    // ;
    let word = WordDef {
        name: "accept-loop".to_string(),
        effect: None,
        body: vec![
            make_word_call("chan.make"),
            make_word_call("dup"),
            Statement::Quotation {
                span: None,
                id: 0,
                body: vec![make_word_call("worker")],
            },
            make_word_call("strand.spawn"),
            make_word_call("drop"),
            make_word_call("drop"),
            make_word_call("chan.send"),
        ],
        source: None,
        allowed_lints: vec![],
    };

    let mut analyzer = ResourceAnalyzer::new(Path::new("test.seq"));
    let diagnostics = analyzer.analyze_word(&word);

    assert!(
        diagnostics.is_empty(),
        "Expected no warnings when channel is transferred via strand.spawn: {:?}",
        diagnostics
    );
}

#[test]
fn test_else_branch_only_leak() {
    // : test ( -- )
    //   chan.make
    //   true if chan.close else drop then ;
    // The else branch drops without cleanup - should warn about inconsistency
    // AND the join should track that the resource might not be consumed
    let word = WordDef {
        name: "test".to_string(),
        effect: None,
        body: vec![
            make_word_call("chan.make"),
            Statement::BoolLiteral(true),
            Statement::If {
                then_branch: vec![make_word_call("chan.close")],
                else_branch: Some(vec![make_word_call("drop")]),
                span: None,
            },
        ],
        source: None,
        allowed_lints: vec![],
    };

    let mut analyzer = ResourceAnalyzer::new(Path::new("test.seq"));
    let diagnostics = analyzer.analyze_word(&word);

    // Should have warnings: branch inconsistency + drop without cleanup
    assert!(
        !diagnostics.is_empty(),
        "Expected warnings for else-branch leak: {:?}",
        diagnostics
    );
}

#[test]
fn test_branch_join_both_consume() {
    // : test ( -- )
    //   chan.make
    //   true if chan.close else chan.close then ;
    // Both branches properly consume - no warnings
    let word = WordDef {
        name: "test".to_string(),
        effect: None,
        body: vec![
            make_word_call("chan.make"),
            Statement::BoolLiteral(true),
            Statement::If {
                then_branch: vec![make_word_call("chan.close")],
                else_branch: Some(vec![make_word_call("chan.close")]),
                span: None,
            },
        ],
        source: None,
        allowed_lints: vec![],
    };

    let mut analyzer = ResourceAnalyzer::new(Path::new("test.seq"));
    let diagnostics = analyzer.analyze_word(&word);

    assert!(
        diagnostics.is_empty(),
        "Expected no warnings when both branches consume: {:?}",
        diagnostics
    );
}

#[test]
fn test_branch_join_neither_consume() {
    // : test ( -- )
    //   chan.make
    //   true if else then drop ;
    // Neither branch consumes, then drop after - should warn
    let word = WordDef {
        name: "test".to_string(),
        effect: None,
        body: vec![
            make_word_call("chan.make"),
            Statement::BoolLiteral(true),
            Statement::If {
                then_branch: vec![],
                else_branch: Some(vec![]),
                span: None,
            },
            make_word_call("drop"), // drops the channel
        ],
        source: None,
        allowed_lints: vec![],
    };

    let mut analyzer = ResourceAnalyzer::new(Path::new("test.seq"));
    let diagnostics = analyzer.analyze_word(&word);

    assert_eq!(
        diagnostics.len(),
        1,
        "Expected warning for dropped channel: {:?}",
        diagnostics
    );
    assert!(diagnostics[0].id.contains("channel"));
}

// ========================================================================
// Cross-word analysis tests (ProgramResourceAnalyzer)
// ========================================================================

#[test]
fn test_cross_word_resource_tracking() {
    // Test that resources returned from user-defined words are tracked
    //
    // : make-chan ( -- chan ) chan.make ;
    // : leak-it ( -- ) make-chan drop ;
    //
    // The drop in leak-it should warn because make-chan returns a channel
    use crate::ast::Program;

    let make_chan = WordDef {
        name: "make-chan".to_string(),
        effect: None,
        body: vec![make_word_call("chan.make")],
        source: None,
        allowed_lints: vec![],
    };

    let leak_it = WordDef {
        name: "leak-it".to_string(),
        effect: None,
        body: vec![make_word_call("make-chan"), make_word_call("drop")],
        source: None,
        allowed_lints: vec![],
    };

    let program = Program {
        words: vec![make_chan, leak_it],
        includes: vec![],
        unions: vec![],
    };

    let mut analyzer = ProgramResourceAnalyzer::new(Path::new("test.seq"));
    let diagnostics = analyzer.analyze_program(&program);

    assert_eq!(
        diagnostics.len(),
        1,
        "Expected warning for dropped channel from make-chan: {:?}",
        diagnostics
    );
    assert!(diagnostics[0].id.contains("channel"));
    assert!(diagnostics[0].message.contains("make-chan"));
}

#[test]
fn test_cross_word_proper_cleanup() {
    // Test that properly cleaned up cross-word resources don't warn
    //
    // : make-chan ( -- chan ) chan.make ;
    // : use-it ( -- ) make-chan chan.close ;
    use crate::ast::Program;

    let make_chan = WordDef {
        name: "make-chan".to_string(),
        effect: None,
        body: vec![make_word_call("chan.make")],
        source: None,
        allowed_lints: vec![],
    };

    let use_it = WordDef {
        name: "use-it".to_string(),
        effect: None,
        body: vec![make_word_call("make-chan"), make_word_call("chan.close")],
        source: None,
        allowed_lints: vec![],
    };

    let program = Program {
        words: vec![make_chan, use_it],
        includes: vec![],
        unions: vec![],
    };

    let mut analyzer = ProgramResourceAnalyzer::new(Path::new("test.seq"));
    let diagnostics = analyzer.analyze_program(&program);

    assert!(
        diagnostics.is_empty(),
        "Expected no warnings for properly closed channel: {:?}",
        diagnostics
    );
}

#[test]
fn test_cross_word_chain() {
    // Test multi-level cross-word tracking
    //
    // : make-chan ( -- chan ) chan.make ;
    // : wrap-chan ( -- chan ) make-chan ;
    // : leak-chain ( -- ) wrap-chan drop ;
    use crate::ast::Program;

    let make_chan = WordDef {
        name: "make-chan".to_string(),
        effect: None,
        body: vec![make_word_call("chan.make")],
        source: None,
        allowed_lints: vec![],
    };

    let wrap_chan = WordDef {
        name: "wrap-chan".to_string(),
        effect: None,
        body: vec![make_word_call("make-chan")],
        source: None,
        allowed_lints: vec![],
    };

    let leak_chain = WordDef {
        name: "leak-chain".to_string(),
        effect: None,
        body: vec![make_word_call("wrap-chan"), make_word_call("drop")],
        source: None,
        allowed_lints: vec![],
    };

    let program = Program {
        words: vec![make_chan, wrap_chan, leak_chain],
        includes: vec![],
        unions: vec![],
    };

    let mut analyzer = ProgramResourceAnalyzer::new(Path::new("test.seq"));
    let diagnostics = analyzer.analyze_program(&program);

    // Should detect the leak through the chain
    assert_eq!(
        diagnostics.len(),
        1,
        "Expected warning for dropped channel through chain: {:?}",
        diagnostics
    );
}
