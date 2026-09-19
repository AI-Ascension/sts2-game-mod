// SPDX-License-Identifier: MIT

use super::LOCALE_REFERENCE_MAX_IDENTITY_BYTES;

/// Text direction the owner registry reports for one locale.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LocaleDirection {
    /// Left-to-right script order.
    LeftToRight,
    /// Right-to-left script order.
    RightToLeft,
    /// The owner registry did not report a direction.
    Unknown,
}

impl LocaleDirection {
    /// Returns the canonical token for this direction.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LeftToRight => "ltr",
            Self::RightToLeft => "rtl",
            Self::Unknown => "unknown",
        }
    }

    /// Parses an owner-registry direction token.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ltr" => Some(Self::LeftToRight),
            "rtl" => Some(Self::RightToLeft),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

/// Plural category used to select one locale-specific plural form.
///
/// The vocabulary follows the documented plural categories an owner registry may declare.  This
/// slice never computes a category from a count: the caller states the category it was given, and
/// an unrecognized category stays [`LocalePluralCategory::Unknown`] instead of being guessed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LocalePluralCategory {
    /// The `zero` category.
    Zero,
    /// The `one` category.
    One,
    /// The `two` category.
    Two,
    /// The `few` category.
    Few,
    /// The `many` category.
    Many,
    /// The `other` category, which is also the documented fallback form.
    Other,
    /// The owner registry did not report a category.
    Unknown,
}

impl LocalePluralCategory {
    /// Number of distinct plural categories.
    pub const COUNT: usize = 7;

    /// Returns the canonical token for this category.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Zero => "zero",
            Self::One => "one",
            Self::Two => "two",
            Self::Few => "few",
            Self::Many => "many",
            Self::Other => "other",
            Self::Unknown => "unknown",
        }
    }

    /// Parses an owner-registry plural category token.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "zero" => Some(Self::Zero),
            "one" => Some(Self::One),
            "two" => Some(Self::Two),
            "few" => Some(Self::Few),
            "many" => Some(Self::Many),
            "other" => Some(Self::Other),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    /// Returns whether this category is the documented plural fallback form.
    #[must_use]
    pub const fn is_fallback_form(self) -> bool {
        matches!(self, Self::Other)
    }
}

/// Owner-declared reason a localized value is not available.
///
/// The reason is carried instead of substituting an empty string, a zero or an invented
/// description for a value the owner could not supply.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LocaleUnavailableReason {
    /// The owner registry has no translation for this reference in this locale.
    NotTranslated,
    /// The requested locale is not in the catalog's supported set.
    UnsupportedLocale,
    /// The owner could not determine availability for this reference.
    Unknown,
}

/// Language-independent identity of one referenced definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocaleEntityReference {
    /// Stable owner-defined entity family.
    pub entity_kind: String,
    /// Opaque ID scoped by `entity_kind`.
    pub namespaced_id: String,
}

/// Typed value substituted for one placeholder name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalePlaceholderValue {
    /// Localized literal text.
    Text(String),
    /// A numeric amount carried exactly as the owner rendered it, never reformatted.
    Number(String),
    /// A stable reference to another definition.
    Reference(LocaleEntityReference),
    /// A value the owner could not supply, with the explicit reason.
    Unavailable(LocaleUnavailableReason),
}

/// One placeholder value supplied when rendering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalePlaceholder {
    /// Placeholder name, matching the name the stored text requires.
    pub name: String,
    /// Typed value for this name.
    pub value: LocalePlaceholderValue,
}

/// One ordered piece of a stored, locale-qualified text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocaleTextSegment {
    /// Literal rendered text for this locale.
    Text(String),
    /// A substitution point naming one required placeholder.
    Placeholder(String),
    /// A typed stable reference to another definition.
    Reference(LocaleEntityReference),
    /// An effect amount carried verbatim so a number is never silently changed.
    Effect {
        /// Owner-defined effect kind.
        kind: String,
        /// Owner-rendered amount, preserved exactly.
        amount: String,
    },
}

/// One ordered piece of a rendered response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocaleRenderedSegment {
    /// Literal rendered text, preserving Unicode and meaning.
    Text(String),
    /// An effect amount carried verbatim.
    Effect {
        /// Owner-defined effect kind.
        kind: String,
        /// Owner-rendered amount, preserved exactly.
        amount: String,
    },
    /// A typed stable reference to another definition.
    Reference(LocaleEntityReference),
    /// A declared placeholder that had no supplied value.
    ///
    /// The name is retained so the gap stays visible rather than collapsing to an empty string.
    UnresolvedPlaceholder(String),
}

/// One source-declared supported locale, with its direction and explicit fallback chain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocaleInput {
    /// Locale tag as the owner registry spells it.
    pub locale: String,
    /// Reported text direction.
    pub direction: LocaleDirection,
    /// Explicit fallback chain, starting with this locale and ending with the default locale.
    pub fallback: Vec<String>,
}

/// One source-owned rendered text for one entity, locale and plural key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocaleEntryInput {
    /// Stable owner-defined entity family.
    pub entity_kind: String,
    /// Opaque ID scoped by `entity_kind`.
    pub namespaced_id: String,
    /// Locale this text is rendered in.
    pub locale: String,
    /// Plural category this text is rendered for.
    pub plural: LocalePluralCategory,
    /// Revision of this text, independent of the semantic content revision.
    pub revision: String,
    /// Ordered rendered segments.
    pub segments: Vec<LocaleTextSegment>,
    /// Placeholder names the segments may reference.
    pub requires: Vec<String>,
}

/// How completely one render satisfied its request.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LocaleCompleteness {
    /// The requested locale supplied the text and every declared placeholder resolved.
    Complete,
    /// Text came from a fallback locale, or at least one placeholder stayed unresolved.
    Partial,
}

/// Validates one opaque identity token used anywhere in this module.
pub(super) fn validate_identity(
    value: &str,
    field: &'static str,
) -> Result<(), super::LocaleCatalogError> {
    if value.is_empty()
        || value.len() > LOCALE_REFERENCE_MAX_IDENTITY_BYTES
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric() && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-')
        })
    {
        return Err(super::LocaleCatalogError::InvalidInput(field));
    }
    Ok(())
}

impl LocaleTextSegment {
    /// Returns the aggregate retained byte length of one segment.
    pub(super) fn retained_len(&self) -> usize {
        match self {
            Self::Text(value) | Self::Placeholder(value) => value.len(),
            Self::Reference(reference) => {
                reference.entity_kind.len() + reference.namespaced_id.len()
            }
            Self::Effect { kind, amount } => kind.len() + amount.len(),
        }
    }
}
