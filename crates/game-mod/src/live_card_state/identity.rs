// SPDX-License-Identifier: MIT

use super::LIVE_CARD_MAX_ID_BYTES;

/// Identity components that fence a live card read.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LiveCardReadReference {
    /// Live game instance identity.
    pub instance_id: String,
    /// Active run identity.
    pub run_id: String,
    /// Immutable content-manifest identity used by the card definition reference.
    pub content_manifest: String,
    /// Coherent owner-local snapshot identity.
    pub snapshot: String,
    /// Monotonic owner-local epoch for this snapshot.
    pub epoch: u64,
}

impl LiveCardReadReference {
    /// Creates a bounded, non-empty read fence.
    pub fn new(
        instance_id: impl Into<String>,
        run_id: impl Into<String>,
        content_manifest: impl Into<String>,
        snapshot: impl Into<String>,
        epoch: u64,
    ) -> Result<Self, LiveCardIdentityError> {
        let reference = Self {
            instance_id: instance_id.into(),
            run_id: run_id.into(),
            content_manifest: content_manifest.into(),
            snapshot: snapshot.into(),
            epoch,
        };
        for (field, value) in [
            ("instance_id", reference.instance_id.as_str()),
            ("run_id", reference.run_id.as_str()),
            ("content_manifest", reference.content_manifest.as_str()),
            ("snapshot", reference.snapshot.as_str()),
        ] {
            validate_identity(field, value)?;
        }
        Ok(reference)
    }
}

/// Failure while constructing a live card identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiveCardIdentityError {
    /// An identity component was empty.
    Empty {
        /// Component name.
        field: &'static str,
    },
    /// An identity component exceeded the local bound.
    TooLong {
        /// Component name.
        field: &'static str,
    },
    /// An identity component contained a control or delimiter that is not safe for a local ID.
    InvalidCharacters {
        /// Component name.
        field: &'static str,
    },
}

impl std::fmt::Display for LiveCardIdentityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty { field } => write!(formatter, "{field} must not be empty"),
            Self::TooLong { field } => {
                write!(formatter, "{field} exceeds {LIVE_CARD_MAX_ID_BYTES} bytes")
            }
            Self::InvalidCharacters { field } => {
                write!(formatter, "{field} contains invalid characters")
            }
        }
    }
}

impl std::error::Error for LiveCardIdentityError {}

/// Stable content definition identity shared by card instances and static indexes.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CardDefinitionReference {
    /// Content manifest that owns the definition.
    pub manifest: String,
    /// Namespaced definition identity.
    pub definition_id: String,
}

impl CardDefinitionReference {
    /// Creates a bounded definition reference.
    pub fn new(
        manifest: impl Into<String>,
        definition_id: impl Into<String>,
    ) -> Result<Self, LiveCardIdentityError> {
        let reference = Self {
            manifest: manifest.into(),
            definition_id: definition_id.into(),
        };
        validate_identity("manifest", &reference.manifest)?;
        validate_identity("definition_id", &reference.definition_id)?;
        Ok(reference)
    }
}

/// Stable instance identity used by pages, selectors, and detail reads.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CardInstanceReference {
    /// The coherent read fence that created this reference.
    pub read: LiveCardReadReference,
    /// Distinct live instance identity, even when definitions are equal.
    pub instance_id: String,
}

impl CardInstanceReference {
    /// Creates an instance reference bound to one read fence.
    pub fn new(
        read: LiveCardReadReference,
        instance_id: impl Into<String>,
    ) -> Result<Self, LiveCardIdentityError> {
        let instance_id = instance_id.into();
        validate_identity("card_instance_id", &instance_id)?;
        Ok(Self { read, instance_id })
    }
}

pub(super) fn validate_identity(
    field: &'static str,
    value: &str,
) -> Result<(), LiveCardIdentityError> {
    if value.is_empty() {
        return Err(LiveCardIdentityError::Empty { field });
    }
    if value.len() > LIVE_CARD_MAX_ID_BYTES {
        return Err(LiveCardIdentityError::TooLong { field });
    }
    if value.chars().any(|character| {
        character.is_control()
            || character.is_whitespace()
            || matches!(character, '"' | '\\' | '{' | '}' | '[' | ']')
    }) {
        return Err(LiveCardIdentityError::InvalidCharacters { field });
    }
    Ok(())
}
