//! Derive macro for `mongo_json_schema::Schema`.
//!
//! See the `mongo_json_schema` crate documentation for usage.

mod attr;
mod rename;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, Data, DataEnum, DeriveInput, Fields, FieldsNamed, FieldsUnnamed,
};

use attr::{parse_container, parse_field, parse_variant, ContainerAttr, FieldValidation, TagType};
use rename::RenameRule;

#[proc_macro_derive(Schema, attributes(serde, schema))]
pub fn derive_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match derive_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_impl(input: DeriveInput) -> syn::Result<TokenStream2> {
    let container = parse_container(&input.attrs)?;

    let cr: TokenStream2 = match &container.crate_path {
        Some(path) => {
            let path: syn::Path = syn::parse_str(path)?;
            quote!(#path)
        }
        None => quote!(::mongo_json_schema),
    };

    let body = match &input.data {
        Data::Struct(data) => schema_for_struct(&cr, &data.fields, &container)?,
        Data::Enum(data) => schema_for_enum(&cr, data, &container)?,
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                &input,
                "`Schema` cannot be derived for unions",
            ))
        }
    };

    let body = with_meta(body, &container);

    // Add a `T: Schema` bound for every generic type parameter.
    let ident = &input.ident;
    let mut generics = input.generics.clone();
    let type_idents: Vec<_> = input
        .generics
        .type_params()
        .map(|tp| tp.ident.clone())
        .collect();
    if !type_idents.is_empty() {
        let where_clause = generics.make_where_clause();
        for id in &type_idents {
            where_clause.predicates.push(parse_quote!(#id: #cr::Schema));
        }
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #cr::Schema for #ident #ty_generics #where_clause {
            fn mongo_json_schema() -> #cr::SchemaObject {
                #body
            }
        }
    })
}

