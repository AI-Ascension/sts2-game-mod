// SPDX-License-Identifier: MIT

//! Typed progression values whose availability is stated, so an untracked value is never a zero.

use super::super::ProgressionReferenceError;
use super::{PROGRESSION_MAX_TEXT_BYTES, validate_identity, validate_text};

/// Availability of one progression field.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProgressionFieldStatus {
    /// A value was reported.
    Available,
    /// The field does not apply to this entry, such as progress on a non-incremental achievement.
    NotApplicable,
    /// The source did not report the field for this read.
    NotObserved,
    /// The supported build does not carry the field.
    Unsupported,
    /// The field exists but is withheld at this scope.
    Withheld,
}

impl ProgressionFieldStatus {
    /// Returns whether a value may be published for this status.
    #[must_use]
    pub const fn is_available(self) -> bool {
        matches!(self, Self::Available)
    }

    /// Returns whether the status names a reason the field carries no value.
    #[must_use]
    pub const fn is_stated(self) -> bool {
        !self.is_available()
    }
}

/// A value whose availability is explicit.
///
/// The status and the value are one field, not two, because a caller that can read the value
/// without reading its status will eventually read a missing value as a default.  The constructors
/// are the only way to build one, so `Available` always carries a value and every other status
/// carries none.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressionFieldValue<T> {
    status: ProgressionFieldStatus,
    value: Option<T>,
}

impl<T> ProgressionFieldValue<T> {
    /// States that a value was reported.
    #[must_use]
    pub fn available(value: T) -> Self {
        Self {
            status: ProgressionFieldStatus::Available,
            value: Some(value),
        }
    }

    /// States that the field does not apply to this entry.
    #[must_use]
    pub const fn not_applicable() -> Self {
        Self {
            status: ProgressionFieldStatus::NotApplicable,
            value: None,
        }
    }

    /// States that the source did not report the field for this read.
    #[must_use]
    pub const fn not_observed() -> Self {
        Self {
            status: ProgressionFieldStatus::NotObserved,
            value: None,
        }
    }

    /// States that the supported build does not carry the field.
    #[must_use]
    pub const fn unsupported() -> Self {
        Self {
            status: ProgressionFieldStatus::Unsupported,
            value: None,
        }
    }

    /// States that the field is withheld at this scope.
    #[must_use]
    pub const fn withheld() -> Self {
        Self {
            status: ProgressionFieldStatus::Withheld,
            value: None,
        }
    }

    /// Returns the availability status.
    #[must_use]
    pub const fn status(&self) -> ProgressionFieldStatus {
        self.status
    }

    /// Returns whether a value was reported.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.status.is_available()
    }

    /// Returns the reported value, or `None` when the status states no value.
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Consumes the field and returns the reported value, or `None`.
    #[must_use]
    pub fn into_value(self) -> Option<T> {
        self.value
    }

    /// Returns whether the status and the value agree.
    #[must_use]
    pub const fn is_consistent(&self) -> bool {
        self.status.is_available() == self.value.is_some()
    }
}

/// Exact unit of one reported progression quantity.
///
/// Every unit is stated with the value, because a bare number whose unit is left to the caller
/// cannot be compared with the host's own value.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProgressionUnit {
    /// A count of discrete things, such as floors reached or cards discovered.
    Count,
    /// A whole-number percentage of a stated whole.
    Percent,
    /// Whole seconds of wall-clock or in-game time.
    Seconds,
    /// Whole milliseconds of wall-clock or in-game time.
    Milliseconds,
    /// The game's own score unit.
    Score,
    /// An owner-defined unit, named rather than assumed.
    OwnerDefined {
        /// Owner-defined unit identity.
        unit: String,
    },
}

impl ProgressionUnit {
    /// Validates an owner-defined unit identity.
    pub fn owner_defined(unit: &str) -> Result<Self, ProgressionReferenceError> {
        validate_identity(unit, "progression_unit")?;
        Ok(Self::OwnerDefined {
            unit: unit.to_owned(),
        })
    }
}

