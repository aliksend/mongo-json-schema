//! Schemas for primitive/scalar types.

use crate::schema::SchemaObject;
use crate::Schema;

/// Implements [`Schema`] for a type by returning a schema with a single
/// `bsonType`.
macro_rules! bson_type_impl {
    ($($ty:ty => $bson_type:literal),* $(,)?) => {
        $(
            impl Schema for $ty {
                fn mongo_json_schema() -> SchemaObject {
                    SchemaObject::of_bson_type($bson_type)
                }
            }
        )*
    };
}

bson_type_impl! {
    str => "string",
    String => "string",
    bool => "bool",
    // Floating point numbers map to BSON `double`.
    f32 => "double",
    f64 => "double",
}

// 32-bit-or-smaller signed/unsigned integers fit in a BSON `int` (int32).
bson_type_impl! {
    i8 => "int",
    i16 => "int",
    i32 => "int",
    u8 => "int",
    u16 => "int",
}

// Wider integers map to BSON `long` (int64). `u32` is included because its
// range exceeds `i32::MAX`. `u64`/`i128`/`u128` may in theory overflow an
// int64, but `long` is the closest BSON type available.
bson_type_impl! {
    i64 => "long",
    i128 => "long",
    isize => "long",
    u32 => "long",
    u64 => "long",
    u128 => "long",
    usize => "long",
}

// `char` is a single-character string.
impl Schema for char {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject {
            min_length: Some(1),
            max_length: Some(1),
            ..SchemaObject::of_bson_type("string")
        }
    }
}

macro_rules! nonzero_impl {
    ($($nz:ty => $inner:ty),* $(,)?) => {
        $(
            impl Schema for $nz {
                fn mongo_json_schema() -> SchemaObject {
                    <$inner as Schema>::mongo_json_schema()
                }
            }
        )*
    };
}

nonzero_impl! {
    core::num::NonZeroI8 => i8,
    core::num::NonZeroI16 => i16,
    core::num::NonZeroI32 => i32,
    core::num::NonZeroI64 => i64,
    core::num::NonZeroI128 => i128,
    core::num::NonZeroIsize => isize,
    core::num::NonZeroU8 => u8,
    core::num::NonZeroU16 => u16,
    core::num::NonZeroU32 => u32,
    core::num::NonZeroU64 => u64,
    core::num::NonZeroU128 => u128,
    core::num::NonZeroUsize => usize,
}
