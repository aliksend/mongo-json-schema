#![allow(dead_code)]
//! Derived schemas for structs, including serde attribute handling.

use mongo_json_schema::{Schema, SchemaObject};
use serde_json::json;

#[derive(Schema)]
#[serde(rename_all = "camelCase")]
struct User {
    first_name: String,
    age: u8,
    #[serde(rename = "emailAddress")]
    email: Option<String>,
}

#[test]
fn rename_all_and_rename_and_option() {
    assert_eq!(
        User::mongo_json_schema().to_value(),
        json!({
            "bsonType": "object",
            "properties": {
                "firstName": { "bsonType": "string" },
                "age": { "bsonType": "int" },
                "emailAddress": { "bsonType": ["string", "null"] }
            },
            "required": ["firstName", "age"]
        })
    );
}

#[derive(Schema)]
#[serde(deny_unknown_fields)]
struct Strict {
    a: i32,
}

#[test]
fn deny_unknown_fields_sets_additional_properties_false() {
    assert_eq!(
        Strict::mongo_json_schema().to_value(),
        json!({
            "bsonType": "object",
            "properties": { "a": { "bsonType": "int" } },
            "required": ["a"],
            "additionalProperties": false
        })
    );
}

#[derive(Schema)]
#[serde(default)]
struct WithContainerDefault {
    a: i32,
    b: String,
}

#[test]
fn container_default_makes_all_fields_optional() {
    assert_eq!(
        WithContainerDefault::mongo_json_schema().to_value(),
        json!({
            "bsonType": "object",
            "properties": {
                "a": { "bsonType": "int" },
                "b": { "bsonType": "string" }
            }
        })
    );
}

#[derive(Schema)]
struct WithFieldDefault {
    #[serde(default)]
    a: i32,
    b: String,
}

#[test]
fn field_default_is_not_required() {
    assert_eq!(
        WithFieldDefault::mongo_json_schema().to_value(),
        json!({
            "bsonType": "object",
            "properties": {
                "a": { "bsonType": "int" },
                "b": { "bsonType": "string" }
            },
            "required": ["b"]
        })
    );
}

#[derive(Schema)]
struct Skipping {
    keep: i32,
    #[serde(skip)]
    #[allow(dead_code)]
    ignore: String,
}

#[test]
fn skip_removes_field() {
    assert_eq!(
        Skipping::mongo_json_schema().to_value(),
        json!({
            "bsonType": "object",
            "properties": { "keep": { "bsonType": "int" } },
            "required": ["keep"]
        })
    );
}

#[derive(Schema)]
struct Inner {
    a: i32,
}

#[derive(Schema)]
struct Outer {
    #[serde(flatten)]
    inner: Inner,
    b: String,
}

#[test]
fn flatten_merges_fields() {
    assert_eq!(
        Outer::mongo_json_schema().to_value(),
        json!({
            "bsonType": "object",
            "properties": {
                "a": { "bsonType": "int" },
                "b": { "bsonType": "string" }
            },
            "required": ["a", "b"]
        })
    );
}

#[derive(Schema)]
struct Nested {
    user: User,
    tags: Vec<String>,
}

#[test]
fn nested_struct_is_inlined() {
    assert_eq!(
        Nested::mongo_json_schema().to_value(),
        json!({
            "bsonType": "object",
            "properties": {
                "user": {
                    "bsonType": "object",
                    "properties": {
                        "firstName": { "bsonType": "string" },
                        "age": { "bsonType": "int" },
                        "emailAddress": { "bsonType": ["string", "null"] }
                    },
                    "required": ["firstName", "age"]
                },
                "tags": { "bsonType": "array", "items": { "bsonType": "string" } }
            },
            "required": ["user", "tags"]
        })
    );
}

/// A documented struct.
///
/// Second line.
#[derive(Schema)]
struct Documented {
    /// The identifier.
    id: i32,
}

#[test]
fn doc_comments_become_descriptions() {
    assert_eq!(
        Documented::mongo_json_schema().to_value(),
        json!({
            "bsonType": "object",
            "description": "A documented struct.\n\nSecond line.",
            "properties": {
                "id": { "bsonType": "int", "description": "The identifier." }
            },
            "required": ["id"]
        })
    );
}

#[derive(Schema)]
struct WithBsonTypeOverride {
    #[schema(bson_type = "objectId")]
    id: String,
}

#[test]
fn bson_type_override() {
    assert_eq!(
        WithBsonTypeOverride::mongo_json_schema().to_value(),
        json!({
            "bsonType": "object",
            "properties": { "id": { "bsonType": "objectId" } },
            "required": ["id"]
        })
    );
}

