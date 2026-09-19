// SPDX-License-Identifier: MIT

use super::{
    field::{RunConfigurationField, RunText},
    value::RunValue,
};

/// Exact source producer compatibility for this slice.
pub const RUN_CONFIGURATION_PRODUCER_VERSION: &str = "run-configuration-reference-producer-v1";
/// Manifest entity kind that inventories run configurations.
pub const RUN_CONFIGURATION_ENTITY_KIND: &str = "run_configuration";
/// Maximum records retained in one catalog.
pub const RUN_CONFIGURATION_MAX_RECORDS: usize = 64;
/// Maximum modifiers retained on one record.
pub const RUN_CONFIGURATION_MAX_MODIFIERS: usize = 32;
/// Maximum identities retained in one identity set.
pub const RUN_CONFIGURATION_MAX_IDS: usize = 32;
/// Maximum bytes in one identity.
pub const RUN_CONFIGURATION_MAX_IDENTIFIER_BYTES: usize = 128;
/// Maximum bytes in one localized text value.
pub const RUN_CONFIGURATION_MAX_TEXT_BYTES: usize = 512;
/// Maximum bytes in one retained seed.
pub const RUN_CONFIGURATION_MAX_SEED_BYTES: usize = 64;
/// Maximum items returned in one page.
pub const RUN_CONFIGURATION_MAX_PAGE_ITEMS: usize = 32;
/// Maximum continuations one reader retains.
pub const RUN_CONFIGURATION_MAX_CONTINUATIONS: usize = 64;
/// Maximum estimated bytes retained for one record.
pub const RUN_CONFIGURATION_MAX_RECORD_BYTES: usize = 16_384;
/// Identity namespaces that never belong to an ordinary configuration read.
pub const RUN_CONFIGURATION_FORBIDDEN_IDENTITY_PREFIXES: [&str; 1] = ["rng_state"];

/// Closed inventory of run-affecting configuration fields.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunFieldKind {
    /// Mode the run was admitted in.
    Mode,
    /// Advertised difficulty or ascension level.
    Difficulty,
    /// Character identity.
    Character,
    /// Character loadout or starting deck selection.
    Loadout,
    /// Act sequence the run will traverse.
    ActSequence,
    /// Active run modifiers.
    Modifiers,
    /// Multiplayer scaling applied to the settled configuration.
    MultiplayerScaling,
    /// Active content set and enabled content packages.
    ActiveContent,
    /// Applicable profile unlock or rule settings.
    UnlockRule,
    /// Visible run seed under the seed visibility policy.
    Seed,
}

impl RunFieldKind {
    /// Every field kind in stable declaration order.
    pub const ALL: [Self; 10] = [
        Self::Mode,
        Self::Difficulty,
        Self::Character,
        Self::Loadout,
        Self::ActSequence,
        Self::Modifiers,
        Self::MultiplayerScaling,
        Self::ActiveContent,
        Self::UnlockRule,
        Self::Seed,
    ];

    /// Returns whether the kind must always carry a settled host value.
    ///
    /// The remaining kinds are mode-specific or profile-specific and may report an explicit
    /// `NotApplicable` outcome instead.
    #[must_use]
    pub const fn is_required(self) -> bool {
        matches!(
            self,
            Self::Mode
                | Self::Difficulty
                | Self::Character
                | Self::ActSequence
                | Self::ActiveContent
        )
    }

    /// Returns the stable wire identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mode => "mode",
            Self::Difficulty => "difficulty",
            Self::Character => "character",
            Self::Loadout => "loadout",
            Self::ActSequence => "act_sequence",
            Self::Modifiers => "modifiers",
            Self::MultiplayerScaling => "multiplayer_scaling",
            Self::ActiveContent => "active_content",
            Self::UnlockRule => "unlock_rule",
            Self::Seed => "seed",
        }
    }
}

/// Mode the run was admitted in.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunMode {
    /// Standard run.
    Standard,
    /// Custom run.
    Custom,
    /// Daily run.
    Daily,
    /// Cooperative run.
    Cooperative,
    /// The host could not classify the mode.
    Unknown,
}

/// Advertised difficulty, kept distinct from the mode that advertises it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunDifficulty {
    /// Base difficulty with no ascension level.
    Base,
    /// Advertised ascension level.
    Ascension(u32),
    /// Owner-named custom difficulty.
    Custom(String),
    /// The host could not classify the difficulty.
    Unknown,
}

