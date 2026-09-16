// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

use super::api::ExactRestoreError;

pub(super) const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// Full current-owner fence from the exact-restore protocol.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExactRestoreOwnerFence {
    pub(super) deployment_id: String,
    pub(super) instance_id: String,
    pub(super) instance_incarnation: String,
    pub(super) boot_id: String,
    pub(super) authority_generation: u64,
    pub(super) host_fence_id: String,
    pub(super) host_fence_generation: u64,
    pub(super) lease_id: String,
    pub(super) lease_epoch: u64,
    pub(super) session_id: String,
    pub(super) lease_expires_at_millis: u64,
}

impl ExactRestoreOwnerFence {
    /// Creates a complete current-owner fence after validating protocol bounds.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        deployment_id: String,
        instance_id: String,
        instance_incarnation: String,
        boot_id: String,
        authority_generation: u64,
        host_fence_id: String,
        host_fence_generation: u64,
        lease_id: String,
        lease_epoch: u64,
        session_id: String,
        lease_expires_at_millis: u64,
    ) -> Result<Self, ExactRestoreError> {
        let value = Self {
            deployment_id,
            instance_id,
            instance_incarnation,
            boot_id,
            authority_generation,
            host_fence_id,
            host_fence_generation,
            lease_id,
            lease_epoch,
            session_id,
            lease_expires_at_millis,
        };
        value.validate()?;
        Ok(value)
    }

    /// Returns the deployment UUID.
    pub fn deployment_id(&self) -> &str {
        &self.deployment_id
    }

    /// Returns the instance UUID.
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Returns the instance incarnation UUID.
    pub fn instance_incarnation(&self) -> &str {
        &self.instance_incarnation
    }

    /// Returns the host boot UUID.
    pub fn boot_id(&self) -> &str {
        &self.boot_id
    }

    /// Returns the authority generation.
    pub const fn authority_generation(&self) -> u64 {
        self.authority_generation
    }

    /// Returns the local host-fence UUID.
    pub fn host_fence_id(&self) -> &str {
        &self.host_fence_id
    }

    /// Returns the local host-fence generation.
    pub const fn host_fence_generation(&self) -> u64 {
        self.host_fence_generation
    }

    /// Returns the lease UUID.
    pub fn lease_id(&self) -> &str {
        &self.lease_id
    }

    /// Returns the current lease epoch.
    pub const fn lease_epoch(&self) -> u64 {
        self.lease_epoch
    }

    /// Returns the current session identity.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Returns the lease expiry observed by the owner.
    pub const fn lease_expires_at_millis(&self) -> u64 {
        self.lease_expires_at_millis
    }

    pub(super) fn validate(&self) -> Result<(), ExactRestoreError> {
        for value in [
            self.deployment_id.as_str(),
            self.instance_id.as_str(),
            self.instance_incarnation.as_str(),
            self.boot_id.as_str(),
            self.host_fence_id.as_str(),
            self.lease_id.as_str(),
        ] {
            if !valid_uuid(value, false) {
                return Err(ExactRestoreError::OwnerUnavailable);
            }
        }
        if self.session_id.is_empty()
            || self.session_id.len() > 512
            || self.authority_generation == 0
            || self.host_fence_generation == 0
            || self.lease_epoch == 0
            || self.lease_expires_at_millis == 0
            || [
                self.authority_generation,
                self.host_fence_generation,
                self.lease_epoch,
                self.lease_expires_at_millis,
            ]
            .into_iter()
            .any(|value| value > MAX_SAFE_INTEGER)
        {
            return Err(ExactRestoreError::OwnerUnavailable);
        }
        Ok(())
    }

    pub(super) fn matches_transport(&self, authorization: &ExactRestoreAuthorization) -> bool {
        self.instance_id == authorization.instance_id
            && self.session_id == authorization.session_id
            && self.lease_id == authorization.lease_id
            && self.lease_epoch == authorization.lease_epoch
    }
}

/// Current owner observation with its independently observed clock.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactRestoreCurrentOwner {
    /// Owner fence read by an authoritative owner adapter.
    pub fence: ExactRestoreOwnerFence,
    /// Unix time in milliseconds observed by that same adapter.
    pub observed_at_millis: u64,
}

/// Authenticated outer transport context forwarded by the fixed native listener.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactRestoreAuthorization {
    /// Trusted caller principal forwarded by the authenticated gateway.
    pub principal: String,
    /// Correlation identity copied from the HTTP envelope.
    pub correlation_id: String,
    /// Current instance identity copied from the HTTP envelope.
    pub instance_id: String,
    /// Current session identity copied from the HTTP envelope.
    pub session_id: String,
    /// Current lease identity copied from the HTTP envelope.
    pub lease_id: String,
    /// Current lease epoch copied from the HTTP envelope.
    pub lease_epoch: u64,
}

impl ExactRestoreAuthorization {
    /// Creates a bounded transport context.
    pub fn new(
        principal: String,
        correlation_id: String,
        instance_id: String,
        session_id: String,
        lease_id: String,
        lease_epoch: u64,
    ) -> Result<Self, ExactRestoreError> {
        let context = Self {
            principal,
            correlation_id,
            instance_id,
            session_id,
            lease_id,
            lease_epoch,
        };
        if [
            context.principal.as_str(),
            context.correlation_id.as_str(),
            context.instance_id.as_str(),
            context.session_id.as_str(),
            context.lease_id.as_str(),
        ]
        .into_iter()
        .any(|value| value.is_empty() || value.len() > 512)
            || !valid_uuid(&context.instance_id, false)
            || context.lease_epoch == 0
            || context.lease_epoch > MAX_SAFE_INTEGER
        {
            return Err(ExactRestoreError::Unauthorized);
        }
        Ok(context)
    }
}

/// Authoritative owner fence provider used on every phase.
pub trait RestoreOwnerProvider {
    /// Reads the full current owner and its live clock.
    fn current_owner(&mut self) -> Result<ExactRestoreCurrentOwner, ExactRestoreError>;
}

/// Production owner provider that refuses because no authoritative local provider is installed.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExactRestoreUnavailableOwner;

impl RestoreOwnerProvider for ExactRestoreUnavailableOwner {
    fn current_owner(&mut self) -> Result<ExactRestoreCurrentOwner, ExactRestoreError> {
        Err(ExactRestoreError::OwnerUnavailable)
    }
}

pub(super) fn valid_uuid(value: &str, version_four: bool) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => *byte == b'-',
            14 if version_four => *byte == b'4',
            19 if version_four => matches!(byte, b'8' | b'9' | b'a' | b'b'),
            _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(byte),
        })
}
