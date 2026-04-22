use super::*;

#[test]
fn test_unify_concrete_types() {
    assert!(unify_types(&Type::Int, &Type::Int).is_ok());
    assert!(unify_types(&Type::Bool, &Type::Bool).is_ok());
    assert!(unify_types(&Type::String, &Type::String).is_ok());

    assert!(unify_types(&Type::Int, &Type::Bool).is_err());
}

#[test]
fn test_unify_type_variable() {
    let subst = unify_types(&Type::Var("T".to_string()), &Type::Int).unwrap();
    assert_eq!(subst.types.get("T"), Some(&Type::Int));

    let subst = unify_types(&Type::Bool, &Type::Var("U".to_string())).unwrap();
    assert_eq!(subst.types.get("U"), Some(&Type::Bool));
}

#[test]
fn test_unify_empty_stacks() {
    assert!(unify_stacks(&StackType::Empty, &StackType::Empty).is_ok());
}

#[test]
fn test_unify_row_variable() {
    let subst = unify_stacks(
        &StackType::RowVar("a".to_string()),
        &StackType::singleton(Type::Int),
    )
    .unwrap();

    assert_eq!(subst.rows.get("a"), Some(&StackType::singleton(Type::Int)));
}

#[test]
fn test_unify_cons_stacks() {
    // ( Int ) unifies with ( Int )
    let s1 = StackType::singleton(Type::Int);
    let s2 = StackType::singleton(Type::Int);

    assert!(unify_stacks(&s1, &s2).is_ok());
}

#[test]
fn test_unify_cons_with_type_var() {
    // ( T ) unifies with ( Int ), producing T := Int
    let s1 = StackType::singleton(Type::Var("T".to_string()));
    let s2 = StackType::singleton(Type::Int);

    let subst = unify_stacks(&s1, &s2).unwrap();
    assert_eq!(subst.types.get("T"), Some(&Type::Int));
}

#[test]
fn test_unify_row_poly_stack() {
    // ( ..a Int ) unifies with ( Bool Int ), producing ..a := ( Bool )
    let s1 = StackType::RowVar("a".to_string()).push(Type::Int);
    let s2 = StackType::Empty.push(Type::Bool).push(Type::Int);

    let subst = unify_stacks(&s1, &s2).unwrap();

    assert_eq!(subst.rows.get("a"), Some(&StackType::singleton(Type::Bool)));
}

#[test]
fn test_unify_polymorphic_dup() {
    // dup: ( ..a T -- ..a T T )
    // Applied to: ( Int ) should work with ..a := Empty, T := Int

    let input_actual = StackType::singleton(Type::Int);
    let input_declared = StackType::RowVar("a".to_string()).push(Type::Var("T".to_string()));

    let subst = unify_stacks(&input_declared, &input_actual).unwrap();

    assert_eq!(subst.rows.get("a"), Some(&StackType::Empty));
    assert_eq!(subst.types.get("T"), Some(&Type::Int));

    // Apply substitution to output: ( ..a T T )
    let output_declared = StackType::RowVar("a".to_string())
        .push(Type::Var("T".to_string()))
        .push(Type::Var("T".to_string()));

    let output_actual = subst.apply_stack(&output_declared);

    // Should be ( Int Int )
    assert_eq!(
        output_actual,
        StackType::Empty.push(Type::Int).push(Type::Int)
    );
}

#[test]
fn test_subst_compose() {
    // s1: T := Int
    let mut s1 = Subst::empty();
    s1.types.insert("T".to_string(), Type::Int);

    // s2: U := T
    let mut s2 = Subst::empty();
    s2.types.insert("U".to_string(), Type::Var("T".to_string()));

    // Compose: should give U := Int, T := Int
    let composed = s1.compose(&s2);

    assert_eq!(composed.types.get("T"), Some(&Type::Int));
    assert_eq!(composed.types.get("U"), Some(&Type::Int));
}

#[test]
fn test_occurs_check_type_var_with_itself() {
    // Unifying T with T should succeed (no substitution needed)
    let result = unify_types(&Type::Var("T".to_string()), &Type::Var("T".to_string()));
    assert!(result.is_ok());
    let subst = result.unwrap();
    // Should be empty - no substitution needed when unifying var with itself
    assert!(subst.types.is_empty());
}

