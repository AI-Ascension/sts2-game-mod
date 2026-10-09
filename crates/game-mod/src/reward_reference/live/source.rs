// SPDX-License-Identifier: MIT

use super::super::{RewardCatalogBinding, RewardSourceError};
use super::identity::RewardLiveInstanceId;
use super::snapshot::RewardLiveSnapshotInput;

/// Request fence passed to one object-safe live source port.
#[derive(Debug)]
pub struct RewardLiveCaptureRequest<'a> {
    pub(super) catalog: &'a RewardCatalogBinding,
    pub(super) instance_id: &'a RewardLiveInstanceId,
}

impl RewardLiveCaptureRequest<'_> {
    /// Expected static catalog binding for the next owned capture.
    #[must_use]
    pub const fn catalog(&self) -> &RewardCatalogBinding {
        self.catalog
    }

    /// Expected selected game instance.
    #[must_use]
    pub const fn instance_id(&self) -> &RewardLiveInstanceId {
        self.instance_id
    }
}

/// Object-safe boundary for copying detached current live reward facts.
///
/// An implementation must return only currently visible source facts as owned values. It must
/// not claim, select, evaluate RNG, retain host objects, or infer missing host fields.
pub trait RewardLiveSource {
    /// Copies one bounded live snapshot for the requested catalog and game instance.
    ///
    /// The returned snapshot follows [`RewardLiveSnapshotInput`]'s parent-scoped uniqueness
    /// contract; nested raw IDs need not be globally unique across different owners.
    fn capture(
        &self,
        request: &RewardLiveCaptureRequest<'_>,
    ) -> Result<RewardLiveSnapshotInput, RewardSourceError>;
}
