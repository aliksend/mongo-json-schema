//! The data model used to represent a MongoDB-compatible JSON Schema.
//!
//! Unlike a plain JSON Schema (as produced by [`schemars`](https://docs.rs/schemars)),
//! MongoDB's `$jsonSchema` operator supports only a subset of JSON Schema draft 4,
//! with some MongoDB-specific extensions. Most notably:
//!
//! * It uses `bsonType` (with BSON type aliases such as `objectId`, `date`,
//!   `decimal`, `long`, `int`, `double`, `binData`, ...) in addition to `type`.
//! * It does **not** support `$ref`, `$defs`/`definitions`, `$schema`, `$id`,
//!   `format`, or `default`. Because of the lack of `$ref`, every generated schema
//!   is fully self-contained (inlined).
//!
//! See <https://www.mongodb.com/docs/manual/reference/operator/query/jsonSchema/>.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// An insertion-order-preserving map, used for `properties`, `patternProperties`, etc.
pub type Map<K, V> = indexmap::IndexMap<K, V>;

/// A value that is either a single `T` or a list of `T`.
///
/// Used for keywords such as `bsonType` (which is either `"string"` or
/// `["string", "null"]`) and `items`.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SingleOrVec<T> {
    Single(Box<T>),
    Vec(Vec<T>),
}

impl<T> From<T> for SingleOrVec<T> {
    fn from(single: T) -> Self {
        SingleOrVec::Single(Box::new(single))
    }
}

impl<T> From<Vec<T>> for SingleOrVec<T> {
    fn from(vec: Vec<T>) -> Self {
        SingleOrVec::Vec(vec)
    }
}

/// The value of the `additionalProperties` / `additionalItems` keyword: either a
/// boolean, or a nested schema.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BoolOrSchema {
    Bool(bool),
    Schema(Box<SchemaObject>),
}

impl From<bool> for BoolOrSchema {
    fn from(b: bool) -> Self {
        BoolOrSchema::Bool(b)
    }
}

impl From<SchemaObject> for BoolOrSchema {
    fn from(schema: SchemaObject) -> Self {
        BoolOrSchema::Schema(Box::new(schema))
    }
}

/// A MongoDB-compatible JSON Schema object.
///
/// Only fields that are `Some`/non-empty are serialized, so the default value
/// serializes to `{}` (which matches "any value" in MongoDB).
#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct SchemaObject {
    /// The `bsonType` keyword — the MongoDB BSON type alias(es) this value may take,
    /// e.g. `"string"`, `"objectId"`, `"date"`, `"int"`, `"long"`, `"double"`,
    /// `"decimal"`, `"bool"`, `"object"`, `"array"`, `"binData"`, `"null"`, ...
    #[serde(rename = "bsonType", skip_serializing_if = "Option::is_none")]
    pub bson_type: Option<SingleOrVec<String>>,

    /// The standard JSON Schema `type` keyword. MongoDB only allows the JSON type
    /// names here (`"object"`, `"array"`, `"number"`, `"boolean"`, `"string"`,
    /// `"null"`) and forbids mixing it with `bsonType`. Most impls prefer
    /// [`bson_type`](Self::bson_type); this is provided for completeness.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub instance_type: Option<SingleOrVec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The `enum` keyword — the exhaustive set of allowed values.
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<Value>>,

    // --- Combinators ---
    #[serde(rename = "allOf", skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<SchemaObject>>,
    #[serde(rename = "anyOf", skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<SchemaObject>>,
    #[serde(rename = "oneOf", skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<SchemaObject>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not: Option<Box<SchemaObject>>,

    // --- Object validation ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Map<String, SchemaObject>>,
    #[serde(rename = "patternProperties", skip_serializing_if = "Option::is_none")]
    pub pattern_properties: Option<Map<String, SchemaObject>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(
        rename = "additionalProperties",
        skip_serializing_if = "Option::is_none"
    )]
    pub additional_properties: Option<Box<BoolOrSchema>>,
    #[serde(rename = "minProperties", skip_serializing_if = "Option::is_none")]
    pub min_properties: Option<u64>,
    #[serde(rename = "maxProperties", skip_serializing_if = "Option::is_none")]
    pub max_properties: Option<u64>,

    // --- Array validation ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<SingleOrVec<SchemaObject>>,
    #[serde(rename = "additionalItems", skip_serializing_if = "Option::is_none")]
    pub additional_items: Option<Box<BoolOrSchema>>,
    #[serde(rename = "minItems", skip_serializing_if = "Option::is_none")]
    pub min_items: Option<u64>,
    #[serde(rename = "maxItems", skip_serializing_if = "Option::is_none")]
    pub max_items: Option<u64>,
    #[serde(rename = "uniqueItems", skip_serializing_if = "Option::is_none")]
    pub unique_items: Option<bool>,

    // --- Numeric validation ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(rename = "exclusiveMinimum", skip_serializing_if = "Option::is_none")]
    pub exclusive_minimum: Option<f64>,
    #[serde(rename = "exclusiveMaximum", skip_serializing_if = "Option::is_none")]
    pub exclusive_maximum: Option<f64>,
    #[serde(rename = "multipleOf", skip_serializing_if = "Option::is_none")]
    pub multiple_of: Option<f64>,

    // --- String validation ---
    #[serde(rename = "minLength", skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u64>,
    #[serde(rename = "maxLength", skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    /// Any other keywords, flattened into the schema object. Useful for MongoDB
    /// extensions or keywords not modeled above (e.g. `dependencies`).
    #[serde(flatten)]
    pub extensions: Map<String, Value>,
}

