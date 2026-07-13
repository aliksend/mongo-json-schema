//! Parsing of `#[serde(...)]` and `#[schema(...)]` attributes.
//!
//! Only the subset relevant to schema generation is understood. Unknown keys are
//! ignored (rather than erroring) so that arbitrary serde attributes can coexist.

use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::{Attribute, LitStr};

use crate::rename::RenameRule;

/// How a serde enum is tagged, derived from `tag`/`content`/`untagged`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagType {
    /// Default: `{ "Variant": <data> }`.
    External,
    /// `#[serde(tag = "...")]`.
    Internal { tag: String },
    /// `#[serde(tag = "...", content = "...")]`.
    Adjacent { tag: String, content: String },
    /// `#[serde(untagged)]`.
    None,
}

/// Attributes parsed from a struct/enum (container) declaration.
#[derive(Debug, Default)]
pub struct ContainerAttr {
    pub rename_all: Option<RenameRule>,
    pub rename_all_fields: Option<RenameRule>,
    pub tag: Option<String>,
    pub content: Option<String>,
    pub untagged: bool,
    pub deny_unknown_fields: bool,
    pub transparent: bool,
    pub default: bool,
    pub description: Option<String>,
    pub title: Option<String>,
    /// Path to the `mongo_json_schema` crate (override via `#[schema(crate = "...")]`).
    pub crate_path: Option<String>,
}

impl ContainerAttr {
    pub fn tag_type(&self) -> TagType {
        if self.untagged {
            TagType::None
        } else {
            match (&self.tag, &self.content) {
                (Some(tag), Some(content)) => TagType::Adjacent {
                    tag: tag.clone(),
                    content: content.clone(),
                },
                (Some(tag), None) => TagType::Internal { tag: tag.clone() },
                _ => TagType::External,
            }
        }
    }
}

/// Validation keywords settable on a field via `#[schema(...)]`. Numeric bounds
/// are stored as raw token streams so integer/float/negative literals and const
/// expressions all work; they are emitted with the appropriate cast.
#[derive(Debug, Default)]
pub struct FieldValidation {
    pub minimum: Option<TokenStream2>,
    pub maximum: Option<TokenStream2>,
    pub exclusive_minimum: Option<TokenStream2>,
    pub exclusive_maximum: Option<TokenStream2>,
    pub multiple_of: Option<TokenStream2>,
    pub min_length: Option<TokenStream2>,
    pub max_length: Option<TokenStream2>,
    pub pattern: Option<String>,
    pub min_items: Option<TokenStream2>,
    pub max_items: Option<TokenStream2>,
    pub unique_items: Option<TokenStream2>,
    pub min_properties: Option<TokenStream2>,
    pub max_properties: Option<TokenStream2>,
}

/// Attributes parsed from a struct field.
#[derive(Debug, Default)]
pub struct FieldAttr {
    pub rename: Option<String>,
    pub skip: bool,
    pub skip_serializing: bool,
    pub skip_deserializing: bool,
    pub flatten: bool,
    pub default: bool,
    pub description: Option<String>,
    pub title: Option<String>,
    /// `#[schema(bson_type = "...")]`: force the field's `bsonType`.
    pub bson_type: Option<String>,
    pub validation: FieldValidation,
}

impl FieldAttr {
    /// A field is entirely absent from the schema if serde skips it in both
    /// directions.
    pub fn is_skipped(&self) -> bool {
        self.skip || (self.skip_serializing && self.skip_deserializing)
    }
}

/// Attributes parsed from an enum variant.
#[derive(Debug, Default)]
pub struct VariantAttr {
    pub rename: Option<String>,
    pub rename_all: Option<RenameRule>,
    pub skip: bool,
    pub skip_serializing: bool,
    pub skip_deserializing: bool,
    pub description: Option<String>,
    pub title: Option<String>,
}

impl VariantAttr {
    pub fn is_skipped(&self) -> bool {
        self.skip || (self.skip_serializing && self.skip_deserializing)
    }
}

/// Extracts and concatenates `///` doc-comment lines into a single string.
fn doc_string(attrs: &[Attribute]) -> Option<String> {
    let mut lines: Vec<String> = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        if let syn::Meta::NameValue(nv) = &attr.meta {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            {
                lines.push(s.value().trim().to_owned());
            }
        }
    }
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n").trim().to_owned())
    }
}

