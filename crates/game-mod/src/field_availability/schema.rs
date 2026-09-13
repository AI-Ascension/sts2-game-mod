// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use super::{LocalFieldOrigin, LocalFieldResult, LocalFieldStatus, LocalFieldValue};

/// Value kind declared by an owner-local field allowlist.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LocalFieldValueKind {
    Integer,
    Boolean,
    Text,
    TextList,
}

/// One allowlisted field definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalFieldDefinition {
    /// Stable owner-local field name.
    pub name: String,
    /// Expected local value kind.
    pub value_kind: LocalFieldValueKind,
    /// Whether this field is required for the entity kind.
    pub required: bool,
    /// Whether it is part of the bounded basic surface.
    pub basic: bool,
    /// Whether a host extractor exists for this build/kind.
    pub supported: bool,
    /// Whether caller scope must deny the field.
    pub protected: bool,
    /// Allowlisted detail group, when basic reads may recover it.
    pub detail_group: Option<String>,
}

impl LocalFieldDefinition {
    /// Creates a supported, public basic field.
    #[must_use]
    pub fn new(name: impl Into<String>, value_kind: LocalFieldValueKind) -> Self {
        Self {
            name: name.into(),
            value_kind,
            required: false,
            basic: true,
            supported: true,
            protected: false,
            detail_group: None,
        }
    }
}

/// Allowlist and field-group declarations for one owner-defined kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalKindSchema {
    /// Owner-defined kind name.
    pub kind: String,
    /// Field definitions keyed by their exact local name.
    pub fields: BTreeMap<String, LocalFieldDefinition>,
    /// Detail groups keyed by exact local group name.
    pub groups: BTreeMap<String, BTreeSet<String>>,
}

impl LocalKindSchema {
    /// Starts an empty kind schema.
    #[must_use]
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            fields: BTreeMap::new(),
            groups: BTreeMap::new(),
        }
    }

    /// Adds or replaces one local field declaration.
    #[must_use]
    pub fn with_field(mut self, definition: LocalFieldDefinition) -> Self {
        self.fields.insert(definition.name.clone(), definition);
        self
    }

    /// Adds an allowlisted detail group.
    #[must_use]
    pub fn with_group(
        mut self,
        group: impl Into<String>,
        fields: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.groups
            .insert(group.into(), fields.into_iter().map(Into::into).collect());
        self
    }
}

/// Synthetic source state used only for deterministic tests and local contract checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalFixtureValue {
    /// A concrete observed value.
    Value(LocalFieldValue),
    /// Field does not apply in this fixture phase.
    NotApplicable,
    /// Field is supported but not observable in the basic surface.
    NotObserved,
    /// Source cannot classify the value.
    Unknown,
    /// Source encountered a bounded extraction failure.
    Failed,
}

/// One synthetic entity; no host objects or reflection paths are representable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalEntityFixture {
    /// Opaque entity identity.
    pub entity_id: String,
    /// Synthetic field outcomes.
    pub values: BTreeMap<String, LocalFixtureValue>,
    /// Declared encoded-size estimate for each allowlisted detail group.
    pub detail_bytes: BTreeMap<String, usize>,
}

/// Field support summary for one kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalFieldSupport {
    /// Field name.
    pub field: String,
    /// Declared value kind.
    pub value_kind: LocalFieldValueKind,
    /// Required-field marker.
    pub required: bool,
    /// Basic-surface marker.
    pub basic: bool,
    /// Extractor support marker.
    pub supported: bool,
    /// Caller-scope protection marker.
    pub protected: bool,
    /// Detail group, if recovery is allowlisted.
    pub detail_group: Option<String>,
}

/// Coverage evidence for one required field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalRequiredFieldCoverage {
    /// Required field name.
    pub field: String,
    /// Number of entities in the source.
    pub entity_count: usize,
    /// Number of entities with an available value on the basic surface.
    pub available_count: usize,
    /// Aggregate status without coercing unavailable values to defaults.
    pub status: LocalFieldStatus,
}

/// Per-kind support and required-field coverage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalSupportReport {
    /// Owner-defined kind.
    pub entity_kind: String,
    /// Whether collection total is known.
    pub count_known: bool,
    /// Number of entities currently in the source.
    pub entity_count: usize,
    /// Allowlisted fields.
    pub fields: Vec<LocalFieldSupport>,
    /// Required-field coverage evidence.
    pub required_fields: Vec<LocalRequiredFieldCoverage>,
}

/// Bounded detail response after identity and allowlist checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalDetailResponse {
    /// Owner-defined entity kind.
    pub entity_kind: String,
    /// Opaque entity identity.
    pub entity_id: String,
    /// Allowlisted field group.
    pub field_group: String,
    /// Returned fields.
    pub fields: BTreeMap<String, LocalFieldResult>,
    /// Conservative synthetic encoded-size estimate validated against field values.
    pub estimated_bytes: usize,
    /// Identity shared by all fields.
    pub origin: LocalFieldOrigin,
}

/// Sanitized failures from the owner-local bounded read engine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalReadError {
    /// No allowlisted kind matched the request.
    UnknownKind,
    /// Field was not in the allowlist.
    UnknownField,
    /// Explicit request named a protected field.
    DeniedField,
    /// Detail group was not allowlisted for the kind.
    UnknownFieldGroup,
    /// Entity identity was not present in the bound snapshot.
    EntityNotFound,
    /// Request page size violated the configured bound.
    InvalidPageSize,
    /// Cursor was malformed, consumed, or bound to another read.
    InvalidContinuation,
    /// Cursor or detail identity no longer matches the source.
    StaleReference,
    /// Detail payload exceeded the configured byte bound.
    DetailTooLarge { limit: usize, actual: usize },
    /// Detail payload has no bounded size estimate.
    DetailSizeUnavailable,
    /// Fixture schema violated the local allowlist invariants.
    InvalidSchema,
}

impl std::fmt::Display for LocalReadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownKind => formatter.write_str("unknown entity kind"),
            Self::UnknownField => formatter.write_str("unknown field"),
            Self::DeniedField => formatter.write_str("field access denied"),
            Self::UnknownFieldGroup => formatter.write_str("unknown field group"),
            Self::EntityNotFound => formatter.write_str("entity not found"),
            Self::InvalidPageSize => formatter.write_str("invalid page size"),
            Self::InvalidContinuation => formatter.write_str("invalid continuation"),
            Self::StaleReference => formatter.write_str("stale read reference"),
            Self::DetailTooLarge { .. } => formatter.write_str("detail exceeds byte bound"),
            Self::DetailSizeUnavailable => formatter.write_str("detail size unavailable"),
            Self::InvalidSchema => formatter.write_str("invalid local field schema"),
        }
    }
}

impl std::error::Error for LocalReadError {}
