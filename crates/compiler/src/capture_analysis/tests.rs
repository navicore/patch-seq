use super::*;
use crate::types::{Effect, StackType, Type};

fn make_stack(types: &[Type]) -> StackType {
    let mut stack = StackType::Empty;
    for t in types {
        stack = StackType::Cons {
            rest: Box::new(stack),
            top: t.clone(),
        };
    }
    stack
}

fn make_effect(inputs: &[Type], outputs: &[Type]) -> Effect {
    Effect {
        inputs: make_stack(inputs),
        outputs: make_stack(outputs),
        effects: Vec::new(),
    }
}

#[test]
fn test_extract_empty_stack() {
    let types = extract_concrete_types(&StackType::Empty);
    assert!(types.is_empty());
}

#[test]
fn test_extract_single_type() {
    let stack = make_stack(&[Type::Int]);
    let types = extract_concrete_types(&stack);
    assert_eq!(types, vec![Type::Int]);
}

#[test]
fn test_extract_multiple_types() {
    let stack = make_stack(&[Type::Int, Type::String, Type::Bool]);
    let types = extract_concrete_types(&stack);
    assert_eq!(types, vec![Type::Int, Type::String, Type::Bool]);
}

#[test]
fn test_calculate_no_captures() {
    // Body needs (Int -- Int), call provides (Int -- Int)
    let body = make_effect(&[Type::Int], &[Type::Int]);
    let call = make_effect(&[Type::Int], &[Type::Int]);

    let captures = calculate_captures(&body, &call).unwrap();
    assert!(captures.is_empty());
}

#[test]
fn test_calculate_one_capture() {
    // Body needs (Int Int -- Int), call provides (Int -- Int)
    // Should capture one Int
    let body = make_effect(&[Type::Int, Type::Int], &[Type::Int]);
    let call = make_effect(&[Type::Int], &[Type::Int]);

    let captures = calculate_captures(&body, &call).unwrap();
    assert_eq!(captures, vec![Type::Int]);
}

#[test]
fn test_calculate_multiple_captures() {
    // Body needs (Int String Bool -- Bool), call provides (Bool -- Bool)
    // Should capture [Int, String] (bottom to top)
    let body = make_effect(&[Type::Int, Type::String, Type::Bool], &[Type::Bool]);
    let call = make_effect(&[Type::Bool], &[Type::Bool]);

    let captures = calculate_captures(&body, &call).unwrap();
    assert_eq!(captures, vec![Type::Int, Type::String]);
}

#[test]
fn test_calculate_all_captured() {
    // Body needs (Int String -- Int), call provides ( -- Int)
    // Should capture [Int, String]
    let body = make_effect(&[Type::Int, Type::String], &[Type::Int]);
    let call = make_effect(&[], &[Type::Int]);

    let captures = calculate_captures(&body, &call).unwrap();
    assert_eq!(captures, vec![Type::Int, Type::String]);
}

#[test]
fn test_calculate_error_too_many_call_inputs() {
    // Body needs (Int -- Int), call provides (Int Int -- Int)
    // Error: call provides more than body needs
    let body = make_effect(&[Type::Int], &[Type::Int]);
    let call = make_effect(&[Type::Int, Type::Int], &[Type::Int]);

    let result = calculate_captures(&body, &call);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("provides 2 values but body only needs 1")
    );
}