impl SchemaObject {
    /// A schema that permits any value (`{}`).
    pub fn any() -> Self {
        SchemaObject::default()
    }

    /// A schema for a single BSON type, e.g. `SchemaObject::of_bson_type("objectId")`.
    pub fn of_bson_type(bson_type: &str) -> Self {
        SchemaObject {
            bson_type: Some(SingleOrVec::Single(Box::new(bson_type.to_owned()))),
            ..Default::default()
        }
    }

    /// Returns `true` if this schema is `{}` (permits any value).
    pub fn is_any(&self) -> bool {
        *self == SchemaObject::default()
    }

    /// Produces a schema equivalent to this one but that also permits `null`.
    ///
    /// This is what `Option<T>` maps to. The exact transformation depends on the
    /// shape of the schema:
    ///
    /// * If it declares a `bsonType`, `"null"` is appended to it.
    /// * Otherwise if it is an `enum`, `null` is added to the allowed values.
    /// * Otherwise (combinators, `{}`, ...) it is wrapped in
    ///   `{ "anyOf": [ <schema>, { "bsonType": "null" } ] }`.
    pub fn optional(self) -> Self {
        // `{}` already allows null.
        if self.is_any() {
            return self;
        }

        if let Some(bson_type) = self.bson_type.clone() {
            // Only fold `null` into `bsonType` when the schema is a plain type
            // constraint (no combinators that would conflict).
            if self.any_of.is_none() && self.one_of.is_none() && self.all_of.is_none() {
                let mut types = match bson_type {
                    SingleOrVec::Single(t) => vec![*t],
                    SingleOrVec::Vec(v) => v,
                };
                if !types.iter().any(|t| t == "null") {
                    types.push("null".to_owned());
                }
                return SchemaObject {
                    bson_type: Some(SingleOrVec::Vec(types)),
                    ..self
                };
            }
        } else if let Some(values) = self.enum_values.clone() {
            if self.any_of.is_none() && self.one_of.is_none() && self.all_of.is_none() {
                let mut values = values;
                if !values.iter().any(|v| v.is_null()) {
                    values.push(Value::Null);
                }
                return SchemaObject {
                    enum_values: Some(values),
                    ..self
                };
            }
        }

        // Fallback: wrap in anyOf with a null schema.
        SchemaObject {
            any_of: Some(vec![self, SchemaObject::of_bson_type("null")]),
            ..Default::default()
        }
    }

    /// Serializes this schema to a [`serde_json::Value`]. This is the value you
    /// place under the `$jsonSchema` key of a MongoDB collection validator.
    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).expect("SchemaObject is always serializable")
    }
}
