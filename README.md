# mongo_json_schema

Generate [MongoDB-compatible](https://www.mongodb.com/docs/manual/reference/operator/query/jsonSchema/)
JSON Schemas (`$jsonSchema`) from Rust types.

This crate is a spiritual sibling of [`schemars`](https://github.com/GREsau/schemars):
it derives a JSON Schema from your types and mirrors `schemars`' handling of
`serde` attributes as closely as possible. The difference is the **output
dialect** — schemas are emitted in the flavour MongoDB's `$jsonSchema` operator
understands.

## How it differs from schemars

MongoDB's `$jsonSchema` supports only a subset of JSON Schema draft 4, plus a few
MongoDB-specific extensions. This crate targets that dialect:

| Aspect       | `schemars`                         | `mongo_json_schema`                                                                   |
| ------------ | ---------------------------------- | ------------------------------------------------------------------------------------- |
| Type keyword | `"type"` + `"format"`              | `"bsonType"` (`objectId`, `date`, `decimal`, `long`, `int`, `double`, `binData`, ...) |
| References   | `$ref` into `$defs`                | none — every schema is **fully inlined**                                              |
| Envelope     | `$schema`, `$id`, `title`, `$defs` | omitted (MongoDB rejects them)                                                        |
| Nullable     | `["T", "null"]` or `anyOf`         | same, but expressed with `bsonType`                                                   |

Because MongoDB does not support `$ref`, all nested and referenced types are
inlined into a single self-contained document. This means truly recursive types
cannot be represented.

## Features

| Feature      | Default | Enables                                                                             |
| ------------ | ------- | ----------------------------------------------------------------------------------- |
| `derive`     | ✅      | the `#[derive(Schema)]` macro                                                       |
| `chrono_0-4` |         | schemas for `chrono` 0.4 types (`DateTime` → `date`, ...)                           |
| `bson_2`     |         | schemas for `bson` 2 types (`ObjectId` → `objectId`, `Decimal128` → `decimal`, ...) |

```toml
[dependencies]
mongo_json_schema = { version = "0.1", features = ["chrono_0-4", "bson_2"] }
```

## Deriving a schema

```rust
use mongo_json_schema::Schema;

#[derive(Schema)]
#[serde(rename_all = "camelCase")]
struct User {
    first_name: String,
    age: u8,
    #[serde(rename = "emailAddress")]
    email: Option<String>,
}

let schema = User::mongo_json_schema().to_value();
```

produces

```json
{
  "bsonType": "object",
  "properties": {
    "firstName": { "bsonType": "string" },
    "age": { "bsonType": "int" },
    "emailAddress": { "bsonType": ["string", "null"] }
  },
  "required": ["firstName", "age"]
}
```

You can drop the result straight into a collection validator:

```rust,ignore
db.create_collection("users")
    .validator(doc! { "$jsonSchema": bson::to_document(&User::mongo_json_schema())? })
    .await?;
```

## Implementing `Schema` by hand

For types you don't own — or when you want full control — implement the trait
directly:

```rust
use mongo_json_schema::{Schema, SchemaObject};

struct MyId(String);

impl Schema for MyId {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject::of_bson_type("objectId")
    }
}
```

## Supported `serde` attributes

Container:

- `rename_all`, `rename_all_fields`
- `tag`, `content`, `untagged` (all four enum representations)
- `deny_unknown_fields` → `additionalProperties: false`
- `default` → all fields optional
- `transparent`

Field:

- `rename` (including `rename(serialize = ..., deserialize = ...)`)
- `skip`, `skip_serializing`, `skip_deserializing`
- `flatten`
- `default` → field not `required`

Unknown `serde` attributes are ignored, so they can coexist with a real
`#[derive(Serialize, Deserialize)]`.

## `mongo_json_schema`-specific attributes

- `#[schema(description = "...")]`, `#[schema(title = "...")]` — on containers
  and fields. Doc comments are used as the `description` when no explicit one is
  given.
- `#[schema(bson_type = "objectId")]` — force a field's `bsonType`.
- `#[schema(crate = "path::to::mongo_json_schema")]` — override the crate path
  (for re-exports).

### Field validation keywords

Attach validation constraints to any field via `#[schema(...)]`; they are
written onto that field's schema:

| Keyword                                  | Applies to | Schema key                             |
| ---------------------------------------- | ---------- | -------------------------------------- |
| `minimum`, `maximum`                     | numbers    | `minimum`, `maximum`                   |
| `exclusive_minimum`, `exclusive_maximum` | numbers    | `exclusiveMinimum`, `exclusiveMaximum` |
| `multiple_of`                            | numbers    | `multipleOf`                           |
| `min_length`, `max_length`               | strings    | `minLength`, `maxLength`               |
| `pattern`                                | strings    | `pattern`                              |
| `min_items`, `max_items`, `unique_items` | arrays     | `minItems`, `maxItems`, `uniqueItems`  |
| `min_properties`, `max_properties`       | objects    | `minProperties`, `maxProperties`       |

Numeric values accept integer, float, negative, or const-expression forms:

```rust
use mongo_json_schema::Schema;

#[derive(Schema)]
struct Product {
    #[schema(minimum = 0, maximum = 120)]
    age: u32,
    #[schema(min_length = 1, max_length = 64, pattern = "^[a-z0-9-]+$")]
    slug: String,
    #[schema(min_items = 1, unique_items = true)]
    tags: Vec<String>,
    // Constraints combine with `Option`: the field stays nullable and optional.
    #[schema(minimum = 1)]
    score: Option<i32>,
}
```

`age` becomes `{ "bsonType": "long", "minimum": 0.0, "maximum": 120.0 }`, and
`score` becomes `{ "bsonType": ["int", "null"], "minimum": 1.0 }`.

For anything not covered here, implement `Schema` by hand or build a
`SchemaObject` directly — every keyword field is public.

## Type mapping

| Rust                                                          | `bsonType`                        |
| ------------------------------------------------------------- | --------------------------------- |
| `String`, `&str`, `char`, `Path`, `IpAddr`, ...               | `string`                          |
| `i8`/`i16`/`i32`/`u8`/`u16`                                   | `int`                             |
| `i64`/`u32`/`u64`/`isize`/`usize`/`i128`/`u128`               | `long`                            |
| `f32`/`f64`                                                   | `double`                          |
| `bool`                                                        | `bool`                            |
| `()`, `PhantomData`                                           | `null`                            |
| `Vec<T>`, `[T]`, `VecDeque<T>`, `[T; N]`                      | `array`                           |
| `HashSet<T>`, `BTreeSet<T>`                                   | `array` + `uniqueItems`           |
| `HashMap<_, V>`, `BTreeMap<_, V>`                             | `object` + `additionalProperties` |
| tuples                                                        | `array` (fixed length)            |
| `Option<T>`                                                   | `T` with `null` added             |
| `Box`/`Rc`/`Arc`/`Cow`/`&T`/`Cell`/`RefCell`/`Mutex`/`RwLock` | delegates to inner                |
| `chrono::DateTime<Tz>`                                        | `date`                            |
| `bson::oid::ObjectId`                                         | `objectId`                        |
| `bson::Decimal128`                                            | `decimal`                         |
| `bson::DateTime`                                              | `date`                            |
| `bson::Binary`, `bson::Uuid`                                  | `binData`                         |
| `bson::Timestamp`                                             | `timestamp`                       |

## License

MIT. Portions are inspired by / ported from `schemars` and `serde`, both MIT.
See [LICENSE](LICENSE).
