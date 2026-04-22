//! Variant constructor functions `patch_seq_make_variant_0` through
//! `patch_seq_make_variant_12`: builds a tagged variant from a Symbol tag
//! plus N field values popped off the stack.

use crate::stack::{Stack, pop, push};
use crate::value::Value;
use std::sync::Arc;

// ============================================================================
// Type-safe variant constructors with fixed arity
// Now accept Symbol as tag for dynamic variant construction (SON support)
// ============================================================================

/// Create a variant with 0 fields (just a tag)
///
/// Stack effect: ( Symbol -- Variant )
///
/// # Safety
/// Stack must have at least one Symbol (the tag) on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_0(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-0: expected Symbol (tag), got {:?}", tag_val),
        };

        let variant = Value::Variant(Arc::new(VariantData::new(tag, vec![])));
        push(stack, variant)
    }
}

/// Create a variant with 1 field
///
/// Stack effect: ( field1 Symbol -- Variant )
///
/// # Safety
/// Stack must have field1 and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_1(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-1: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(tag, vec![field1])));
        push(stack, variant)
    }
}

/// Create a variant with 2 fields
///
/// Stack effect: ( field1 field2 Symbol -- Variant )
///
/// # Safety
/// Stack must have field1, field2, and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_2(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-2: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(tag, vec![field1, field2])));
        push(stack, variant)
    }
}

/// Create a variant with 3 fields
///
/// Stack effect: ( field1 field2 field3 Symbol -- Variant )
///
/// # Safety
/// Stack must have field1, field2, field3, and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_3(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-3: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![field1, field2, field3],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 4 fields
///
/// Stack effect: ( field1 field2 field3 field4 Symbol -- Variant )
///
/// # Safety
/// Stack must have field1, field2, field3, field4, and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_4(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-4: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![field1, field2, field3, field4],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 5 fields
///
/// Stack effect: ( field1 field2 field3 field4 field5 Symbol -- Variant )
///
/// # Safety
/// Stack must have 5 fields and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_5(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-5: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field5) = pop(stack);
        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![field1, field2, field3, field4, field5],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 6 fields
///
/// Stack effect: ( field1 field2 field3 field4 field5 field6 Symbol -- Variant )
///
/// # Safety
/// Stack must have 6 fields and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_6(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-6: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field6) = pop(stack);
        let (stack, field5) = pop(stack);
        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![field1, field2, field3, field4, field5, field6],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 7 fields
///
/// Stack effect: ( field1 field2 field3 field4 field5 field6 field7 Symbol -- Variant )
///
/// # Safety
/// Stack must have 7 fields and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_7(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-7: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field7) = pop(stack);
        let (stack, field6) = pop(stack);
        let (stack, field5) = pop(stack);
        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![field1, field2, field3, field4, field5, field6, field7],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 8 fields
///
/// Stack effect: ( field1 field2 field3 field4 field5 field6 field7 field8 Symbol -- Variant )
///
/// # Safety
/// Stack must have 8 fields and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_8(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-8: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field8) = pop(stack);
        let (stack, field7) = pop(stack);
        let (stack, field6) = pop(stack);
        let (stack, field5) = pop(stack);
        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![
                field1, field2, field3, field4, field5, field6, field7, field8,
            ],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 9 fields
///
/// Stack effect: ( field1 ... field9 Symbol -- Variant )
///
/// # Safety
/// Stack must have 9 fields and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_9(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-9: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field9) = pop(stack);
        let (stack, field8) = pop(stack);
        let (stack, field7) = pop(stack);
        let (stack, field6) = pop(stack);
        let (stack, field5) = pop(stack);
        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![
                field1, field2, field3, field4, field5, field6, field7, field8, field9,
            ],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 10 fields
///
/// Stack effect: ( field1 ... field10 Symbol -- Variant )
///
/// # Safety
/// Stack must have 10 fields and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_10(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-10: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field10) = pop(stack);
        let (stack, field9) = pop(stack);
        let (stack, field8) = pop(stack);
        let (stack, field7) = pop(stack);
        let (stack, field6) = pop(stack);
        let (stack, field5) = pop(stack);
        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![
                field1, field2, field3, field4, field5, field6, field7, field8, field9, field10,
            ],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 11 fields
///
/// Stack effect: ( field1 ... field11 Symbol -- Variant )
///
/// # Safety
/// Stack must have 11 fields and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_11(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-11: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field11) = pop(stack);
        let (stack, field10) = pop(stack);
        let (stack, field9) = pop(stack);
        let (stack, field8) = pop(stack);
        let (stack, field7) = pop(stack);
        let (stack, field6) = pop(stack);
        let (stack, field5) = pop(stack);
        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![
                field1, field2, field3, field4, field5, field6, field7, field8, field9, field10,
                field11,
            ],
        )));
        push(stack, variant)
    }
}

/// Create a variant with 12 fields
///
/// Stack effect: ( field1 ... field12 Symbol -- Variant )
///
/// # Safety
/// Stack must have 12 fields and Symbol tag on top
#[unsafe(no_mangle)]
pub unsafe extern "C" fn patch_seq_make_variant_12(stack: Stack) -> Stack {
    use crate::value::VariantData;

    unsafe {
        let (stack, tag_val) = pop(stack);
        let tag = match tag_val {
            Value::Symbol(s) => s,
            _ => panic!("make-variant-12: expected Symbol (tag), got {:?}", tag_val),
        };

        let (stack, field12) = pop(stack);
        let (stack, field11) = pop(stack);
        let (stack, field10) = pop(stack);
        let (stack, field9) = pop(stack);
        let (stack, field8) = pop(stack);
        let (stack, field7) = pop(stack);
        let (stack, field6) = pop(stack);
        let (stack, field5) = pop(stack);
        let (stack, field4) = pop(stack);
        let (stack, field3) = pop(stack);
        let (stack, field2) = pop(stack);
        let (stack, field1) = pop(stack);
        let variant = Value::Variant(Arc::new(VariantData::new(
            tag,
            vec![
                field1, field2, field3, field4, field5, field6, field7, field8, field9, field10,
                field11, field12,
            ],
        )));
        push(stack, variant)
    }
}