/// Wraps a schema expression to apply container-level `description`/`title`.
fn with_meta(base: TokenStream2, container: &ContainerAttr) -> TokenStream2 {
    let mut setters = Vec::new();
    if let Some(desc) = &container.description {
        setters
            .push(quote! { schema.description = ::std::option::Option::Some(#desc.to_owned()); });
    }
    if let Some(title) = &container.title {
        setters.push(quote! { schema.title = ::std::option::Option::Some(#title.to_owned()); });
    }
    if setters.is_empty() {
        return base;
    }
    quote! {
        {
            let mut schema = #base;
            #(#setters)*
            schema
        }
    }
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

fn schema_for_struct(
    cr: &TokenStream2,
    fields: &Fields,
    container: &ContainerAttr,
) -> syn::Result<TokenStream2> {
    match fields {
        Fields::Named(named) => {
            // `#[serde(transparent)]` on a single-field struct delegates to that field.
            if container.transparent {
                if let Some(field) = single_included_field(&named.named)? {
                    let ty = &field.ty;
                    return Ok(quote! { <#ty as #cr::Schema>::mongo_json_schema() });
                }
            }
            object_schema_expr(
                cr,
                named,
                container.rename_all,
                container.default,
                container.deny_unknown_fields,
                None,
            )
        }
        Fields::Unnamed(unnamed) => {
            // A single-field tuple struct is a serde newtype: transparent.
            if unnamed.unnamed.len() == 1 {
                let ty = &unnamed.unnamed[0].ty;
                Ok(quote! { <#ty as #cr::Schema>::mongo_json_schema() })
            } else {
                Ok(tuple_schema_expr(cr, unnamed))
            }
        }
        Fields::Unit => Ok(quote! { #cr::SchemaObject::of_bson_type("null") }),
    }
}

/// Returns the sole non-skipped field of a struct, if there is exactly one.
fn single_included_field(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> syn::Result<Option<&syn::Field>> {
    let mut included = Vec::new();
    for field in fields {
        if !parse_field(&field.attrs)?.is_skipped() {
            included.push(field);
        }
    }
    Ok(if included.len() == 1 {
        Some(included[0])
    } else {
        None
    })
}

/// Builds an object-typed schema for a set of named fields.
///
/// `injected_tag` adds a leading `tag -> { enum: [variant] }` property, used for
/// internally tagged enums.
fn object_schema_expr(
    cr: &TokenStream2,
    fields: &FieldsNamed,
    rename_all: Option<RenameRule>,
    container_default: bool,
    deny_unknown: bool,
    injected_tag: Option<(&str, &str)>,
) -> syn::Result<TokenStream2> {
    let mut stmts: Vec<TokenStream2> = Vec::new();

    if let Some((tag, variant_name)) = injected_tag {
        let tag_schema = variant_string_schema(cr, variant_name);
        stmts.push(quote! {
            properties.insert(#tag.to_owned(), #tag_schema);
            required.push(#tag.to_owned());
        });
    }

    for field in &fields.named {
        let fattr = parse_field(&field.attrs)?;
        if fattr.is_skipped() {
            continue;
        }
        let ident = field.ident.as_ref().unwrap();
        let ty = &field.ty;

        if fattr.flatten {
            stmts.push(quote! {
                {
                    let flat = <#ty as #cr::Schema>::mongo_json_schema();
                    if let ::std::option::Option::Some(p) = flat.properties {
                        properties.extend(p);
                    }
                    if let ::std::option::Option::Some(r) = flat.required {
                        required.extend(r);
                    }
                    if schema.additional_properties.is_none() {
                        schema.additional_properties = flat.additional_properties;
                    }
                    if schema.pattern_properties.is_none() {
                        schema.pattern_properties = flat.pattern_properties;
                    }
                }
            });
            continue;
        }

        let key = field_key(&ident.to_string(), &fattr.rename, rename_all);

        // A `bson_type` override builds the field schema from scratch, so the
        // field's type need not implement `Schema` at all. Otherwise we start
        // from the type's own derived/manual schema.
        let field_schema_init = if let Some(bt) = &fattr.bson_type {
            quote! { let mut field_schema = #cr::SchemaObject::of_bson_type(#bt); }
        } else {
            quote! { let mut field_schema = <#ty as #cr::Schema>::mongo_json_schema(); }
        };

        let mut field_build: Vec<TokenStream2> = Vec::new();
        if let Some(desc) = &fattr.description {
            field_build
                .push(quote! { field_schema.description = ::std::option::Option::Some(#desc.to_owned()); });
        }
        if let Some(title) = &fattr.title {
            field_build.push(
                quote! { field_schema.title = ::std::option::Option::Some(#title.to_owned()); },
            );
        }
        field_build.extend(validation_setters(&fattr.validation));

        // A field is required unless it (or the container) has a default, or it is
        // an `Option`. With a `bson_type` override the type is not consulted (it
        // may not implement `Schema`), so the field is treated as required.
        let required_stmt = if container_default || fattr.default {
            quote! {}
        } else if fattr.bson_type.is_some() {
            quote! { required.push(#key.to_owned()); }
        } else {
            quote! {
                if !<#ty as #cr::Schema>::_mongo_is_option() {
                    required.push(#key.to_owned());
                }
            }
        };

        stmts.push(quote! {
            {
                #field_schema_init
                #(#field_build)*
                properties.insert(#key.to_owned(), field_schema);
            }
            #required_stmt
        });
    }

    Ok(quote! {
        {
            let mut schema = #cr::SchemaObject::of_bson_type("object");
            let mut properties = #cr::Map::<::std::string::String, #cr::SchemaObject>::new();
            let mut required = ::std::vec::Vec::<::std::string::String>::new();
            #(#stmts)*
            if !properties.is_empty() {
                schema.properties = ::std::option::Option::Some(properties);
            }
            if !required.is_empty() {
                schema.required = ::std::option::Option::Some(required);
            }
            if #deny_unknown && schema.additional_properties.is_none() {
                schema.additional_properties =
                    ::std::option::Option::Some(::std::boxed::Box::new(#cr::BoolOrSchema::Bool(false)));
            }
            schema
        }
    })
}

fn tuple_schema_expr(cr: &TokenStream2, fields: &FieldsUnnamed) -> TokenStream2 {
    let types: Vec<_> = fields.unnamed.iter().map(|f| &f.ty).collect();
    let len = types.len() as u64;
    let items = types
        .iter()
        .map(|ty| quote! { <#ty as #cr::Schema>::mongo_json_schema() });
    quote! {
        #cr::SchemaObject {
            items: ::std::option::Option::Some(#cr::SingleOrVec::Vec(::std::vec![ #(#items),* ])),
            min_items: ::std::option::Option::Some(#len),
            max_items: ::std::option::Option::Some(#len),
            additional_items: ::std::option::Option::Some(
                ::std::boxed::Box::new(#cr::BoolOrSchema::Bool(false))
            ),
            ..#cr::SchemaObject::of_bson_type("array")
        }
    }
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// A non-skipped enum variant paired with its serialized name and the rename
/// rule that applies to its (struct-variant) fields.
struct VariantInfo<'a> {
    variant: &'a syn::Variant,
    name: String,
    rename_all_fields: Option<RenameRule>,
}

fn schema_for_enum(
    cr: &TokenStream2,
    data: &DataEnum,
    container: &ContainerAttr,
) -> syn::Result<TokenStream2> {
    // Gather non-skipped variants with their serialized names.
    let mut variants: Vec<VariantInfo> = Vec::new();
    for variant in &data.variants {
        let vattr = parse_variant(&variant.attrs)?;
        if vattr.is_skipped() {
            continue;
        }
        let name = vattr
            .rename
            .clone()
            .unwrap_or_else(|| match container.rename_all {
                Some(rule) => rule.apply_to_variant(&variant.ident.to_string()),
                None => variant.ident.to_string(),
            });
        let rename_all_fields = vattr.rename_all.or(container.rename_all_fields);
        variants.push(VariantInfo {
            variant,
            name,
            rename_all_fields,
        });
    }

    match container.tag_type() {
        TagType::External => enum_external(cr, &variants),
        TagType::Internal { tag } => enum_internal(cr, &variants, &tag),
        TagType::Adjacent { tag, content } => enum_adjacent(cr, &variants, &tag, &content),
        TagType::None => enum_untagged(cr, &variants),
    }
}

/// `{ "bsonType": "string", "enum": ["<name>"] }`.
fn variant_string_schema(cr: &TokenStream2, name: &str) -> TokenStream2 {
    quote! {
        #cr::SchemaObject {
            enum_values: ::std::option::Option::Some(::std::vec![
                #cr::_private::serde_json::Value::String(#name.to_owned())
            ]),
            ..#cr::SchemaObject::of_bson_type("string")
        }
    }
}

/// The data-carrying part of a variant, ignoring the tag.
fn variant_payload_expr(
    cr: &TokenStream2,
    variant: &syn::Variant,
    rename_all_fields: Option<RenameRule>,
) -> syn::Result<TokenStream2> {
    match &variant.fields {
        Fields::Unit => Ok(quote! { #cr::SchemaObject::of_bson_type("null") }),
        Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => {
            let ty = &unnamed.unnamed[0].ty;
            Ok(quote! { <#ty as #cr::Schema>::mongo_json_schema() })
        }
        Fields::Unnamed(unnamed) => Ok(tuple_schema_expr(cr, unnamed)),
        Fields::Named(named) => {
            object_schema_expr(cr, named, rename_all_fields, false, false, None)
        }
    }
}

fn enum_external(cr: &TokenStream2, variants: &[VariantInfo]) -> syn::Result<TokenStream2> {
    let all_unit = variants
        .iter()
        .all(|v| matches!(v.variant.fields, Fields::Unit));

    if all_unit {
        let names = variants.iter().map(|v| {
            let n = &v.name;
            quote! { #cr::_private::serde_json::Value::String(#n.to_owned()) }
        });
        return Ok(quote! {
            #cr::SchemaObject {
                enum_values: ::std::option::Option::Some(::std::vec![ #(#names),* ]),
                ..#cr::SchemaObject::of_bson_type("string")
            }
        });
    }

    let mut branches = Vec::new();
    for v in variants {
        let name = &v.name;
        let branch = match &v.variant.fields {
            Fields::Unit => variant_string_schema(cr, name),
            _ => {
                let payload = variant_payload_expr(cr, v.variant, v.rename_all_fields)?;
                quote! {
                    {
                        let mut props = #cr::Map::<::std::string::String, #cr::SchemaObject>::new();
                        props.insert(#name.to_owned(), #payload);
                        #cr::SchemaObject {
                            properties: ::std::option::Option::Some(props),
                            required: ::std::option::Option::Some(::std::vec![#name.to_owned()]),
                            additional_properties: ::std::option::Option::Some(
                                ::std::boxed::Box::new(#cr::BoolOrSchema::Bool(false))
                            ),
                            ..#cr::SchemaObject::of_bson_type("object")
                        }
                    }
                }
            }
        };
        branches.push(branch);
    }

    Ok(quote! {
        #cr::SchemaObject {
            one_of: ::std::option::Option::Some(::std::vec![ #(#branches),* ]),
            ..::std::default::Default::default()
        }
    })
}

fn enum_internal(
    cr: &TokenStream2,
    variants: &[VariantInfo],
    tag: &str,
) -> syn::Result<TokenStream2> {
    let mut branches = Vec::new();
    for v in variants {
        let name = &v.name;
        let tag_schema = variant_string_schema(cr, name);
        let branch = match &v.variant.fields {
            Fields::Unit => quote! {
                {
                    let mut props = #cr::Map::<::std::string::String, #cr::SchemaObject>::new();
                    props.insert(#tag.to_owned(), #tag_schema);
                    #cr::SchemaObject {
                        properties: ::std::option::Option::Some(props),
                        required: ::std::option::Option::Some(::std::vec![#tag.to_owned()]),
                        ..#cr::SchemaObject::of_bson_type("object")
                    }
                }
            },
            Fields::Named(named) => object_schema_expr(
                cr,
                named,
                v.rename_all_fields,
                false,
                false,
                Some((tag, name)),
            )?,
            Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => {
                // Internally tagged newtype: merge the tag into the inner object.
                let ty = &unnamed.unnamed[0].ty;
                quote! {
                    {
                        let mut s = <#ty as #cr::Schema>::mongo_json_schema();
                        let mut new_props = #cr::Map::<::std::string::String, #cr::SchemaObject>::new();
                        new_props.insert(#tag.to_owned(), #tag_schema);
                        if let ::std::option::Option::Some(p) = s.properties.take() {
                            new_props.extend(p);
                        }
                        s.properties = ::std::option::Option::Some(new_props);
                        let mut req = s.required.take().unwrap_or_default();
                        req.insert(0, #tag.to_owned());
                        s.required = ::std::option::Option::Some(req);
                        s.bson_type = ::std::option::Option::Some(
                            #cr::SingleOrVec::Single(::std::boxed::Box::new("object".to_owned()))
                        );
                        s
                    }
                }
            }
            Fields::Unnamed(_) => {
                return Err(syn::Error::new_spanned(
                    v.variant,
                    "internally tagged enums do not support tuple variants",
                ))
            }
        };
        branches.push(branch);
    }

    Ok(quote! {
        #cr::SchemaObject {
            one_of: ::std::option::Option::Some(::std::vec![ #(#branches),* ]),
            ..::std::default::Default::default()
        }
    })
}

fn enum_adjacent(
    cr: &TokenStream2,
    variants: &[VariantInfo],
    tag: &str,
    content: &str,
) -> syn::Result<TokenStream2> {
    let mut branches = Vec::new();
    for v in variants {
        let name = &v.name;
        let tag_schema = variant_string_schema(cr, name);
        let branch = match &v.variant.fields {
            Fields::Unit => quote! {
                {
                    let mut props = #cr::Map::<::std::string::String, #cr::SchemaObject>::new();
                    props.insert(#tag.to_owned(), #tag_schema);
                    #cr::SchemaObject {
                        properties: ::std::option::Option::Some(props),
                        required: ::std::option::Option::Some(::std::vec![#tag.to_owned()]),
                        ..#cr::SchemaObject::of_bson_type("object")
                    }
                }
            },
            _ => {
                let payload = variant_payload_expr(cr, v.variant, v.rename_all_fields)?;
                quote! {
                    {
                        let mut props = #cr::Map::<::std::string::String, #cr::SchemaObject>::new();
                        props.insert(#tag.to_owned(), #tag_schema);
                        props.insert(#content.to_owned(), #payload);
                        #cr::SchemaObject {
                            properties: ::std::option::Option::Some(props),
                            required: ::std::option::Option::Some(
                                ::std::vec![#tag.to_owned(), #content.to_owned()]
                            ),
                            ..#cr::SchemaObject::of_bson_type("object")
                        }
                    }
                }
            }
        };
        branches.push(branch);
    }

    Ok(quote! {
        #cr::SchemaObject {
            one_of: ::std::option::Option::Some(::std::vec![ #(#branches),* ]),
            ..::std::default::Default::default()
        }
    })
}

fn enum_untagged(cr: &TokenStream2, variants: &[VariantInfo]) -> syn::Result<TokenStream2> {
    let mut branches = Vec::new();
    for v in variants {
        branches.push(variant_payload_expr(cr, v.variant, v.rename_all_fields)?);
    }
    Ok(quote! {
        #cr::SchemaObject {
            any_of: ::std::option::Option::Some(::std::vec![ #(#branches),* ]),
            ..::std::default::Default::default()
        }
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Emits assignment statements applying `#[schema(...)]` validation keywords to
/// a local `field_schema` variable.
fn validation_setters(v: &FieldValidation) -> Vec<TokenStream2> {
    let mut out = Vec::new();
    // Numeric keywords are cast to f64 (the schema field type); count keywords
    // are used as-is (inferred `u64`).
    macro_rules! number {
        ($opt:expr, $field:ident) => {
            if let Some(tokens) = &$opt {
                out.push(quote! {
                    field_schema.$field = ::std::option::Option::Some((#tokens) as f64);
                });
            }
        };
    }
    macro_rules! count {
        ($opt:expr, $field:ident) => {
            if let Some(tokens) = &$opt {
                out.push(quote! {
                    field_schema.$field = ::std::option::Option::Some(#tokens);
                });
            }
        };
    }

    number!(v.minimum, minimum);
    number!(v.maximum, maximum);
    number!(v.exclusive_minimum, exclusive_minimum);
    number!(v.exclusive_maximum, exclusive_maximum);
    number!(v.multiple_of, multiple_of);

    count!(v.min_length, min_length);
    count!(v.max_length, max_length);
    count!(v.min_items, min_items);
    count!(v.max_items, max_items);
    count!(v.min_properties, min_properties);
    count!(v.max_properties, max_properties);
    count!(v.unique_items, unique_items);

    if let Some(pattern) = &v.pattern {
        out.push(quote! {
            field_schema.pattern = ::std::option::Option::Some(#pattern.to_owned());
        });
    }
    out
}

/// Computes a field's serialized key from its Rust name, explicit `rename`, and
/// the container `rename_all` rule.
fn field_key(ident: &str, rename: &Option<String>, rename_all: Option<RenameRule>) -> String {
    if let Some(rename) = rename {
        return rename.clone();
    }
    let base = ident.strip_prefix("r#").unwrap_or(ident);
    match rename_all {
        Some(rule) => rule.apply_to_field(base),
        None => base.to_owned(),
    }
}
