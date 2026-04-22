use super::*;

#[test]
fn test_empty_stack() {
    let stack = StackType::empty();
    assert_eq!(stack, StackType::Empty);
}

#[test]
fn test_singleton_stack() {
    let stack = StackType::singleton(Type::Int);
    assert_eq!(
        stack,
        StackType::Cons {
            rest: Box::new(StackType::Empty),
            top: Type::Int
        }
    );
}

#[test]
fn test_push_pop() {
    let stack = StackType::empty().push(Type::Int).push(Type::Bool);

    let (rest, top) = stack.pop().unwrap();
    assert_eq!(top, Type::Bool);

    let (rest2, top2) = rest.pop().unwrap();
    assert_eq!(top2, Type::Int);
    assert_eq!(rest2, StackType::Empty);
}

#[test]
fn test_from_vec() {
    let stack = StackType::from_vec(vec![Type::Int, Type::Bool, Type::String]);

    // Stack should be: String on top of Bool on top of Int on top of Empty
    let (rest, top) = stack.pop().unwrap();
    assert_eq!(top, Type::String);

    let (rest2, top2) = rest.pop().unwrap();
    assert_eq!(top2, Type::Bool);

    let (rest3, top3) = rest2.pop().unwrap();
    assert_eq!(top3, Type::Int);
    assert_eq!(rest3, StackType::Empty);
}

#[test]
fn test_row_variable() {
    let stack = StackType::Cons {
        rest: Box::new(StackType::RowVar("a".to_string())),
        top: Type::Int,
    };

    // This represents: Int on top of ..a
    let (rest, top) = stack.pop().unwrap();
    assert_eq!(top, Type::Int);
    assert_eq!(rest, StackType::RowVar("a".to_string()));
}

#[test]
fn test_effect() {
    // Effect: ( Int -- Bool )
    let effect = Effect::new(
        StackType::singleton(Type::Int),
        StackType::singleton(Type::Bool),
    );

    assert_eq!(effect.inputs, StackType::singleton(Type::Int));
    assert_eq!(effect.outputs, StackType::singleton(Type::Bool));
}

#[test]
fn test_polymorphic_effect() {
    // Effect: ( ..a Int -- ..a Bool )
    let inputs = StackType::Cons {
        rest: Box::new(StackType::RowVar("a".to_string())),
        top: Type::Int,
    };

    let outputs = StackType::Cons {
        rest: Box::new(StackType::RowVar("a".to_string())),
        top: Type::Bool,
    };

    let effect = Effect::new(inputs, outputs);

    // Verify structure
    assert!(matches!(effect.inputs, StackType::Cons { .. }));
    assert!(matches!(effect.outputs, StackType::Cons { .. }));
}
