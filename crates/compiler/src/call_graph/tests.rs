use super::*;
use crate::ast::WordDef;

fn make_word(name: &str, calls: Vec<&str>) -> WordDef {
    let body = calls
        .into_iter()
        .map(|c| Statement::WordCall {
            name: c.to_string(),
            span: None,
        })
        .collect();
    WordDef {
        name: name.to_string(),
        effect: None,
        body,
        source: None,
        allowed_lints: vec![],
    }
}

#[test]
fn test_no_recursion() {
    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![
            make_word("foo", vec!["bar"]),
            make_word("bar", vec![]),
            make_word("baz", vec!["foo"]),
        ],
    };

    let graph = CallGraph::build(&program);
    assert!(!graph.is_recursive("foo"));
    assert!(!graph.is_recursive("bar"));
    assert!(!graph.is_recursive("baz"));
    assert!(graph.recursive_cycles().is_empty());
}

#[test]
fn test_direct_recursion() {
    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![
            make_word("countdown", vec!["countdown"]),
            make_word("helper", vec![]),
        ],
    };

    let graph = CallGraph::build(&program);
    assert!(graph.is_recursive("countdown"));
    assert!(!graph.is_recursive("helper"));
    assert_eq!(graph.recursive_cycles().len(), 1);
}

#[test]
fn test_mutual_recursion_pair() {
    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![
            make_word("ping", vec!["pong"]),
            make_word("pong", vec!["ping"]),
        ],
    };

    let graph = CallGraph::build(&program);
    assert!(graph.is_recursive("ping"));
    assert!(graph.is_recursive("pong"));
    assert!(graph.are_mutually_recursive("ping", "pong"));
    assert_eq!(graph.recursive_cycles().len(), 1);
    assert_eq!(graph.recursive_cycles()[0].len(), 2);
}

#[test]
fn test_mutual_recursion_triple() {
    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![
            make_word("a", vec!["b"]),
            make_word("b", vec!["c"]),
            make_word("c", vec!["a"]),
        ],
    };

    let graph = CallGraph::build(&program);
    assert!(graph.is_recursive("a"));
    assert!(graph.is_recursive("b"));
    assert!(graph.is_recursive("c"));
    assert!(graph.are_mutually_recursive("a", "b"));
    assert!(graph.are_mutually_recursive("b", "c"));
    assert!(graph.are_mutually_recursive("a", "c"));
    assert_eq!(graph.recursive_cycles().len(), 1);
    assert_eq!(graph.recursive_cycles()[0].len(), 3);
}

#[test]
fn test_multiple_independent_cycles() {
    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![
            // Cycle 1: ping <-> pong
            make_word("ping", vec!["pong"]),
            make_word("pong", vec!["ping"]),
            // Cycle 2: even <-> odd
            make_word("even", vec!["odd"]),
            make_word("odd", vec!["even"]),
            // Non-recursive
            make_word("main", vec!["ping", "even"]),
        ],
    };

    let graph = CallGraph::build(&program);
    assert!(graph.is_recursive("ping"));
    assert!(graph.is_recursive("pong"));
    assert!(graph.is_recursive("even"));
    assert!(graph.is_recursive("odd"));
    assert!(!graph.is_recursive("main"));

    assert!(graph.are_mutually_recursive("ping", "pong"));
    assert!(graph.are_mutually_recursive("even", "odd"));
    assert!(!graph.are_mutually_recursive("ping", "even"));

    assert_eq!(graph.recursive_cycles().len(), 2);
}

#[test]
fn test_calls_to_unknown_words() {
    // Calls to builtins or external words should be ignored
    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![make_word("foo", vec!["dup", "drop", "unknown_builtin"])],
    };

    let graph = CallGraph::build(&program);
    assert!(!graph.is_recursive("foo"));
    // Callees should only include known words
    assert!(graph.callees("foo").unwrap().is_empty());
}

#[test]
fn test_cycle_with_builtins_interspersed() {
    // Cycles should be detected even when builtins are called between user words
    // e.g., : foo dup drop bar ;  : bar swap foo ;
    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![
            make_word("foo", vec!["dup", "drop", "bar"]),
            make_word("bar", vec!["swap", "foo"]),
        ],
    };

    let graph = CallGraph::build(&program);
    // foo and bar should still form a cycle despite builtin calls
    assert!(graph.is_recursive("foo"));
    assert!(graph.is_recursive("bar"));
    assert!(graph.are_mutually_recursive("foo", "bar"));

    // Builtins should not appear in callees
    let foo_callees = graph.callees("foo").unwrap();
    assert!(foo_callees.contains("bar"));
    assert!(!foo_callees.contains("dup"));
    assert!(!foo_callees.contains("drop"));
}

#[test]
fn test_cycle_through_quotation() {
    // Calls inside quotations should be detected
    // e.g., : foo [ bar ] call ;  : bar foo ;
    use crate::ast::Statement;

    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![
            WordDef {
                name: "foo".to_string(),
                effect: None,
                body: vec![
                    Statement::Quotation {
                        id: 0,
                        body: vec![Statement::WordCall {
                            name: "bar".to_string(),
                            span: None,
                        }],
                        span: None,
                    },
                    Statement::WordCall {
                        name: "call".to_string(),
                        span: None,
                    },
                ],
                source: None,
                allowed_lints: vec![],
            },
            make_word("bar", vec!["foo"]),
        ],
    };

    let graph = CallGraph::build(&program);
    // foo calls bar (inside quotation), bar calls foo
    assert!(graph.is_recursive("foo"));
    assert!(graph.is_recursive("bar"));
    assert!(graph.are_mutually_recursive("foo", "bar"));
}

#[test]
fn test_cycle_through_if_branch() {
    // Calls inside if branches should be detected
    use crate::ast::Statement;

    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![
            WordDef {
                name: "even".to_string(),
                effect: None,
                body: vec![Statement::If {
                    then_branch: vec![],
                    else_branch: Some(vec![Statement::WordCall {
                        name: "odd".to_string(),
                        span: None,
                    }]),
                    span: None,
                }],
                source: None,
                allowed_lints: vec![],
            },
            WordDef {
                name: "odd".to_string(),
                effect: None,
                body: vec![Statement::If {
                    then_branch: vec![],
                    else_branch: Some(vec![Statement::WordCall {
                        name: "even".to_string(),
                        span: None,
                    }]),
                    span: None,
                }],
                source: None,
                allowed_lints: vec![],
            },
        ],
    };

    let graph = CallGraph::build(&program);
    assert!(graph.is_recursive("even"));
    assert!(graph.is_recursive("odd"));
    assert!(graph.are_mutually_recursive("even", "odd"));
}
