// SPDX-License-Identifier: MIT

use super::{SAVE_PROFILE_MAX_ID_BYTES, SAVE_PROFILE_MAX_SLOTS};

/// Validation failure for an owner-local opaque identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileIdentityError {
    /// The identity has no value.
    Empty { kind: &'static str },
    /// The identity exceeds the bounded owner-local limit.
    TooLong { kind: &'static str },
    /// The value resembles a path or contains an unsafe control character.
    PathLike { kind: &'static str },
    /// The value contains a character outside the bounded token alphabet.
    InvalidCharacter { kind: &'static str },
}

impl std::fmt::Display for ProfileIdentityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty { kind } => write!(formatter, "{kind} must not be empty"),
            Self::TooLong { kind } => write!(
                formatter,
                "{kind} exceeds {SAVE_PROFILE_MAX_ID_BYTES} bytes"
            ),
            Self::PathLike { kind } => write!(formatter, "{kind} must not be path-like"),
            Self::InvalidCharacter { kind } => {
                write!(formatter, "{kind} contains an invalid character")
            }
        }
    }
}

impl std::error::Error for ProfileIdentityError {}

fn validate_identity(value: String, kind: &'static str) -> Result<String, ProfileIdentityError> {
    if value.is_empty() {
        return Err(ProfileIdentityError::Empty { kind });
    }
    if value.len() > SAVE_PROFILE_MAX_ID_BYTES {
        return Err(ProfileIdentityError::TooLong { kind });
    }
    if value == "." || value == ".." || value.starts_with('~') {
        return Err(ProfileIdentityError::PathLike { kind });
    }
    if value
        .bytes()
        .any(|byte| matches!(byte, b'/' | b'\\' | 0..=31 | 127))
    {
        return Err(ProfileIdentityError::PathLike { kind });
    }
    if value.len() >= 2 && value.as_bytes()[1] == b':' && value.as_bytes()[0].is_ascii_alphabetic()
    {
        return Err(ProfileIdentityError::PathLike { kind });
    }
    if value
        .bytes()
        .any(|byte| !byte.is_ascii_alphanumeric() && !matches!(byte, b'.' | b'_' | b'-' | b':'))
    {
        return Err(ProfileIdentityError::InvalidCharacter { kind });
    }
    Ok(value)
}

macro_rules! opaque_identity {
    ($name:ident, $kind:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Validates and owns one bounded opaque token.
            pub fn new(value: impl Into<String>) -> Result<Self, ProfileIdentityError> {
                Ok(Self(validate_identity(value.into(), $kind)?))
            }

            /// Returns the owner-local token without assigning transport meaning.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

opaque_identity!(
    SaveSlotId,
    "save_slot_id",
    "Opaque host-owned save-slot identity."
);
opaque_identity!(
    InstanceIdentity,
    "instance_id",
    "Opaque selected game-instance identity."
);
opaque_identity!(
    UserDataIdentity,
    "user_data_id",
    "Opaque gateway-selected user-data identity."
);
opaque_identity!(
    WorkflowProfileId,
    "workflow_profile_id",
    "Opaque workflow profile identity, intentionally distinct from a save slot."
);
opaque_identity!(
    ProviderProfileId,
    "provider_profile_id",
    "Opaque provider/model profile identity, intentionally distinct from a save slot."
);
opaque_identity!(
    HostCompatibility,
    "host_compatibility",
    "Bounded owner-defined host compatibility identity."
);
opaque_identity!(
    SelectionAuthority,
    "selection_authority",
    "Opaque explicit authority witness for a selection request."
);
opaque_identity!(
    SelectionIdempotencyKey,
    "selection_idempotency_key",
    "Opaque durable key used to reconcile one selection operation."
);

/// Read-only freshness witness for one already-isolated user-data identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaveProfileBaseline {
    user_data_id: UserDataIdentity,
    digest: String,
    revision: u64,
}

impl SaveProfileBaseline {
    /// Creates a bounded baseline without reading or writing a filesystem.
    pub fn new(
        user_data_id: UserDataIdentity,
        digest: impl Into<String>,
        revision: u64,
    ) -> Result<Self, ProfileIdentityError> {
        Ok(Self {
            user_data_id,
            digest: validate_identity(digest.into(), "baseline_digest")?,
            revision,
        })
    }

    /// Returns the user-data identity fenced by this baseline.
    #[must_use]
    pub const fn user_data_id(&self) -> &UserDataIdentity {
        &self.user_data_id
    }

    /// Returns the bounded digest/reference value.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// Returns the owner-local freshness revision.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }
}

/// Identity and freshness fence supplied to discovery and selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BaselineFence {
    instance_id: InstanceIdentity,
    baseline: SaveProfileBaseline,
}

impl BaselineFence {
    /// Creates a fence for one selected instance and isolated user-data baseline.
    pub fn new(
        instance_id: InstanceIdentity,
        baseline: SaveProfileBaseline,
    ) -> Result<Self, ProfileIdentityError> {
        if baseline.user_data_id().as_str().is_empty() {
            return Err(ProfileIdentityError::Empty {
                kind: "baseline_user_data_id",
            });
        }
        Ok(Self {
            instance_id,
            baseline,
        })
    }

    /// Returns the selected instance identity.
    #[must_use]
    pub const fn instance_id(&self) -> &InstanceIdentity {
        &self.instance_id
    }

    /// Returns the user-data baseline.
    #[must_use]
    pub const fn baseline(&self) -> &SaveProfileBaseline {
        &self.baseline
    }
}

/// Ensures fixture callers cannot silently expand the bounded slot inventory.
pub(super) const fn slot_limit_is_valid(count: usize) -> bool {
    count <= SAVE_PROFILE_MAX_SLOTS
}
