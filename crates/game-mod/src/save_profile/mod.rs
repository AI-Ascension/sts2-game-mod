// SPDX-License-Identifier: MIT

//! Owner-local save-slot discovery and selection safety boundary.
//!
//! This module deliberately contains no filesystem access, save payload, host reflection,
//! transport route, or disposable-directory provisioning. `FakeSaveProfileHost` is a synthetic
//! in-memory fixture for exercising fences and reconciliation. `UnavailableSaveProfileHost`
//! keeps real host discovery and selection explicitly unavailable until an authorized owner
//! contract and exact-host evidence exist.

mod error;
mod fixture;
mod identity;
mod model;
mod port;

pub use error::{ProfileDiscoveryError, ProfileFixtureError, ProfileSelectionRejection};
pub use fixture::FakeSaveProfileHost;
pub use identity::{
    BaselineFence, HostCompatibility, InstanceIdentity, ProfileIdentityError, ProviderProfileId,
    SaveProfileBaseline, SaveSlotId, SelectionAuthority, SelectionIdempotencyKey, UserDataIdentity,
    WorkflowProfileId,
};
pub use model::{
    ProfileDiscovery, ProfileDiscoveryRequest, ProfileSelectionReceipt, ProfileSelectionRequest,
    SaveProfileStatus, SaveSlotSummary,
};
pub use port::{
    DisposableProvisioningAvailability, ProfileReadPort, ProfileSelectionAvailability,
    ProfileSelectionPort, UnavailableSaveProfileHost,
};

/// Maximum number of summaries returned by the owner-local fixture.
pub const SAVE_PROFILE_MAX_SLOTS: usize = 32;
/// Maximum byte length of an opaque identity or bounded compatibility value.
pub const SAVE_PROFILE_MAX_ID_BYTES: usize = 128;
