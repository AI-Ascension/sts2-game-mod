// SPDX-License-Identifier: MIT

mod profile;
mod reference;
mod value;

pub use profile::*;
pub use reference::*;
pub use value::*;

/// Source-only producer identity; this is not a wire or native ABI version.
pub const PROGRESSION_REFERENCE_PRODUCER_VERSION: &str = "game-progression-reference-producer-v1";
/// Maximum bytes accepted for one owner-defined identity.
pub const PROGRESSION_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized or owner-defined text value.
pub const PROGRESSION_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aggregate bytes retained for one progression entry.
pub const PROGRESSION_MAX_ENTRY_BYTES: usize = 64 * 1024;
/// Maximum progression entries in one source snapshot.
pub const PROGRESSION_MAX_ENTRIES: usize = 4_096;
/// Maximum requirements attached to one entry.
pub const PROGRESSION_MAX_REQUIREMENTS: usize = 32;
/// Maximum content references attached to one entry.
pub const PROGRESSION_MAX_CONTENT_REFERENCES: usize = 64;
/// Maximum related identities attached to one entry.
pub const PROGRESSION_MAX_RELATED: usize = 16;
/// Maximum declared field rows on one entry.
pub const PROGRESSION_MAX_FIELD_ROWS: usize = 16;
/// Maximum entries returned by one bounded page.
pub const PROGRESSION_MAX_PAGE_ITEMS: usize = 64;
/// Manifest family resolved for unlock/requirement definitions.
pub const PROGRESSION_UNLOCK_KIND: &str = "unlock";
/// Manifest family resolved for achievement definitions.
pub const PROGRESSION_ACHIEVEMENT_KIND: &str = "achievement";
/// Manifest family resolved for compendium/discovery definitions.
pub const PROGRESSION_COMPENDIUM_KIND: &str = "compendium_entry";
/// Manifest family resolved for character definitions.
pub const PROGRESSION_CHARACTER_KIND: &str = "character";
/// Manifest family resolved for best-record definitions.
pub const PROGRESSION_BEST_RECORD_KIND: &str = "best_record";
/// Manifest family resolved for aggregate-statistic definitions.
pub const PROGRESSION_STATISTIC_KIND: &str = "statistic";

/// Owner-defined progression domain.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProgressionDomain {
    /// Unlocks and the requirements that gate them.
    Unlocks,
    /// Achievements, including incremental progress toward one.
    Achievements,
    /// Compendium entries and discoveries.
    Compendium,
    /// Per-character progression.
    CharacterProgression,
    /// Best records, such as a best floor or a fastest run.
    BestRecords,
    /// Aggregate statistics over the profile's history.
    AggregateStatistics,
    /// Account or cloud data that is not game progression and is not projected here.
    AccountScoped,
}

impl ProgressionDomain {
    /// Returns every domain, so coverage can be stated rather than inferred.
    #[must_use]
    pub const fn all() -> [Self; 7] {
        [
            Self::Unlocks,
            Self::Achievements,
            Self::Compendium,
            Self::CharacterProgression,
            Self::BestRecords,
            Self::AggregateStatistics,
            Self::AccountScoped,
        ]
    }

    /// Returns whether this domain is game progression rather than account-scoped data.
    #[must_use]
    pub const fn is_game_progression(self) -> bool {
        !matches!(self, Self::AccountScoped)
    }

    /// Returns the manifest family this domain resolves against, when it has one.
    #[must_use]
    pub const fn manifest_kind(self) -> Option<&'static str> {
        match self {
            Self::Unlocks => Some(PROGRESSION_UNLOCK_KIND),
            Self::Achievements => Some(PROGRESSION_ACHIEVEMENT_KIND),
            Self::Compendium => Some(PROGRESSION_COMPENDIUM_KIND),
            Self::CharacterProgression => Some(PROGRESSION_CHARACTER_KIND),
            Self::BestRecords => Some(PROGRESSION_BEST_RECORD_KIND),
            Self::AggregateStatistics => Some(PROGRESSION_STATISTIC_KIND),
            Self::AccountScoped => None,
        }
    }
}