/// Whether a settled field may still change during the run.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunMutability {
    /// The host fixed the value at run start.
    FixedAtStart,
    /// The host may still change the value during the run.
    Mutable,
    /// The host could not classify the mutability.
    Unknown,
}

/// Where a settled value came from.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunProvenance {
    /// The host settled the value and it is authoritative for the run.
    SettledHost,
    /// The value only echoes what the caller requested.
    RequestedEcho,
    /// The value was copied from an owner-owned record without runtime verification.
    SourceDerived,
    /// Evidence is insufficient to classify the value.
    Unverified,
    /// The host could not classify the value.
    Unknown,
}

/// Whether the field may be discovered publicly.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunSensitivity {
    /// The field may be discovered publicly.
    Public,
    /// The field covers credentials, private endpoints, or operator secrets.
    Private,
}

/// Owner-defined visibility of a field.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunVisibility {
    /// Visible wherever the scope permits.
    Public,
    /// Visible only in the owner scope.
    OwnerOnly,
    /// Never visible, including to the owner.
    Hidden,
    /// The host could not classify the visibility.
    Unknown,
}

/// Caller scope a read is performed under.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunVisibilityScope {
    /// Ordinary public read.
    Public,
    /// Public read with the seed withheld regardless of policy.
    SeedBlind,
    /// Owner read, including owner-only fields.
    Owner,
}

/// Seed visibility policy applied to the run.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunSeedPolicy {
    /// The seed is visible under the ordinary seed policy.
    Visible,
    /// The seed exists but must not be revealed.
    Withheld,
    /// The host could not classify the seed.
    Unknown,
}

/// State of one declared run modifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunModifierState {
    /// The modifier is active for the settled configuration.
    Active,
    /// The modifier is declared but not active.
    Inactive,
    /// The host cannot classify the modifier or its effects.
    Unknown,
}

/// Explicit support state for the run-configuration family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunConfigurationFamilyState {
    /// The source projects the family.
    Handled,
    /// The source advertises the family but cannot project it.
    Unsupported,
    /// The family is temporarily unavailable.
    Unavailable,
}

/// Explicit support state and count for the run-configuration family.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunConfigurationFamilyCoverage {
    /// Owner-defined family identity.
    pub entity_kind: String,
    /// Whether the source projects the family.
    pub state: RunConfigurationFamilyState,
    /// Number of definitions the source declares.
    pub definition_count: usize,
}

/// Typed source-owned run configuration before it is bound to a catalog witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunConfigurationRecordInput {
    /// Run identity settled at admission.
    pub run_id: String,
    /// Live instance identity for the selected run.
    pub instance_id: String,
    /// Monotonic snapshot epoch.
    pub epoch: u64,
    /// Monotonic configuration revision.
    pub revision: u64,
    /// Seed visibility policy applied to the run.
    pub seed_policy: RunSeedPolicy,
    /// Typed requested and settled field records.
    pub fields: Vec<RunFieldRecord>,
    /// Declared run modifiers.
    pub modifiers: Vec<RunModifierInput>,
}

/// One field's requested and settled values with their provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunFieldRecord {
    /// Field kind.
    pub kind: RunFieldKind,
    /// Setup the caller requested at run start.
    pub requested: RunConfigurationField<RunValue>,
    /// Value the host settled.
    pub settled: RunConfigurationField<RunValue>,
    /// Whether the settled value may still change.
    pub mutability: RunMutability,
    /// Where the settled value came from.
    pub provenance: RunProvenance,
    /// Whether the field may be discovered publicly.
    pub sensitivity: RunSensitivity,
    /// Owner-defined visibility.
    pub visibility: RunVisibility,
}

/// One declared run modifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunModifierInput {
    /// Namespaced modifier identity.
    pub modifier_id: String,
    /// Localized modifier label.
    pub label: String,
    /// Declared state.
    pub state: RunModifierState,
    /// Field kinds the modifier changes once active.
    pub alters: Vec<RunFieldKind>,
}

/// Declared run modifier bound to a catalog witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunModifier {
    /// Namespaced modifier identity.
    pub modifier_id: String,
    /// Localized modifier label or an explicit non-value.
    pub label: RunText,
    /// Declared state.
    pub state: RunModifierState,
    /// Field kinds the modifier changes once active.
    pub alters: Vec<RunFieldKind>,
}
