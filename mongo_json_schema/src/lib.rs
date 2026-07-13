//! # mongo_json_schema
//!
//! Generate [MongoDB-compatible](https://www.mongodb.com/docs/manual/reference/operator/query/jsonSchema/)
//! JSON Schemas (`$jsonSchema`) from Rust types.
//!
//! This crate is heavily inspired by, and API-compatible-in-spirit with,
//! [`schemars`](https://docs.rs/schemars). The key difference is that it emits
//! schemas that MongoDB's `$jsonSchema` operator understands:
//!
//! * BSON type aliases via `bsonType` (`objectId`, `date`, `decimal`, `long`,
//!   `int`, `double`, `binData`, ...) instead of JSON `type`/`format`.
//! * No `$ref` / `$defs` — every schema is fully inlined, because MongoDB does
//!   not support references.
//!
//! ## Deriving a schema
//!
//! ```
//! use mongo_json_schema::Schema;
//!
//! #[derive(Schema)]
//! #[serde(rename_all = "camelCase")]
//! struct User {
//!     first_name: String,
//!     age: u8,
//!     #[serde(rename = "emailAddress")]
//!     email: Option<String>,
//! }
//!
//! let schema = User::mongo_json_schema().to_value();
//! ```
//!
//! ## Implementing [`Schema`] manually
//!
//! ```
//! use mongo_json_schema::{Schema, SchemaObject};
//!
//! struct MyId(String);
//!
//! impl Schema for MyId {
//!     fn mongo_json_schema() -> SchemaObject {
//!         SchemaObject::of_bson_type("objectId")
//!     }
//! }
//! ```

mod impls;
mod schema;

pub use schema::{BoolOrSchema, Map, SchemaObject, SingleOrVec};

#[cfg(feature = "derive")]
pub use mongo_json_schema_derive::Schema;

/// Implementation details used by the derive macro. Not part of the public API;
/// do not depend on anything here directly.
#[doc(hidden)]
pub mod _private {
    pub use serde_json;
}

/// A type that can describe itself with a MongoDB-compatible JSON Schema.
///
/// Implement this trait — either via `#[derive(Schema)]` or by hand — to make a
/// type usable as a field in other schema-deriving types.
pub trait Schema {
    /// Generates the MongoDB JSON Schema for this type.
    fn mongo_json_schema() -> SchemaObject;

    /// Whether this type is `Option`-like, i.e. it permits absence.
    ///
    /// The derive macro uses this to decide whether a struct field should be
    /// listed in the schema's `required` array. Only [`Option`] overrides it.
    ///
    /// This is an implementation detail; you should not need to override it.
    #[doc(hidden)]
    fn _mongo_is_option() -> bool {
        false
    }
}
