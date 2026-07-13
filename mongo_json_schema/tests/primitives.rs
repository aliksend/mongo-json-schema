#![allow(dead_code)]
//! Schemas for primitive and standard-library types.

use mongo_json_schema::{Schema, SchemaObject};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

fn schema_of<T: Schema>() -> serde_json::Value {
    T::mongo_json_schema().to_value()
}

#[test]
fn strings() {
    assert_eq!(schema_of::<String>(), json!({ "bsonType": "string" }));
    assert_eq!(
        schema_of::<char>(),
        json!({
            "bsonType": "string",
            "minLength": 1,
            "maxLength": 1
        })
    );
}

#[test]
fn integers() {
    assert_eq!(schema_of::<i8>(), json!({ "bsonType": "int" }));
    assert_eq!(schema_of::<i32>(), json!({ "bsonType": "int" }));
    assert_eq!(schema_of::<u16>(), json!({ "bsonType": "int" }));

    assert_eq!(schema_of::<i64>(), json!({ "bsonType": "long" }));
    assert_eq!(schema_of::<u32>(), json!({ "bsonType": "long" }));
    assert_eq!(schema_of::<u64>(), json!({ "bsonType": "long" }));
    assert_eq!(schema_of::<usize>(), json!({ "bsonType": "long" }));
}

#[test]
fn floats_and_bool() {
    assert_eq!(schema_of::<f32>(), json!({ "bsonType": "double" }));
    assert_eq!(schema_of::<f64>(), json!({ "bsonType": "double" }));
    assert_eq!(schema_of::<bool>(), json!({ "bsonType": "bool" }));
}

#[test]
fn nonzero() {
    assert_eq!(
        schema_of::<std::num::NonZeroU64>(),
        json!({ "bsonType": "long" })
    );
}

#[test]
fn option_makes_type_nullable() {
    assert_eq!(
        schema_of::<Option<String>>(),
        json!({ "bsonType": ["string", "null"] })
    );
    assert_eq!(
        schema_of::<Option<i32>>(),
        json!({ "bsonType": ["int", "null"] })
    );
    // `Option<Option<T>>` should not accumulate duplicate nulls.
    assert_eq!(
        schema_of::<Option<Option<i32>>>(),
        json!({ "bsonType": ["int", "null"] })
    );
    assert!(<Option<i32> as Schema>::_mongo_is_option());
    assert!(!<i32 as Schema>::_mongo_is_option());
}

#[test]
fn wrappers_delegate() {
    use std::borrow::Cow;
    use std::sync::Arc;
    assert_eq!(schema_of::<Box<String>>(), json!({ "bsonType": "string" }));
    assert_eq!(schema_of::<Arc<i32>>(), json!({ "bsonType": "int" }));
    assert_eq!(
        schema_of::<Cow<'static, str>>(),
        json!({ "bsonType": "string" })
    );
    assert_eq!(schema_of::<&i32>(), json!({ "bsonType": "int" }));
}

#[test]
fn sequences() {
    assert_eq!(
        schema_of::<Vec<i32>>(),
        json!({ "bsonType": "array", "items": { "bsonType": "int" } })
    );
    assert_eq!(
        schema_of::<HashSet<String>>(),
        json!({ "bsonType": "array", "items": { "bsonType": "string" }, "uniqueItems": true })
    );
    assert_eq!(
        schema_of::<BTreeSet<String>>(),
        json!({ "bsonType": "array", "items": { "bsonType": "string" }, "uniqueItems": true })
    );
}

#[test]
fn fixed_arrays() {
    assert_eq!(
        schema_of::<[u8; 4]>(),
        json!({
            "bsonType": "array",
            "items": { "bsonType": "int" },
            "minItems": 4,
            "maxItems": 4
        })
    );
}

#[test]
fn maps() {
    let expected = json!({
        "bsonType": "object",
        "additionalProperties": { "bsonType": "int" }
    });
    assert_eq!(schema_of::<HashMap<String, i32>>(), expected);
    assert_eq!(schema_of::<BTreeMap<String, i32>>(), expected);
}

#[test]
fn tuples() {
    assert_eq!(
        schema_of::<(i32, String)>(),
        json!({
            "bsonType": "array",
            "items": [{ "bsonType": "int" }, { "bsonType": "string" }],
            "minItems": 2,
            "maxItems": 2,
            "additionalItems": false
        })
    );
    assert_eq!(schema_of::<()>(), json!({ "bsonType": "null" }));
}

#[test]
fn any_schema() {
    assert_eq!(SchemaObject::any().to_value(), json!({}));
    assert!(SchemaObject::any().is_any());
}
