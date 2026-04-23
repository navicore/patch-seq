//! Stack-effect lookup for the REPL IR pane.
//!
//! The compiler (`seqc::builtins::builtin_signature`) is the single source
//! of truth for builtin stack effects. This module just converts the
//! compiler's `Effect` into the display-friendly `StackEffect` the
//! ASCII-art renderer wants and strips the type-variable freshening
//! suffixes (`T$5` → `T`) that the compiler adds internally.

use super::stack_art::{Stack, StackEffect, StackValue};
use seqc::{Effect, StackType, Type};

/// Look up a stack effect by word name.
///
/// Returns `None` if the name is not a registered compiler builtin.
pub fn get_effect(word: &str) -> Option<StackEffect> {
    let effect = seqc::builtins::builtin_signature(word)?;
    Some(effect_to_display(word, &effect))
}

fn effect_to_display(name: &str, effect: &Effect) -> StackEffect {
    StackEffect::new(
        name.to_string(),
        stack_type_to_stack(&effect.inputs),
        stack_type_to_stack(&effect.outputs),
    )
}

fn stack_type_to_stack(st: &StackType) -> Stack {
    let mut values = Vec::new();
    let mut row_var = None;
    walk(st, &mut values, &mut row_var);
    let base = Stack::with_rest(row_var.as_deref().unwrap_or("rest"));
    values.into_iter().fold(base, |s, v| s.push(v))
}

fn walk(st: &StackType, values: &mut Vec<StackValue>, row_var: &mut Option<String>) {
    match st {
        StackType::Empty => {}
        StackType::Cons { rest, top } => {
            walk(rest, values, row_var);
            values.push(type_to_value(top));
        }
        StackType::RowVar(name) => {
            *row_var = Some(strip_freshening(name).to_string());
        }
    }
}

fn type_to_value(ty: &Type) -> StackValue {
    match ty {
        Type::Var(name) => StackValue::var(strip_freshening(name).to_string()),
        Type::Int => StackValue::ty("Int"),
        Type::Float => StackValue::ty("Float"),
        Type::Bool => StackValue::ty("Bool"),
        Type::String => StackValue::ty("String"),
        Type::Symbol => StackValue::ty("Symbol"),
        Type::Channel => StackValue::ty("Channel"),
        Type::Union(name) => StackValue::ty(name.clone()),
        Type::Quotation(_) => StackValue::ty("Quot"),
        Type::Closure { .. } => StackValue::ty("Closure"),
    }
}

/// Strip the type-variable freshening suffix (`T$5` → `T`).
fn strip_freshening(name: &str) -> &str {
    name.split('$').next().unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_up_known_builtins() {
        assert!(get_effect("dup").is_some());
        assert!(get_effect("drop").is_some());
        assert!(get_effect("swap").is_some());
        assert!(get_effect("i.+").is_some());
        assert!(get_effect("nonexistent").is_none());
    }

    #[test]
    fn and_is_typed_as_bool() {
        let sig = get_effect("and")
            .expect("and should be a builtin")
            .render_signature();
        assert!(
            sig.contains("Bool") && !sig.contains("Int"),
            "and should use Bool, not Int, got: {}",
            sig
        );
    }

    #[test]
    fn i_add_is_typed_as_int() {
        let sig = get_effect("i.+")
            .expect("i.+ should be a builtin")
            .render_signature();
        assert!(
            sig.contains("Int"),
            "i.+ should be typed as Int, got: {}",
            sig
        );
    }
}