pub fn parse_container(attrs: &[Attribute]) -> syn::Result<ContainerAttr> {
    let mut out = ContainerAttr {
        description: doc_string(attrs),
        ..Default::default()
    };

    for attr in attrs {
        if attr.path().is_ident("serde") {
            attr.parse_nested_meta(|meta| {
                let path = &meta.path;
                if path.is_ident("rename_all") {
                    out.rename_all = Some(parse_rename_rule(&meta)?);
                } else if path.is_ident("rename_all_fields") {
                    out.rename_all_fields = Some(parse_rename_rule(&meta)?);
                } else if path.is_ident("tag") {
                    out.tag = Some(parse_lit_str(&meta)?);
                } else if path.is_ident("content") {
                    out.content = Some(parse_lit_str(&meta)?);
                } else if path.is_ident("untagged") {
                    out.untagged = true;
                } else if path.is_ident("deny_unknown_fields") {
                    out.deny_unknown_fields = true;
                } else if path.is_ident("transparent") {
                    out.transparent = true;
                } else if path.is_ident("default") {
                    out.default = true;
                    let _ = meta.value().and_then(|v| v.parse::<LitStr>());
                } else {
                    // Ignore unknown/irrelevant serde attributes, consuming any value.
                    swallow_value(&meta);
                }
                Ok(())
            })?;
        } else if attr.path().is_ident("schema") {
            attr.parse_nested_meta(|meta| {
                let path = &meta.path;
                if path.is_ident("description") {
                    out.description = Some(parse_lit_str(&meta)?);
                } else if path.is_ident("title") {
                    out.title = Some(parse_lit_str(&meta)?);
                } else if path.is_ident("crate") {
                    out.crate_path = Some(parse_lit_str(&meta)?);
                } else {
                    swallow_value(&meta);
                }
                Ok(())
            })?;
        }
    }

    Ok(out)
}

pub fn parse_field(attrs: &[Attribute]) -> syn::Result<FieldAttr> {
    let mut out = FieldAttr {
        description: doc_string(attrs),
        ..Default::default()
    };

    for attr in attrs {
        if attr.path().is_ident("serde") {
            attr.parse_nested_meta(|meta| {
                let path = &meta.path;
                if path.is_ident("rename") {
                    out.rename = Some(parse_rename(&meta)?);
                } else if path.is_ident("skip") {
                    out.skip = true;
                } else if path.is_ident("skip_serializing") {
                    out.skip_serializing = true;
                } else if path.is_ident("skip_deserializing") {
                    out.skip_deserializing = true;
                } else if path.is_ident("flatten") {
                    out.flatten = true;
                } else if path.is_ident("default") {
                    out.default = true;
                    let _ = meta.value().and_then(|v| v.parse::<LitStr>());
                } else {
                    swallow_value(&meta);
                }
                Ok(())
            })?;
        } else if attr.path().is_ident("schema") {
            attr.parse_nested_meta(|meta| {
                let path = &meta.path;
                let v = &mut out.validation;
                if path.is_ident("description") {
                    out.description = Some(parse_lit_str(&meta)?);
                } else if path.is_ident("title") {
                    out.title = Some(parse_lit_str(&meta)?);
                } else if path.is_ident("bson_type") {
                    out.bson_type = Some(parse_lit_str(&meta)?);
                } else if path.is_ident("minimum") {
                    v.minimum = Some(parse_expr_tokens(&meta)?);
                } else if path.is_ident("maximum") {
                    v.maximum = Some(parse_expr_tokens(&meta)?);
                } else if path.is_ident("exclusive_minimum") {
                    v.exclusive_minimum = Some(parse_expr_tokens(&meta)?);
                } else if path.is_ident("exclusive_maximum") {
                    v.exclusive_maximum = Some(parse_expr_tokens(&meta)?);
                } else if path.is_ident("multiple_of") {
                    v.multiple_of = Some(parse_expr_tokens(&meta)?);
                } else if path.is_ident("min_length") {
                    v.min_length = Some(parse_expr_tokens(&meta)?);
                } else if path.is_ident("max_length") {
                    v.max_length = Some(parse_expr_tokens(&meta)?);
                } else if path.is_ident("pattern") {
                    v.pattern = Some(parse_lit_str(&meta)?);
                } else if path.is_ident("min_items") {
                    v.min_items = Some(parse_expr_tokens(&meta)?);
                } else if path.is_ident("max_items") {
                    v.max_items = Some(parse_expr_tokens(&meta)?);
                } else if path.is_ident("unique_items") {
                    v.unique_items = Some(parse_expr_tokens(&meta)?);
                } else if path.is_ident("min_properties") {
                    v.min_properties = Some(parse_expr_tokens(&meta)?);
                } else if path.is_ident("max_properties") {
                    v.max_properties = Some(parse_expr_tokens(&meta)?);
                } else {
                    swallow_value(&meta);
                }
                Ok(())
            })?;
        }
    }

    Ok(out)
}

