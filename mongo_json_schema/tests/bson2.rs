#![allow(dead_code)]
//! Schemas for `bson` 2 types (requires the `bson_2` feature).
#![cfg(feature = "bson_2")]

use bson2::{oid::ObjectId, Binary, Bson, DateTime, Decimal128, Document, Timestamp};
use mongo_json_schema::Schema;
use serde_json::json;

#[test]
fn bson_scalar_types() {
    assert_eq!(
        ObjectId::mongo_json_schema().to_value(),
        json!({ "bsonType": "objectId" })
    );
    assert_eq!(
        DateTime::mongo_json_schema().to_value(),
        json!({ "bsonType": "date" })
    );
    assert_eq!(
        Decimal128::mongo_json_schema().to_value(),
        json!({ "bsonType": "decimal" })
    );
    assert_eq!(
        Timestamp::mongo_json_schema().to_value(),
        json!({ "bsonType": "timestamp" })
    );
    assert_eq!(
        Binary::mongo_json_schema().to_value(),
        json!({ "bsonType": "binData" })
    );
    assert_eq!(
        Document::mongo_json_schema().to_value(),
        json!({ "bsonType": "object" })
    );
    assert_eq!(Bson::mongo_json_schema().to_value(), json!({}));
}

/// A typical MongoDB document with an `ObjectId` primary key.
#[derive(Schema)]
struct Account {
    #[serde(rename = "_id")]
    id: ObjectId,
    balance: Decimal128,
    created_at: DateTime,
    note: Option<String>,
}

#[test]
fn document_with_object_id() {
    assert_eq!(
        Account::mongo_json_schema().to_value(),
        json!({
            "bsonType": "object",
            "description": "A typical MongoDB document with an `ObjectId` primary key.",
            "properties": {
                "_id": { "bsonType": "objectId" },
                "balance": { "bsonType": "decimal" },
                "created_at": { "bsonType": "date" },
                "note": { "bsonType": ["string", "null"] }
            },
            "required": ["_id", "balance", "created_at"]
        })
    );
}
