// SPDX-License-Identifier: MIT

/// The audit category assigned to a discovered gameplay-randomness stream.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RngStreamCategory {
    /// Map and act generation.
    MapActGeneration,
    /// Encounter selection and enemy creation.
    EncounterEnemy,
    /// Deck, draw, and shuffle behavior.
    ShuffleDraw,
    /// Target selection and other targeting behavior.
    Targeting,
    /// Reward generation and selection.
    Rewards,
    /// Shop generation and selection.
    Shops,
    /// Event generation and selection.
    Events,
    /// A gameplay-affecting stream that does not fit a known category.
    OtherGameplay,
}

impl RngStreamCategory {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::MapActGeneration => "map_act_generation",
            Self::EncounterEnemy => "encounter_enemy",
            Self::ShuffleDraw => "shuffle_draw",
            Self::Targeting => "targeting",
            Self::Rewards => "rewards",
            Self::Shops => "shops",
            Self::Events => "events",
            Self::OtherGameplay => "other_gameplay",
        }
    }
}

/// Coverage classification for the discovered stream inventory.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RngCoverageStatus {
    /// Every future-affecting stream for this matrix cell was identified.
    Complete,
    /// At least one stream or lifecycle segment remains unclassified.
    Partial,
    /// The producer could not classify coverage.
    Unknown,
}

impl RngCoverageStatus {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Unknown => "unknown",
        }
    }
}

/// Whether a stream's state can be represented without exposing its raw bytes.
#[derive(Clone, Eq, PartialEq)]
pub enum RngCursorEvidence {
    /// A non-zero or otherwise non-canonical state with an opaque digest and cursor.
    Known {
        /// Digest of the private initial state representation.
        state_digest: String,
        /// Initial cursor or draw index.
        cursor: u64,
    },
    /// The producer proved that the initial state is the known zero state.
    KnownZero {
        /// Initial cursor or draw index for the known zero state.
        cursor: u64,
    },
    /// No state or cursor evidence was available.
    Unavailable,
}

/// How a stream obtains its initial seed.
#[derive(Clone, Eq, PartialEq)]
pub enum RngSeedOrigin {
    /// The stream is derived from the canonical run seed.
    MasterDerived {
        /// Host-observed derivation algorithm/version.
        derivation_version: String,
    },
    /// The stream has a separate source that must be audited independently.
    Independent {
        /// Bounded owner description of the independent source.
        source: String,
    },
    /// No derivation or independent source was available.
    Unavailable,
}

impl RngSeedOrigin {
    /// Returns a projection-safe source classification.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::MasterDerived { .. } => "master_derived",
            Self::Independent { .. } => "independent",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Whether a stream can be serialized for a future checkpoint/audit boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RngSerialization {
    /// The owner supplied a bounded serialization representation.
    Available,
    /// The owner proved that serialization is not supported for this stream.
    Unsupported,
    /// Serialization support was not classified.
    Unknown,
}

impl RngSerialization {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unsupported => "unsupported",
            Self::Unknown => "unknown",
        }
    }
}

/// Whether a source can change declared gameplay state.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GameplayImpact {
    /// The source can affect gameplay state or action ordering.
    AffectsGameplay,
    /// Evidence shows the source cannot affect gameplay state.
    CosmeticOnly,
    /// The producer could not classify the effect.
    Unknown,
}

impl GameplayImpact {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::AffectsGameplay => "gameplay",
            Self::CosmeticOnly => "cosmetic_only",
            Self::Unknown => "unknown",
        }
    }
}

