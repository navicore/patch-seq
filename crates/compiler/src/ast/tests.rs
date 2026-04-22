use super::*;

#[test]
fn test_validate_builtin_words() {
    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![WordDef {
            name: "main".to_string(),
            effect: None,
            body: vec![
                Statement::IntLiteral(2),
                Statement::IntLiteral(3),
                Statement::WordCall {
                    name: "i.add".to_string(),
                    span: None,
                },
                Statement::WordCall {
                    name: "io.write-line".to_string(),
                    span: None,
                },
            ],
            source: None,
            allowed_lints: vec![],
        }],
    };

    // Should succeed - i.add and io.write-line are built-ins
    assert!(program.validate_word_calls().is_ok());
}

#[test]
fn test_validate_user_defined_words() {
    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![
            WordDef {
                name: "helper".to_string(),
                effect: None,
                body: vec![Statement::IntLiteral(42)],
                source: None,
                allowed_lints: vec![],
            },
            WordDef {
                name: "main".to_string(),
                effect: None,
                body: vec![Statement::WordCall {
                    name: "helper".to_string(),
                    span: None,
                }],
                source: None,
                allowed_lints: vec![],
            },
        ],
    };

    // Should succeed - helper is defined
    assert!(program.validate_word_calls().is_ok());
}

#[test]
fn test_validate_undefined_word() {
    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![WordDef {
            name: "main".to_string(),
            effect: None,
            body: vec![Statement::WordCall {
                name: "undefined_word".to_string(),
                span: None,
            }],
            source: None,
            allowed_lints: vec![],
        }],
    };

    // Should fail - undefined_word is not a built-in or user-defined word
    let result = program.validate_word_calls();
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("undefined_word"));
    assert!(error.contains("main"));
}

#[test]
fn test_validate_misspelled_builtin() {
    let program = Program {
        includes: vec![],
        unions: vec![],
        words: vec![WordDef {
            name: "main".to_string(),
            effect: None,
            body: vec![Statement::WordCall {
                name: "wrte_line".to_string(),
                span: None,
            }], // typo
            source: None,
            allowed_lints: vec![],
        }],
    };

    // Should fail with helpful message
    let result = program.validate_word_calls();
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("wrte_line"));
    assert!(error.contains("misspell"));
}

#[test]
fn test_generate_constructors() {
    let mut program = Program {
        includes: vec![],
        unions: vec![UnionDef {
            name: "Message".to_string(),
            variants: vec![
                UnionVariant {
                    name: "Get".to_string(),
                    fields: vec![UnionField {
                        name: "response-chan".to_string(),
                        type_name: "Int".to_string(),
                    }],
                    source: None,
                },
                UnionVariant {
                    name: "Put".to_string(),
                    fields: vec![
                        UnionField {
                            name: "value".to_string(),
                            type_name: "String".to_string(),
                        },
                        UnionField {
                            name: "response-chan".to_string(),
                            type_name: "Int".to_string(),
                        },
                    ],
                    source: None,
                },
            ],
            source: None,
        }],
        words: vec![],
    };

    // Generate constructors, predicates, and accessors
    program.generate_constructors().unwrap();

    // Should have 7 words:
    // - Get variant: Make-Get, is-Get?, Get-response-chan (1 field)
    // - Put variant: Make-Put, is-Put?, Put-value, Put-response-chan (2 fields)
    assert_eq!(program.words.len(), 7);

    // Check Make-Get constructor
    let make_get = program
        .find_word("Make-Get")
        .expect("Make-Get should exist");
    assert_eq!(make_get.name, "Make-Get");
    assert!(make_get.effect.is_some());
    let effect = make_get.effect.as_ref().unwrap();
    // Input: ( ..a Int -- )
    // Output: ( ..a Message -- )
    assert_eq!(
        format!("{:?}", effect.outputs),
        "Cons { rest: RowVar(\"a\"), top: Union(\"Message\") }"
    );

    // Check Make-Put constructor
    let make_put = program
        .find_word("Make-Put")
        .expect("Make-Put should exist");
    assert_eq!(make_put.name, "Make-Put");
    assert!(make_put.effect.is_some());

    // Check the body generates correct code
    // Make-Get should be: :Get variant.make-1
    assert_eq!(make_get.body.len(), 2);
    match &make_get.body[0] {
        Statement::Symbol(s) if s == "Get" => {}
        other => panic!("Expected Symbol(\"Get\") for variant tag, got {:?}", other),
    }
    match &make_get.body[1] {
        Statement::WordCall { name, span: None } if name == "variant.make-1" => {}
        _ => panic!("Expected WordCall(variant.make-1)"),
    }

    // Make-Put should be: :Put variant.make-2
    assert_eq!(make_put.body.len(), 2);
    match &make_put.body[0] {
        Statement::Symbol(s) if s == "Put" => {}
        other => panic!("Expected Symbol(\"Put\") for variant tag, got {:?}", other),
    }
    match &make_put.body[1] {
        Statement::WordCall { name, span: None } if name == "variant.make-2" => {}
        _ => panic!("Expected WordCall(variant.make-2)"),
    }

    // Check is-Get? predicate
    let is_get = program.find_word("is-Get?").expect("is-Get? should exist");
    assert_eq!(is_get.name, "is-Get?");
    assert!(is_get.effect.is_some());
    let effect = is_get.effect.as_ref().unwrap();
    // Input: ( ..a Message -- )
    // Output: ( ..a Bool -- )
    assert_eq!(
        format!("{:?}", effect.outputs),
        "Cons { rest: RowVar(\"a\"), top: Bool }"
    );

    // Check Get-response-chan accessor
    let get_chan = program
        .find_word("Get-response-chan")
        .expect("Get-response-chan should exist");
    assert_eq!(get_chan.name, "Get-response-chan");
    assert!(get_chan.effect.is_some());
    let effect = get_chan.effect.as_ref().unwrap();
    // Input: ( ..a Message -- )
    // Output: ( ..a Int -- )
    assert_eq!(
        format!("{:?}", effect.outputs),
        "Cons { rest: RowVar(\"a\"), top: Int }"
    );
}
