use super::*;
use crate::seqstring::global_string;

#[test]
fn test_int_roundtrip() {
    let value = Value::Int(42);
    let typed = TypedValue::from_value(&value).unwrap();
    let back = typed.to_value();
    assert_eq!(value, back);
}

#[test]
fn test_float_roundtrip() {
    let value = Value::Float(1.23456);
    let typed = TypedValue::from_value(&value).unwrap();
    let back = typed.to_value();
    assert_eq!(value, back);
}

#[test]
fn test_bool_roundtrip() {
    let value = Value::Bool(true);
    let typed = TypedValue::from_value(&value).unwrap();
    let back = typed.to_value();
    assert_eq!(value, back);
}

#[test]
fn test_string_roundtrip() {
    let value = Value::String(global_string("hello".to_string()));
    let typed = TypedValue::from_value(&value).unwrap();
    let back = typed.to_value();
    // Compare string contents (not pointer equality)
    match (&value, &back) {
        (Value::String(a), Value::String(b)) => assert_eq!(a.as_str(), b.as_str()),
        _ => panic!("Expected strings"),
    }
}

#[test]
fn test_map_roundtrip() {
    let mut map = HashMap::new();
    map.insert(
        RuntimeMapKey::String(global_string("key".to_string())),
        Value::Int(42),
    );
    map.insert(RuntimeMapKey::Int(1), Value::Bool(true));

    let value = Value::Map(Box::new(map));
    let typed = TypedValue::from_value(&value).unwrap();
    let back = typed.to_value();

    // Verify map contents
    if let Value::Map(m) = back {
        assert_eq!(m.len(), 2);
    } else {
        panic!("Expected map");
    }
}

#[test]
fn test_variant_roundtrip() {
    let data = VariantData::new(
        global_string("TestVariant".to_string()),
        vec![Value::Int(100), Value::Bool(false)],
    );
    let value = Value::Variant(Arc::new(data));

    let typed = TypedValue::from_value(&value).unwrap();
    let back = typed.to_value();

    if let Value::Variant(v) = back {
        assert_eq!(v.tag.as_str(), "TestVariant");
        assert_eq!(v.fields.len(), 2);
    } else {
        panic!("Expected variant");
    }
}

#[test]
fn test_quotation_not_serializable() {
    let value = Value::Quotation {
        wrapper: 12345,
        impl_: 12345,
    };
    let result = TypedValue::from_value(&value);
    assert!(matches!(
        result,
        Err(SerializeError::QuotationNotSerializable)
    ));
}

#[test]
fn test_closure_not_serializable() {
    use std::sync::Arc;
    let value = Value::Closure {
        fn_ptr: 12345,
        env: Arc::from(vec![Value::Int(1)].into_boxed_slice()),
    };
    let result = TypedValue::from_value(&value);
    assert!(matches!(
        result,
        Err(SerializeError::ClosureNotSerializable)
    ));
}

#[test]
fn test_bytes_roundtrip() {
    let typed = TypedValue::Map(BTreeMap::from([
        (TypedMapKey::String("x".to_string()), TypedValue::Int(10)),
        (TypedMapKey::Int(42), TypedValue::Bool(true)),
    ]));

    let bytes = typed.to_bytes().unwrap();
    let parsed = TypedValue::from_bytes(&bytes).unwrap();
    assert_eq!(typed, parsed);
}

#[test]
fn test_bincode_is_compact() {
    let typed = TypedValue::Int(42);
    let bytes = typed.to_bytes().unwrap();
    assert!(
        bytes.len() < 20,
        "Expected compact encoding, got {} bytes",
        bytes.len()
    );
}

#[test]
fn test_debug_string() {
    let typed = TypedValue::String("hello".to_string());
    assert_eq!(typed.to_debug_string(), "\"hello\"");

    let typed = TypedValue::Int(42);
    assert_eq!(typed.to_debug_string(), "42");
}

#[test]
fn test_nested_structure() {
    // Create nested map with variant
    let inner_variant = TypedValue::Variant {
        tag: "NestedVariant".to_string(),
        fields: vec![TypedValue::String("inner".to_string())],
    };

    let mut inner_map = BTreeMap::new();
    inner_map.insert(TypedMapKey::String("nested".to_string()), inner_variant);

    let outer = TypedValue::Map(inner_map);

    let bytes = outer.to_bytes().unwrap();
    let parsed = TypedValue::from_bytes(&bytes).unwrap();
    assert_eq!(outer, parsed);
}

#[test]
fn test_nan_not_serializable() {
    let value = Value::Float(f64::NAN);
    let result = TypedValue::from_value(&value);
    assert!(matches!(result, Err(SerializeError::NonFiniteFloat(_))));
}

#[test]
fn test_infinity_not_serializable() {
    let value = Value::Float(f64::INFINITY);
    let result = TypedValue::from_value(&value);
    assert!(matches!(result, Err(SerializeError::NonFiniteFloat(_))));

    let value = Value::Float(f64::NEG_INFINITY);
    let result = TypedValue::from_value(&value);
    assert!(matches!(result, Err(SerializeError::NonFiniteFloat(_))));
}

#[test]
fn test_corrupted_data_returns_error() {
    // Random bytes that aren't valid bincode
    let corrupted = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let result = TypedValue::from_bytes(&corrupted);
    assert!(result.is_err());
}

#[test]
fn test_empty_data_returns_error() {
    let result = TypedValue::from_bytes(&[]);
    assert!(result.is_err());
}

#[test]
fn test_truncated_data_returns_error() {
    // Serialize valid data, then truncate
    let typed = TypedValue::String("hello world".to_string());
    let bytes = typed.to_bytes().unwrap();
    let truncated = &bytes[..bytes.len() / 2];
    let result = TypedValue::from_bytes(truncated);
    assert!(result.is_err());
}