/// One discovered future-affecting or cosmetic-only entropy source.
#[derive(Clone, Eq, PartialEq)]
pub struct RngStreamEvidence {
    /// Stable identity assigned by the host owner, not an array index.
    pub stream_id: String,
    /// Audit category for this stream.
    pub category: RngStreamCategory,
    /// Native owner/type identity as observed by the authorized producer.
    pub owner: String,
    /// Algorithm/version where the host exposes one.
    pub algorithm_version: Option<String>,
    /// Seed derivation or independent source classification.
    pub seed_origin: RngSeedOrigin,
    /// Private initial state and cursor evidence.
    pub initial_state: RngCursorEvidence,
    /// Creation lifecycle boundary.
    pub creation_boundary: String,
    /// Reset/reseed lifecycle boundary.
    pub reset_boundary: String,
    /// Bounded call categories observed for the stream.
    pub call_categories: Vec<String>,
    /// Serialization support classification.
    pub serialization: RngSerialization,
    /// Whether this stream affects gameplay.
    pub gameplay: GameplayImpact,
    /// Evidence note for cosmetic-only classification.
    pub evidence: String,
}

/// External entropy and ordering source categories.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ExternalInputKind {
    /// Wall-clock or calendar time.
    WallClock,
    /// Process identity or process-global state.
    ProcessIdentity,
    /// A process/global random source outside the run stream.
    GlobalRandom,
    /// GUIDs, object IDs, or identity counters.
    GuidObjectId,
    /// Hash-map or enumeration order.
    HashEnumerationOrder,
    /// Asynchronous callback completion order.
    AsyncCallback,
    /// Frame or scheduler timing.
    FrameTiming,
    /// Locale or culture-sensitive behavior.
    Locale,
    /// External files or host-provided data.
    ExternalFile,
}

impl ExternalInputKind {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::WallClock => "wall_clock",
            Self::ProcessIdentity => "process_identity",
            Self::GlobalRandom => "global_random",
            Self::GuidObjectId => "guid_object_id",
            Self::HashEnumerationOrder => "hash_enumeration_order",
            Self::AsyncCallback => "async_callback",
            Self::FrameTiming => "frame_timing",
            Self::Locale => "locale",
            Self::ExternalFile => "external_file",
        }
    }
}

/// How one external input is controlled for the audited matrix cell.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ExternalInputControl {
    /// The producer controls this input and binds it to the witness.
    Controlled,
    /// The producer proved that the input is absent for this matrix cell.
    Absent,
    /// The input is present but not controlled.
    Uncontrolled,
    /// The producer could not classify control.
    Unknown,
}

impl ExternalInputControl {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Controlled => "controlled",
            Self::Absent => "absent",
            Self::Uncontrolled => "uncontrolled",
            Self::Unknown => "unknown",
        }
    }
}

/// Whether an entropy inventory is complete or deliberately unavailable.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ExternalInputDeclaration {
    /// The source enumerated every relevant external input.
    Enumerated,
    /// The source proved no relevant external input exists for this cell.
    NoneObserved,
    /// The source did not provide a usable declaration.
    Unknown,
}

impl ExternalInputDeclaration {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Enumerated => "enumerated",
            Self::NoneObserved => "none_observed",
            Self::Unknown => "unknown",
        }
    }
}

/// One bounded external-input inventory entry.
#[derive(Clone, Eq, PartialEq)]
pub struct ExternalInputEvidence {
    /// Stable input category.
    pub kind: ExternalInputKind,
    /// Owner/source identity.
    pub owner: String,
    /// Gameplay relevance classification.
    pub gameplay: GameplayImpact,
    /// Control declaration for this matrix cell.
    pub control: ExternalInputControl,
    /// Evidence note proving the classification.
    pub evidence: String,
}

/// Build, mode, profile, seed, and external-input binding for one witness.
#[derive(Clone, Eq, PartialEq)]
pub struct RngAuditBinding {
    /// Exact host build identity supplied by the authorized producer.
    pub game_build: String,
    /// Mod/adapter compatibility identity.
    pub adapter_compatibility: String,
    /// Supported run mode for this witness.
    pub supported_mode: String,
    /// Profile compatibility identity.
    pub profile_compatibility: String,
    /// Content manifest identity when available.
    pub content_manifest: Option<String>,
    /// Canonical requested/read-back run seed.
    pub canonical_seed: String,
    /// Host-observed master-seed derivation version.
    pub seed_derivation_version: String,
    /// First stable seeded lifecycle boundary.
    pub seeded_boundary: String,
    /// Declaration covering external inputs and ordering.
    pub external_input_declaration: ExternalInputDeclaration,
}