pub fn parse_variant(attrs: &[Attribute]) -> syn::Result<VariantAttr> {
    let mut out = VariantAttr {
        description: doc_string(attrs),
        ..Default::default()
    };

    for attr in attrs {
        if attr.path().is_ident("serde") {
            attr.parse_nested_meta(|meta| {
                let path = &meta.path;
                if path.is_ident("rename") {
                    out.rename = Some(parse_rename(&meta)?);
                } else if path.is_ident("rename_all") {
                    out.rename_all = Some(parse_rename_rule(&meta)?);
                } else if path.is_ident("skip") {
                    out.skip = true;
                } else if path.is_ident("skip_serializing") {
                    out.skip_serializing = true;
                } else if path.is_ident("skip_deserializing") {
                    out.skip_deserializing = true;
                } else {
                    swallow_value(&meta);
                }
                Ok(())
            })?;
        } else if attr.path().is_ident("schema") {
            attr.parse_nested_meta(|meta| {
                let path = &meta.path;
                if path.is_ident("description") {
                    out.description = Some(parse_lit_str(&meta)?);
                } else if path.is_ident("title") {
                    out.title = Some(parse_lit_str(&meta)?);
                } else {
                    swallow_value(&meta);
                }
                Ok(())
            })?;
        }
    }

    Ok(out)
}

/// Parses the `= <expr>` value of an attribute into raw tokens (e.g. `0`,
/// `-1.5`, `MAX_LEN`).
fn parse_expr_tokens(meta: &syn::meta::ParseNestedMeta) -> syn::Result<TokenStream2> {
    let expr: syn::Expr = meta.value()?.parse()?;
    Ok(expr.to_token_stream())
}

fn parse_lit_str(meta: &syn::meta::ParseNestedMeta) -> syn::Result<String> {
    let s: LitStr = meta.value()?.parse()?;
    Ok(s.value())
}

fn parse_rename_rule(meta: &syn::meta::ParseNestedMeta) -> syn::Result<RenameRule> {
    let s: LitStr = meta.value()?.parse()?;
    RenameRule::from_str(&s.value()).ok_or_else(|| {
        syn::Error::new_spanned(&s, format!("unknown rename_all rule: {:?}", s.value()))
    })
}

/// serde's `rename` can be either `rename = "..."` or
/// `rename(serialize = "...", deserialize = "...")`. We take the serialize name
/// (falling back to deserialize) since the schema mirrors the serialized form.
fn parse_rename(meta: &syn::meta::ParseNestedMeta) -> syn::Result<String> {
    // Simple form: `rename = "..."`.
    if let Ok(value) = meta.value() {
        let s: LitStr = value.parse()?;
        return Ok(s.value());
    }
    // Nested form: `rename(serialize = "...", deserialize = "...")`.
    let mut serialize = None;
    let mut deserialize = None;
    meta.parse_nested_meta(|nested| {
        if nested.path.is_ident("serialize") {
            let s: LitStr = nested.value()?.parse()?;
            serialize = Some(s.value());
        } else if nested.path.is_ident("deserialize") {
            let s: LitStr = nested.value()?.parse()?;
            deserialize = Some(s.value());
        } else {
            swallow_value(&nested);
        }
        Ok(())
    })?;
    serialize
        .or(deserialize)
        .ok_or_else(|| meta.error("expected a rename value"))
}

/// Consumes an optional `= value` or `(...)` payload of an unrecognized key so
/// parsing can continue.
fn swallow_value(meta: &syn::meta::ParseNestedMeta) {
    if let Ok(value) = meta.value() {
        // It's a `key = <expr>`; parse and discard the expression.
        let _ = value.parse::<syn::Expr>();
    } else {
        // It might be `key(...)`; try to parse a nested group and discard.
        let _ = meta.parse_nested_meta(|nested| {
            swallow_value(&nested);
            Ok(())
        });
    }
}