#[test]
fn test_occurs_check_row_var_with_itself() {
    // Unifying ..a with ..a should succeed (no substitution needed)
    let result = unify_stacks(
        &StackType::RowVar("a".to_string()),
        &StackType::RowVar("a".to_string()),
    );
    assert!(result.is_ok());
    let subst = result.unwrap();
    // Should be empty - no substitution needed when unifying var with itself
    assert!(subst.rows.is_empty());
}

#[test]
fn test_occurs_check_prevents_infinite_stack() {
    // Attempting to unify ..a with (..a Int) should fail
    // This would create an infinite type: ..a = (..a Int) = ((..a Int) Int) = ...
    let row_var = StackType::RowVar("a".to_string());
    let infinite_stack = StackType::RowVar("a".to_string()).push(Type::Int);

    let result = unify_stacks(&row_var, &infinite_stack);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("Occurs check failed"));
    assert!(err.contains("infinite"));
}

#[test]
fn test_occurs_check_allows_different_row_vars() {
    // Unifying ..a with ..b should succeed (different variables)
    let result = unify_stacks(
        &StackType::RowVar("a".to_string()),
        &StackType::RowVar("b".to_string()),
    );
    assert!(result.is_ok());
    let subst = result.unwrap();
    assert_eq!(
        subst.rows.get("a"),
        Some(&StackType::RowVar("b".to_string()))
    );
}

#[test]
fn test_occurs_check_allows_concrete_stack() {
    // Unifying ..a with (Int String) should succeed (no occurs)
    let row_var = StackType::RowVar("a".to_string());
    let concrete = StackType::Empty.push(Type::Int).push(Type::String);

    let result = unify_stacks(&row_var, &concrete);
    assert!(result.is_ok());
    let subst = result.unwrap();
    assert_eq!(subst.rows.get("a"), Some(&concrete));
}

#[test]
fn test_occurs_in_type() {
    // T occurs in T
    assert!(occurs_in_type("T", &Type::Var("T".to_string())));

    // T does not occur in U
    assert!(!occurs_in_type("T", &Type::Var("U".to_string())));

    // T does not occur in Int
    assert!(!occurs_in_type("T", &Type::Int));
    assert!(!occurs_in_type("T", &Type::String));
    assert!(!occurs_in_type("T", &Type::Bool));
}

#[test]
fn test_occurs_in_stack() {
    // ..a occurs in ..a
    assert!(occurs_in_stack("a", &StackType::RowVar("a".to_string())));

    // ..a does not occur in ..b
    assert!(!occurs_in_stack("a", &StackType::RowVar("b".to_string())));

    // ..a does not occur in Empty
    assert!(!occurs_in_stack("a", &StackType::Empty));

    // ..a occurs in (..a Int)
    let stack = StackType::RowVar("a".to_string()).push(Type::Int);
    assert!(occurs_in_stack("a", &stack));

    // ..a does not occur in (..b Int)
    let stack = StackType::RowVar("b".to_string()).push(Type::Int);
    assert!(!occurs_in_stack("a", &stack));

    // ..a does not occur in (Int String)
    let stack = StackType::Empty.push(Type::Int).push(Type::String);
    assert!(!occurs_in_stack("a", &stack));
}

#[test]
fn test_quotation_type_unification_stack_neutral() {
    // Q[..a -- ..a] should NOT unify with Q[..b -- ..b Int]
    // because the second quotation pushes a value
    use crate::types::Effect;

    let stack_neutral = Type::Quotation(Box::new(Effect::new(
        StackType::RowVar("a".to_string()),
        StackType::RowVar("a".to_string()),
    )));

    let pushes_int = Type::Quotation(Box::new(Effect::new(
        StackType::RowVar("b".to_string()),
        StackType::RowVar("b".to_string()).push(Type::Int),
    )));

    let result = unify_types(&stack_neutral, &pushes_int);
    // This SHOULD fail because unifying outputs would require ..a = ..a Int
    // which is an infinite type
    assert!(
        result.is_err(),
        "Unifying stack-neutral with stack-pushing quotation should fail, got {:?}",
        result
    );
}