/// A reported quantity with its exact unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressionQuantity {
    /// Reported value in the stated unit.
    pub value: i64,
    /// Exact unit of the value.
    pub unit: ProgressionUnit,
}

impl ProgressionQuantity {
    /// Creates one quantity with an exact unit.
    #[must_use]
    pub const fn new(value: i64, unit: ProgressionUnit) -> Self {
        Self { value, unit }
    }

    /// Returns whether the quantity is a percentage inside its own closed range.
    ///
    /// A percentage outside `0..=100` is not clamped, because clamping would report a value the
    /// host did not send.
    #[must_use]
    pub const fn is_bounded_percentage(&self) -> bool {
        match self.unit {
            ProgressionUnit::Percent => self.value >= 0 && self.value <= 100,
            _ => true,
        }
    }
}

/// A localized text value whose absence is stated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressionText {
    status: ProgressionFieldStatus,
    text: Option<String>,
}

impl ProgressionText {
    /// Validates and states an available localized value.
    pub fn available(value: &str) -> Result<Self, ProgressionReferenceError> {
        validate_text(value, "progression_text")?;
        if value.len() > PROGRESSION_MAX_TEXT_BYTES {
            return Err(ProgressionReferenceError::InvalidInput("progression_text"));
        }
        Ok(Self {
            status: ProgressionFieldStatus::Available,
            text: Some(value.to_owned()),
        })
    }

    /// States that the source did not report the text for this read.
    #[must_use]
    pub const fn not_observed() -> Self {
        Self {
            status: ProgressionFieldStatus::NotObserved,
            text: None,
        }
    }

    /// States that the supported build does not carry the text.
    #[must_use]
    pub const fn unsupported() -> Self {
        Self {
            status: ProgressionFieldStatus::Unsupported,
            text: None,
        }
    }

    /// Returns the availability status.
    #[must_use]
    pub const fn status(&self) -> ProgressionFieldStatus {
        self.status
    }

    /// Returns the localized value, or `None` when the status states none.
    #[must_use]
    pub fn value(&self) -> Option<&str> {
        self.text.as_deref()
    }
}

/// Creates an available text value the caller has already validated.
pub(crate) fn text_from_validated(value: String) -> ProgressionText {
    ProgressionText {
        status: ProgressionFieldStatus::Available,
        text: Some(value),
    }
}

/// Field of one progression entry.
///
/// Every entry states a row for each variant, so a field the source does not project is a declared
/// coverage failure rather than a silently missing key.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProgressionField {
    /// Localized entry title.
    Title,
    /// Localized entry description.
    Description,
    /// What the source reports about the entry's state for this profile.
    ReadState,
    /// Progress toward the entry.
    Progress,
    /// Best recorded value for the entry.
    BestValue,
    /// Requirements gating the entry.
    Requirements,
    /// Manifest definitions the entry resolves against.
    ContentReferences,
    /// Related owner identities carried beside the entry.
    RelatedEntries,
}

impl ProgressionField {
    /// Returns every field an entry must state a row for.
    #[must_use]
    pub const fn all() -> [Self; 8] {
        [
            Self::Title,
            Self::Description,
            Self::ReadState,
            Self::Progress,
            Self::BestValue,
            Self::Requirements,
            Self::ContentReferences,
            Self::RelatedEntries,
        ]
    }

    /// Returns the stable field name used in diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Description => "description",
            Self::ReadState => "read_state",
            Self::Progress => "progress",
            Self::BestValue => "best_value",
            Self::Requirements => "requirements",
            Self::ContentReferences => "content_references",
            Self::RelatedEntries => "related_entries",
        }
    }
}

/// Declared availability of one entry field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProgressionFieldAvailability {
    /// Field this row states.
    pub field: ProgressionField,
    /// Availability of that field.
    pub status: ProgressionFieldStatus,
}
