// SPDX-License-Identifier: MIT

use super::super::CHECKPOINT_CAPTURE_MAX_ID_BYTES;

use super::capability::CheckpointBoundary;

/// Identity binding required for every capture operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckpointCaptureIdentity {
    instance_id: String,
    session_id: String,
    lease_id: String,
    lease_epoch: u64,
    run_id: String,
    profile_id: String,
    operation_id: String,
}

impl CheckpointCaptureIdentity {
    /// Creates an identity after enforcing bounded non-empty identifiers.
    pub fn new(
        instance_id: impl Into<String>,
        session_id: impl Into<String>,
        lease_id: impl Into<String>,
        lease_epoch: u64,
        run_id: impl Into<String>,
        profile_id: impl Into<String>,
        operation_id: impl Into<String>,
    ) -> Result<Self, CheckpointIdentityError> {
        let identity = Self {
            instance_id: instance_id.into(),
            session_id: session_id.into(),
            lease_id: lease_id.into(),
            lease_epoch,
            run_id: run_id.into(),
            profile_id: profile_id.into(),
            operation_id: operation_id.into(),
        };
        for (name, value) in [
            ("instance_id", identity.instance_id.as_str()),
            ("session_id", identity.session_id.as_str()),
            ("lease_id", identity.lease_id.as_str()),
            ("run_id", identity.run_id.as_str()),
            ("profile_id", identity.profile_id.as_str()),
            ("operation_id", identity.operation_id.as_str()),
        ] {
            validate_identity_field(name, value)?;
        }
        Ok(identity)
    }

    /// Returns the live instance identity.
    #[must_use]
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Returns the live session identity.
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Returns the lease identity.
    #[must_use]
    pub fn lease_id(&self) -> &str {
        &self.lease_id
    }

    /// Returns the lease epoch.
    #[must_use]
    pub const fn lease_epoch(&self) -> u64 {
        self.lease_epoch
    }

    /// Returns the active run identity.
    #[must_use]
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// Returns the profile identity.
    #[must_use]
    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    /// Returns the logical operation identity used for duplicate admission.
    #[must_use]
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }
}

/// Identity validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointIdentityError {
    /// An identity component was empty.
    Empty {
        /// Component name.
        field: &'static str,
    },
    /// An identity component exceeded [`CHECKPOINT_CAPTURE_MAX_ID_BYTES`].
    TooLong {
        /// Component name.
        field: &'static str,
    },
}

impl std::fmt::Display for CheckpointIdentityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty { field } => write!(formatter, "{field} must not be empty"),
            Self::TooLong { field } => write!(
                formatter,
                "{field} exceeds {} bytes",
                CHECKPOINT_CAPTURE_MAX_ID_BYTES
            ),
        }
    }
}

impl std::error::Error for CheckpointIdentityError {}

fn validate_identity_field(
    field: &'static str,
    value: &str,
) -> Result<(), CheckpointIdentityError> {
    if value.is_empty() {
        return Err(CheckpointIdentityError::Empty { field });
    }
    if value.len() > CHECKPOINT_CAPTURE_MAX_ID_BYTES {
        return Err(CheckpointIdentityError::TooLong { field });
    }
    Ok(())
}

/// A capture request admitted against one live identity and boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckpointCaptureRequest {
    identity: CheckpointCaptureIdentity,
    boundary: CheckpointBoundary,
}

impl CheckpointCaptureRequest {
    /// Creates a request whose identity cannot be changed after admission.
    #[must_use]
    pub const fn new(identity: CheckpointCaptureIdentity, boundary: CheckpointBoundary) -> Self {
        Self { identity, boundary }
    }

    /// Returns the identity fence for this operation.
    #[must_use]
    pub const fn identity(&self) -> &CheckpointCaptureIdentity {
        &self.identity
    }

    /// Returns the requested decision boundary.
    #[must_use]
    pub const fn boundary(&self) -> CheckpointBoundary {
        self.boundary
    }
}
