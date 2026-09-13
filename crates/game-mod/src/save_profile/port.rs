// SPDX-License-Identifier: MIT

use super::{
    ProfileDiscovery, ProfileDiscoveryError, ProfileDiscoveryRequest, ProfileSelectionReceipt,
    ProfileSelectionRejection, ProfileSelectionRequest, SelectionIdempotencyKey,
};

/// Synthetic-only availability marker; no real host capability is implied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileSelectionAvailability {
    /// The operation is implemented only by the in-memory fixture.
    SyntheticFixtureOnly,
    /// Host-thread selection is not currently available.
    UnavailableHost,
}

/// Disposable directory creation remains outside game-mod ownership.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisposableProvisioningAvailability {
    /// Gateway-owned provisioning is not implemented in this target.
    UnavailableOutsideOwner,
}

/// Owner-local, read-only discovery source.
pub trait ProfileReadPort {
    /// Copies bounded summaries and current selection for the exact supplied fence.
    fn discover(
        &self,
        request: &ProfileDiscoveryRequest,
    ) -> Result<ProfileDiscovery, ProfileDiscoveryError>;
}

/// Host-thread selection seam. Only the synthetic fixture implements it today.
pub trait ProfileSelectionPort {
    /// Reports whether this target can perform selection.
    fn selection_availability(&self) -> ProfileSelectionAvailability;

    /// Applies one explicitly fenced selection and returns authoritative readback.
    fn select(
        &mut self,
        request: ProfileSelectionRequest,
    ) -> Result<ProfileSelectionReceipt, ProfileSelectionRejection>;

    /// Reconciles a lost response without selecting another slot.
    fn reconcile(
        &self,
        idempotency_key: &SelectionIdempotencyKey,
    ) -> Result<ProfileSelectionReceipt, ProfileSelectionRejection>;
}

/// Fail-closed host boundary used until exact-host integration is authorized and verified.
#[derive(Debug, Default)]
pub struct UnavailableSaveProfileHost;

impl ProfileReadPort for UnavailableSaveProfileHost {
    fn discover(
        &self,
        _request: &ProfileDiscoveryRequest,
    ) -> Result<ProfileDiscovery, ProfileDiscoveryError> {
        Err(ProfileDiscoveryError::UnavailableHost)
    }
}

impl ProfileSelectionPort for UnavailableSaveProfileHost {
    fn selection_availability(&self) -> ProfileSelectionAvailability {
        ProfileSelectionAvailability::UnavailableHost
    }

    fn select(
        &mut self,
        _request: ProfileSelectionRequest,
    ) -> Result<ProfileSelectionReceipt, ProfileSelectionRejection> {
        Err(ProfileSelectionRejection::UnavailableHost)
    }

    fn reconcile(
        &self,
        _idempotency_key: &SelectionIdempotencyKey,
    ) -> Result<ProfileSelectionReceipt, ProfileSelectionRejection> {
        Err(ProfileSelectionRejection::UnavailableHost)
    }
}
