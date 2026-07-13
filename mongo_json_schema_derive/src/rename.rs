//! Faithful re-implementation of serde's `rename_all` rules.
//!
//! Ported from `serde_derive`'s `internals::case` (MIT/Apache-2.0). The two
//! `apply_to_*` methods differ because serde assumes fields are `snake_case` and
//! variants are `PascalCase` in the source.

/// The rename rule requested via `#[serde(rename_all = "...")]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)] // names deliberately mirror serde's rules
pub enum RenameRule {
    LowerCase,
    UpperCase,
    PascalCase,
    CamelCase,
    SnakeCase,
    ScreamingSnakeCase,
    KebabCase,
    ScreamingKebabCase,
}

impl RenameRule {
    /// Parses a serde rename-rule string, returning `None` if unrecognized.
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "lowercase" => RenameRule::LowerCase,
            "UPPERCASE" => RenameRule::UpperCase,
            "PascalCase" => RenameRule::PascalCase,
            "camelCase" => RenameRule::CamelCase,
            "snake_case" => RenameRule::SnakeCase,
            "SCREAMING_SNAKE_CASE" => RenameRule::ScreamingSnakeCase,
            "kebab-case" => RenameRule::KebabCase,
            "SCREAMING-KEBAB-CASE" => RenameRule::ScreamingKebabCase,
            _ => return None,
        })
    }

    /// Applies the rule to a variant name (assumed `PascalCase` in source).
    pub fn apply_to_variant(self, variant: &str) -> String {
        use RenameRule::*;
        match self {
            PascalCase => variant.to_owned(),
            LowerCase => variant.to_ascii_lowercase(),
            UpperCase => variant.to_ascii_uppercase(),
            CamelCase => variant[..1].to_ascii_lowercase() + &variant[1..],
            SnakeCase => {
                let mut snake = String::new();
                for (i, ch) in variant.char_indices() {
                    if i > 0 && ch.is_uppercase() {
                        snake.push('_');
                    }
                    snake.push(ch.to_ascii_lowercase());
                }
                snake
            }
            ScreamingSnakeCase => SnakeCase.apply_to_variant(variant).to_ascii_uppercase(),
            KebabCase => SnakeCase.apply_to_variant(variant).replace('_', "-"),
            ScreamingKebabCase => ScreamingSnakeCase
                .apply_to_variant(variant)
                .replace('_', "-"),
        }
    }

    /// Applies the rule to a field name (assumed `snake_case` in source).
    pub fn apply_to_field(self, field: &str) -> String {
        use RenameRule::*;
        match self {
            LowerCase | SnakeCase => field.to_owned(),
            UpperCase | ScreamingSnakeCase => field.to_ascii_uppercase(),
            PascalCase => {
                let mut pascal = String::new();
                let mut capitalize = true;
                for ch in field.chars() {
                    if ch == '_' {
                        capitalize = true;
                    } else if capitalize {
                        pascal.push(ch.to_ascii_uppercase());
                        capitalize = false;
                    } else {
                        pascal.push(ch);
                    }
                }
                pascal
            }
            CamelCase => {
                let pascal = PascalCase.apply_to_field(field);
                pascal[..1].to_ascii_lowercase() + &pascal[1..]
            }
            KebabCase => field.replace('_', "-"),
            ScreamingKebabCase => UpperCase.apply_to_field(field).replace('_', "-"),
        }
    }
}
