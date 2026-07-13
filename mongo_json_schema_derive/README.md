# mongo_json_schema_derive

Procedural-macro implementation of `#[derive(Schema)]` for the
[`mongo_json_schema`](https://crates.io/crates/mongo_json_schema) crate.

You almost never depend on this crate directly — enable the `derive` feature of
`mongo_json_schema` (on by default) and use `mongo_json_schema::Schema`.
