//! Schemas for common `std`/`core`/`alloc` container and wrapper types.

use crate::schema::{BoolOrSchema, Map, SchemaObject, SingleOrVec};
use crate::Schema;

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};

// --- `Option<T>`: the only type that reports itself as optional. ---

impl<T: Schema> Schema for Option<T> {
    fn mongo_json_schema() -> SchemaObject {
        T::mongo_json_schema().optional()
    }

    fn _mongo_is_option() -> bool {
        true
    }
}

// --- Transparent wrappers: delegate to the inner type. ---

macro_rules! wrapper_impl {
    ($($wrapper:ident),* $(,)?) => {
        $(
            impl<T: Schema + ?Sized> Schema for $wrapper<T> {
                fn mongo_json_schema() -> SchemaObject {
                    T::mongo_json_schema()
                }
                fn _mongo_is_option() -> bool {
                    T::_mongo_is_option()
                }
            }
        )*
    };
}

wrapper_impl!(Box, Rc, Arc);

macro_rules! sized_wrapper_impl {
    ($($wrapper:ident),* $(,)?) => {
        $(
            impl<T: Schema> Schema for $wrapper<T> {
                fn mongo_json_schema() -> SchemaObject {
                    T::mongo_json_schema()
                }
                fn _mongo_is_option() -> bool {
                    T::_mongo_is_option()
                }
            }
        )*
    };
}

sized_wrapper_impl!(Cell, RefCell, Mutex, RwLock);

impl<'a, T: Schema + ToOwned + ?Sized> Schema for Cow<'a, T> {
    fn mongo_json_schema() -> SchemaObject {
        T::mongo_json_schema()
    }
    fn _mongo_is_option() -> bool {
        T::_mongo_is_option()
    }
}

impl<T: Schema + ?Sized> Schema for &'_ T {
    fn mongo_json_schema() -> SchemaObject {
        T::mongo_json_schema()
    }
    fn _mongo_is_option() -> bool {
        T::_mongo_is_option()
    }
}

impl<T: Schema + ?Sized> Schema for &'_ mut T {
    fn mongo_json_schema() -> SchemaObject {
        T::mongo_json_schema()
    }
    fn _mongo_is_option() -> bool {
        T::_mongo_is_option()
    }
}

// --- Sequences: arrays. ---

fn array_of<T: Schema>(unique: bool) -> SchemaObject {
    SchemaObject {
        items: Some(SingleOrVec::Single(Box::new(T::mongo_json_schema()))),
        unique_items: if unique { Some(true) } else { None },
        ..SchemaObject::of_bson_type("array")
    }
}

macro_rules! seq_impl {
    ($($ty:ident),* $(,)?) => {
        $(
            impl<T: Schema> Schema for $ty<T> {
                fn mongo_json_schema() -> SchemaObject {
                    array_of::<T>(false)
                }
            }
        )*
    };
}

seq_impl!(Vec, VecDeque);

impl<T: Schema> Schema for [T] {
    fn mongo_json_schema() -> SchemaObject {
        array_of::<T>(false)
    }
}

macro_rules! set_impl {
    ($($ty:ident),* $(,)?) => {
        $(
            impl<T: Schema> Schema for $ty<T> {
                fn mongo_json_schema() -> SchemaObject {
                    array_of::<T>(true)
                }
            }
        )*
    };
}

set_impl!(BTreeSet);

impl<T: Schema, S> Schema for HashSet<T, S> {
    fn mongo_json_schema() -> SchemaObject {
        array_of::<T>(true)
    }
}

// Fixed-size arrays `[T; N]`.
impl<T: Schema, const N: usize> Schema for [T; N] {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject {
            items: Some(SingleOrVec::Single(Box::new(T::mongo_json_schema()))),
            min_items: Some(N as u64),
            max_items: Some(N as u64),
            ..SchemaObject::of_bson_type("array")
        }
    }
}

// --- Maps with string keys: objects with `additionalProperties`. ---

fn map_of<V: Schema>() -> SchemaObject {
    SchemaObject {
        additional_properties: Some(Box::new(BoolOrSchema::Schema(Box::new(
            V::mongo_json_schema(),
        )))),
        ..SchemaObject::of_bson_type("object")
    }
}

impl<K, V: Schema, S> Schema for HashMap<K, V, S> {
    fn mongo_json_schema() -> SchemaObject {
        map_of::<V>()
    }
}

impl<K, V: Schema> Schema for BTreeMap<K, V> {
    fn mongo_json_schema() -> SchemaObject {
        map_of::<V>()
    }
}

// --- Tuples: fixed-length heterogeneous arrays. ---

impl Schema for () {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject::of_bson_type("null")
    }
}

macro_rules! tuple_impl {
    ($len:literal => ($($name:ident),+)) => {
        impl<$($name: Schema),+> Schema for ($($name,)+) {
            fn mongo_json_schema() -> SchemaObject {
                SchemaObject {
                    items: Some(SingleOrVec::Vec(vec![$($name::mongo_json_schema()),+])),
                    min_items: Some($len),
                    max_items: Some($len),
                    additional_items: Some(Box::new(BoolOrSchema::Bool(false))),
                    ..SchemaObject::of_bson_type("array")
                }
            }
        }
    };
}

tuple_impl!(1 => (T0));
tuple_impl!(2 => (T0, T1));
tuple_impl!(3 => (T0, T1, T2));
tuple_impl!(4 => (T0, T1, T2, T3));
tuple_impl!(5 => (T0, T1, T2, T3, T4));
tuple_impl!(6 => (T0, T1, T2, T3, T4, T5));
tuple_impl!(7 => (T0, T1, T2, T3, T4, T5, T6));
tuple_impl!(8 => (T0, T1, T2, T3, T4, T5, T6, T7));

// --- String-like path/OS types serialize as strings. ---

macro_rules! string_impl {
    ($($ty:ty),* $(,)?) => {
        $(
            impl Schema for $ty {
                fn mongo_json_schema() -> SchemaObject {
                    SchemaObject::of_bson_type("string")
                }
            }
        )*
    };
}

string_impl!(PathBuf, Path, OsString, OsStr);

// --- Networking / misc scalar types (serde serializes these as strings). ---

string_impl!(
    std::net::IpAddr,
    std::net::Ipv4Addr,
    std::net::Ipv6Addr,
    std::net::SocketAddr,
    std::net::SocketAddrV4,
    std::net::SocketAddrV6,
);

// --- `Duration`: serde serializes it as `{ secs, nanos }`. ---

impl Schema for std::time::Duration {
    fn mongo_json_schema() -> SchemaObject {
        let mut properties = Map::new();
        properties.insert("secs".to_owned(), SchemaObject::of_bson_type("long"));
        properties.insert("nanos".to_owned(), SchemaObject::of_bson_type("long"));
        SchemaObject {
            properties: Some(properties),
            required: Some(vec!["secs".to_owned(), "nanos".to_owned()]),
            ..SchemaObject::of_bson_type("object")
        }
    }
}

// --- `PhantomData` serializes as null. ---

impl<T: ?Sized> Schema for std::marker::PhantomData<T> {
    fn mongo_json_schema() -> SchemaObject {
        SchemaObject::of_bson_type("null")
    }
}
