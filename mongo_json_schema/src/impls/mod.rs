//! [`Schema`](crate::Schema) implementations for standard-library and
//! third-party types.

mod primitives;
mod std_types;

#[cfg(feature = "chrono_0-4")]
mod chrono;

#[cfg(feature = "bson_2")]
mod bson;