/// What the source reports about one entry's state for the selected profile.
///
/// The four non-unlocked states are deliberately separate: collapsed into one "not unlocked"
/// value, a caller reports an untracked or unclassified entry as a locked one, and a locked entry
/// as an undiscovered one.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProgressionReadState {
    /// The profile has the entry.
    Unlocked,
    /// The profile does not have the entry and the source states what gates it.
    Locked,
    /// The profile has not encountered the entry; no lock is claimed.
    Undiscovered,
    /// The supported build does not track the entry for this profile.
    NotTracked,
    /// The entry exists but could not be read at this scope.
    Unavailable,
    /// The source could not classify the entry's state.
    Unclassified,
}

impl ProgressionReadState {
    /// Returns whether this state asserts the profile has the entry.
    #[must_use]
    pub const fn is_held(self) -> bool {
        matches!(self, Self::Unlocked)
    }

    /// Returns whether this state is a specifically stated reason rather than a bare absence.
    #[must_use]
    pub const fn is_stated(self) -> bool {
        !matches!(self, Self::Unclassified)
    }
}

/// Owner-defined visibility of one progression entry.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProgressionVisibility {
    /// Visible on the supported public reference surface.
    Visible,
    /// Visible only in an explicitly owner-authorized scope.
    OwnerOnly,
    /// The source knows the entry exists but must not reveal it.
    Hidden,
}

/// Scope requested by a progression query.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProgressionVisibilityScope {
    /// Publicly visible entries only.
    Public,
    /// Public entries plus owner-only entries.
    Reference,
    /// Explicit owner-authorized scope, including hidden entries.
    Owner,
}

impl ProgressionVisibilityScope {
    /// Returns whether this scope may observe one visibility class.
    #[must_use]
    pub const fn observes(self, visibility: ProgressionVisibility) -> bool {
        match visibility {
            ProgressionVisibility::Visible => true,
            ProgressionVisibility::OwnerOnly => !matches!(self, Self::Public),
            ProgressionVisibility::Hidden => matches!(self, Self::Owner),
        }
    }
}

/// Whether one progression entry is game progression or account-scoped data.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProgressionSensitivity {
    /// Game progression the supported reference surface may publish.
    GameProgression,
    /// Account or cloud data, which this boundary refuses to publish as progression.
    AccountScoped,
}

/// Source support state for one progression domain.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProgressionDomainState {
    /// Source projects every entry in this domain.
    Projected,
    /// The domain exists but no typed source adapter is available.
    Unsupported,
    /// The domain is known but currently unavailable.
    Unavailable,
}

/// Declared support state for one progression domain with the count the source reports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressionDomainCoverage {
    /// Domain identity.
    pub domain: ProgressionDomain,
    /// Source support state.
    pub state: ProgressionDomainState,
    /// Number of entries the source declares in this domain.
    pub entry_count: usize,
    /// Fields this domain does not project, stated rather than omitted.
    pub unsupported_fields: Vec<ProgressionField>,
}

/// Validates an owner-defined identity token.
pub(super) fn validate_identity(
    value: &str,
    field: &'static str,
) -> Result<(), super::ProgressionReferenceError> {
    if value.is_empty()
        || value.len() > PROGRESSION_MAX_IDENTITY_BYTES
        || value.chars().any(char::is_control)
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric()
                && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-' | b'#')
        })
    {
        return Err(super::ProgressionReferenceError::InvalidInput(field));
    }
    Ok(())
}

/// Validates a localized or owner-defined text value.
pub(super) fn validate_text(
    value: &str,
    field: &'static str,
) -> Result<(), super::ProgressionReferenceError> {
    if value.is_empty()
        || value.len() > PROGRESSION_MAX_TEXT_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(super::ProgressionReferenceError::InvalidInput(field));
    }
    Ok(())
}