#[derive(Schema)]
struct WithValidation {
    #[schema(minimum = 0, maximum = 120)]
    age: u32,
    #[schema(minimum = -10.5, exclusive_maximum = 10)]
    temperature: f64,
    #[schema(min_length = 1, max_length = 64, pattern = "^[a-z]+$")]
    slug: String,
    #[schema(min_items = 1, max_items = 10, unique_items = true)]
    tags: Vec<String>,
    // Validation combines with `Option` (the field stays nullable & optional).
    #[schema(minimum = 1)]
    score: Option<i32>,
}

#[test]
fn field_validation_keywords() {
    assert_eq!(
        WithValidation::mongo_json_schema().to_value(),
        json!({
            "bsonType": "object",
            "properties": {
                "age": { "bsonType": "long", "minimum": 0.0, "maximum": 120.0 },
                "temperature": { "bsonType": "double", "minimum": -10.5, "exclusiveMaximum": 10.0 },
                "slug": {
                    "bsonType": "string",
                    "minLength": 1,
                    "maxLength": 64,
                    "pattern": "^[a-z]+$"
                },
                "tags": {
                    "bsonType": "array",
                    "items": { "bsonType": "string" },
                    "minItems": 1,
                    "maxItems": 10,
                    "uniqueItems": true
                },
                "score": { "bsonType": ["int", "null"], "minimum": 1.0 }
            },
            "required": ["age", "temperature", "slug", "tags"]
        })
    );
}

#[derive(Schema)]
struct Newtype(String);

#[derive(Schema)]
struct TupleStruct(i32, String);

#[derive(Schema)]
struct UnitStruct;

#[test]
fn tuple_and_newtype_and_unit_structs() {
    assert_eq!(
        Newtype::mongo_json_schema().to_value(),
        json!({ "bsonType": "string" })
    );
    assert_eq!(
        TupleStruct::mongo_json_schema().to_value(),
        json!({
            "bsonType": "array",
            "items": [{ "bsonType": "int" }, { "bsonType": "string" }],
            "minItems": 2,
            "maxItems": 2,
            "additionalItems": false
        })
    );
    assert_eq!(
        UnitStruct::mongo_json_schema().to_value(),
        json!({ "bsonType": "null" })
    );
}

#[derive(Schema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct Screaming {
    some_field: i32,
    another_one: i32,
}

#[test]
fn screaming_snake_case() {
    assert_eq!(
        Screaming::mongo_json_schema().to_value(),
        json!({
            "bsonType": "object",
            "properties": {
                "SOME_FIELD": { "bsonType": "int" },
                "ANOTHER_ONE": { "bsonType": "int" }
            },
            "required": ["SOME_FIELD", "ANOTHER_ONE"]
        })
    );
}

// A generic struct: type parameters get a `Schema` bound automatically.
#[derive(Schema)]
struct Wrapper<T> {
    value: T,
    count: i32,
}

#[test]
fn generic_struct() {
    assert_eq!(
        Wrapper::<String>::mongo_json_schema().to_value(),
        json!({
            "bsonType": "object",
            "properties": {
                "value": { "bsonType": "string" },
                "count": { "bsonType": "int" }
            },
            "required": ["value", "count"]
        })
    );
}

// Manual implementation for an arbitrary type, used as a field.
struct CustomId(#[allow(dead_code)] String);

impl Schema for CustomId {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject::of_bson_type("objectId")
    }
}

#[derive(Schema)]
struct HasCustomId {
    #[serde(rename = "_id")]
    id: CustomId,
    name: String,
}

// Deriving alongside serde's own derives must not conflict over the shared
// `#[serde(...)]` helper attribute.
#[derive(serde::Serialize, serde::Deserialize, Schema)]
#[serde(rename_all = "camelCase")]
struct AlsoSerde {
    my_field: i32,
}

#[test]
fn coexists_with_serde_derives() {
    assert_eq!(
        AlsoSerde::mongo_json_schema().to_value(),
        json!({
            "bsonType": "object",
            "properties": { "myField": { "bsonType": "int" } },
            "required": ["myField"]
        })
    );
    // And the serde derives still produce the matching serialized form.
    let json = serde_json::to_value(AlsoSerde { my_field: 7 }).unwrap();
    assert_eq!(json, json!({ "myField": 7 }));
}

#[test]
fn manual_impl_used_as_field() {
    assert_eq!(
        HasCustomId::mongo_json_schema().to_value(),
        json!({
            "bsonType": "object",
            "properties": {
                "_id": { "bsonType": "objectId" },
                "name": { "bsonType": "string" }
            },
            "required": ["_id", "name"]
        })
    );
}
