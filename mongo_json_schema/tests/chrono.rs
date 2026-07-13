#![allow(dead_code)]
//! Schemas for `chrono` types (requires the `chrono_0-4` feature).
#![cfg(feature = "chrono_0-4")]

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use mongo_json_schema::Schema;
use serde_json::json;

#[test]
fn datetime_is_bson_date() {
    assert_eq!(
        <DateTime<Utc> as Schema>::mongo_json_schema().to_value(),
        json!({ "bsonType": "date" })
    );
}

#[test]
fn naive_types_are_strings() {
    assert_eq!(
        NaiveDateTime::mongo_json_schema().to_value(),
        json!({ "bsonType": "string" })
    );
    assert_eq!(
        NaiveDate::mongo_json_schema().to_value(),
        json!({ "bsonType": "string" })
    );
    assert_eq!(
        NaiveTime::mongo_json_schema().to_value(),
        json!({ "bsonType": "string" })
    );
}

#[derive(Schema)]
struct Event {
    created_at: DateTime<Utc>,
    scheduled_for: Option<DateTime<Utc>>,
}

#[test]
fn datetime_in_struct() {
    assert_eq!(
        Event::mongo_json_schema().to_value(),
        json!({
            "bsonType": "object",
            "properties": {
                "created_at": { "bsonType": "date" },
                "scheduled_for": { "bsonType": ["date", "null"] }
            },
            "required": ["created_at"]
        })
    );
}
