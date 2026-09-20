// SPDX-License-Identifier: MIT

//! Profile identity, explicit authorization and the catalog binding they fence.

use crate::ContentCursorBinding;
use crate::save_profile::{SaveProfileBaseline, SaveSlotId, UserDataIdentity};

/// Kind of profile a progression snapshot was read under.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProgressionProfileKind {
    /// The profile the owner has active.
    Active,
    /// An offline profile that requires an explicitly supported read port.
    Offline,
    /// A foreign profile owned by another user-data identity.
    Foreign,
}

/// Owner identity of the profile the snapshot was read under.
///
/// The identity reuses the profile-management contract, so this boundary never invents a second
/// profile selector and never addresses a save path.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProgressionProfile {
    /// Opaque gateway/owner user-data identity.
    pub user_data_id: UserDataIdentity,
    /// Opaque save slot the profile was read from.
    pub save_slot: SaveSlotId,
    /// Whether the profile is active, offline or foreign.
    pub kind: ProgressionProfileKind,
}

/// Whether the source established explicit authority to read this profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProgressionProfilePermit {
    /// The owner explicitly permitted reading this exact profile identity.
    Permitted {
        /// Opaque explicit authority witness.
        authority: crate::save_profile::SelectionAuthority,
    },
    /// No explicit permission was established, so no progression is published.
    NotPermitted,
}

/// Static catalog identity: manifest, locale, profile, profile revision and producer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressionCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized progression value.
    pub locale: String,
    /// Profile selected when the entries were read.
    pub profile: ProgressionProfile,
    /// Read-only freshness witness for that profile, reusing the profile baseline.
    pub baseline: SaveProfileBaseline,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

impl ProgressionCatalogBinding {
    /// Returns the profile revision every query must name to be answered.
    #[must_use]
    pub fn revision(&self) -> ProgressionRevision {
        ProgressionRevision {
            user_data_id: self.profile.user_data_id.clone(),
            revision: self.baseline.revision(),
        }
    }
}

/// Documented freshness of one progression read, tied to one profile revision.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProgressionRevision {
    /// Opaque user-data identity the revision belongs to.
    pub user_data_id: UserDataIdentity,
    /// Owner-local monotonic profile revision.
    pub revision: u64,
}

/// Profile a caller believes it is reading.
///
/// Naming the active profile under its own identity is not a selection; naming any other profile,
/// or naming one whose kind needs a read seam this boundary does not hold, is refused rather than
/// answered by switching the active profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProgressionProfileQuery {
    /// The profile the caller has active, with no identity re-stated.
    Active,
    /// One exact profile named by identity and kind.
    Named {
        /// Opaque user-data identity the caller believes it is reading.
        user_data_id: UserDataIdentity,
        /// Kind of profile the caller believes it is reading.
        kind: ProgressionProfileKind,
    },
}

/// Profile binding and authorization supplied by the source for one snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressionProfileInput {
    /// Owner identity of the profile.
    pub profile: ProgressionProfile,
    /// Read-only freshness witness for that profile.
    pub baseline: SaveProfileBaseline,
    /// Explicit permission to read this profile, or its stated absence.
    pub permit: ProgressionProfilePermit,
}
