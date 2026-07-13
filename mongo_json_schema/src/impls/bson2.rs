//! Schemas for [`bson`](bson2) `2` types (feature `bson_2`).

use crate::schema::SchemaObject;
use crate::Schema;

use bson2::{oid::ObjectId, Binary, Bson, DateTime, Decimal128, Document, Regex, Timestamp, Uuid};

impl Schema for ObjectId {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject::of_bson_type("objectId")
    }
}

impl Schema for DateTime {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject::of_bson_type("date")
    }
}

impl Schema for Decimal128 {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject::of_bson_type("decimal")
    }
}

impl Schema for Timestamp {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject::of_bson_type("timestamp")
    }
}

impl Schema for Binary {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject::of_bson_type("binData")
    }
}

// `bson::Uuid` is stored as BSON binary data (subtype 4).
impl Schema for Uuid {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject::of_bson_type("binData")
    }
}

impl Schema for Regex {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject::of_bson_type("regex")
    }
}

impl Schema for Document {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject::of_bson_type("object")
    }
}

// A raw `Bson` value can be anything.
impl Schema for Bson {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject::any()
    }
}
