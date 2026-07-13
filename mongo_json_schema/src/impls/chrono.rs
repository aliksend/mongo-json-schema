//! Schemas for [`chrono`] `0.4` types (feature `chrono_0-4`).

use crate::schema::SchemaObject;
use crate::Schema;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

// A timezone-aware datetime maps to a BSON `date`.
impl<Tz: TimeZone> Schema for DateTime<Tz> {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject::of_bson_type("date")
    }
}

// The "naive" chrono types have no BSON counterpart and are serialized by serde
// as ISO-8601 strings, so they map to `string`.
impl Schema for NaiveDateTime {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject::of_bson_type("string")
    }
}

impl Schema for NaiveDate {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject::of_bson_type("string")
    }
}

impl Schema for NaiveTime {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject::of_bson_type("string")
    }
}
