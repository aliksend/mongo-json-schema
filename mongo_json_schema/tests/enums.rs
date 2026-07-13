#![allow(dead_code)]
//! Derived schemas for the four serde enum representations.

use mongo_json_schema::Schema;
use serde_json::json;

#[derive(Schema)]
enum Simple {
    A,
    B,
    C,
}

#[test]
fn unit_only_enum_is_string_enum() {
    assert_eq!(
        Simple::mongo_json_schema().to_value(),
        json!({ "bsonType": "string", "enum": ["A", "B", "C"] })
    );
}

#[derive(Schema)]
#[serde(rename_all = "snake_case")]
enum RenamedUnits {
    FirstVariant,
    SecondVariant,
}

#[test]
fn enum_rename_all_applies_to_variants() {
    assert_eq!(
        RenamedUnits::mongo_json_schema().to_value(),
        json!({ "bsonType": "string", "enum": ["first_variant", "second_variant"] })
    );
}

#[derive(Schema)]
enum External {
    Unit,
    Newtype(i32),
    Struct { x: i32, y: String },
}

#[test]
fn externally_tagged() {
    assert_eq!(
        External::mongo_json_schema().to_value(),
        json!({
            "oneOf": [
                { "bsonType": "string", "enum": ["Unit"] },
                {
                    "bsonType": "object",
                    "properties": { "Newtype": { "bsonType": "int" } },
                    "required": ["Newtype"],
                    "additionalProperties": false
                },
                {
                    "bsonType": "object",
                    "properties": {
                        "Struct": {
                            "bsonType": "object",
                            "properties": {
                                "x": { "bsonType": "int" },
                                "y": { "bsonType": "string" }
                            },
                            "required": ["x", "y"]
                        }
                    },
                    "required": ["Struct"],
                    "additionalProperties": false
                }
            ]
        })
    );
}

#[derive(Schema)]
#[serde(tag = "type")]
enum Internal {
    A,
    B { x: i32 },
}

#[test]
fn internally_tagged() {
    assert_eq!(
        Internal::mongo_json_schema().to_value(),
        json!({
            "oneOf": [
                {
                    "bsonType": "object",
                    "properties": { "type": { "bsonType": "string", "enum": ["A"] } },
                    "required": ["type"]
                },
                {
                    "bsonType": "object",
                    "properties": {
                        "type": { "bsonType": "string", "enum": ["B"] },
                        "x": { "bsonType": "int" }
                    },
                    "required": ["type", "x"]
                }
            ]
        })
    );
}

#[derive(Schema)]
#[serde(tag = "t", content = "c")]
enum Adjacent {
    Unit,
    Data(i32),
}

#[test]
fn adjacently_tagged() {
    assert_eq!(
        Adjacent::mongo_json_schema().to_value(),
        json!({
            "oneOf": [
                {
                    "bsonType": "object",
                    "properties": { "t": { "bsonType": "string", "enum": ["Unit"] } },
                    "required": ["t"]
                },
                {
                    "bsonType": "object",
                    "properties": {
                        "t": { "bsonType": "string", "enum": ["Data"] },
                        "c": { "bsonType": "int" }
                    },
                    "required": ["t", "c"]
                }
            ]
        })
    );
}

#[derive(Schema)]
#[serde(untagged)]
enum Untagged {
    Number(i32),
    Text(String),
}

#[test]
fn untagged() {
    assert_eq!(
        Untagged::mongo_json_schema().to_value(),
        json!({
            "anyOf": [
                { "bsonType": "int" },
                { "bsonType": "string" }
            ]
        })
    );
}

#[derive(Schema)]
#[serde(rename_all = "camelCase")]
enum InternalRenamed {
    #[serde(rename = "kindA")]
    VariantA,
    VariantB {
        some_value: i32,
    },
}

#[test]
fn variant_rename_and_field_rename_all() {
    // `rename_all = "camelCase"` on the container renames variant *names*, not
    // the struct-variant fields (that would be `rename_all_fields`). An explicit
    // `#[serde(rename = "...")]` wins over the rule.
    assert_eq!(
        InternalRenamed::mongo_json_schema().to_value(),
        json!({
            "oneOf": [
                { "bsonType": "string", "enum": ["kindA"] },
                {
                    "bsonType": "object",
                    "properties": {
                        "variantB": {
                            "bsonType": "object",
                            "properties": { "some_value": { "bsonType": "int" } },
                            "required": ["some_value"]
                        }
                    },
                    "required": ["variantB"],
                    "additionalProperties": false
                }
            ]
        })
    );
}

#[derive(Schema)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
enum FieldsRenamed {
    Variant { some_value: i32 },
}

#[test]
fn rename_all_fields_applies_to_variant_fields() {
    assert_eq!(
        FieldsRenamed::mongo_json_schema().to_value(),
        json!({
            "oneOf": [
                {
                    "bsonType": "object",
                    "properties": {
                        "variant": {
                            "bsonType": "object",
                            "properties": { "someValue": { "bsonType": "int" } },
                            "required": ["someValue"]
                        }
                    },
                    "required": ["variant"],
                    "additionalProperties": false
                }
            ]
        })
    );
}
