// SPDX-License-Identifier: MIT

use super::{WorkshopCompatibilityError, WorkshopManifest, WorkshopManifestError, WorkshopPolicy};

/// Steam-side install state translated into a stable owner-local state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkshopInstallState {
    Missing,
    DownloadPending,
    Downloading,
    NeedsUpdate,
    Installed,
    Corrupt,
}

/// The bounded state snapshot supplied by a managed/Steam adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkshopItemSnapshot {
    pub consumer_app_id: u32,
    pub item_id: u64,
    pub subscribed: bool,
    pub download_app_id: Option<u32>,
    pub state: WorkshopInstallState,
    pub install_path: Option<String>,
    pub manifest: Option<Vec<u8>>,
}

/// A validated package ready for an owner-controlled loader handoff.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkshopPackage {
    pub item_id: u64,
    pub install_path: String,
    pub manifest: WorkshopManifest,
}

/// Result of inspecting a Steam Workshop item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkshopReadiness {
    Wait(WorkshopWaitReason),
    Ready(Box<WorkshopPackage>),
    Reject(WorkshopReject),
}

/// Non-terminal states that require a later Steam install notification or poll.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkshopWaitReason {
    NotSubscribed,
    DownloadPending,
    Downloading,
    NeedsUpdate,
}

/// Terminal reasons for refusing a Workshop item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkshopReject {
    ConsumerAppId,
    ItemId,
    DownloadAppId,
    MissingInstallPath,
    UnsafeInstallPath,
    MissingManifest,
    Corrupt,
    Manifest(WorkshopManifestError),
    Incompatible(WorkshopCompatibilityError),
}

/// The pure consumer decision boundary for Steam/managed adapters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkshopConsumer {
    policy: WorkshopPolicy,
}

impl WorkshopConsumer {
    /// Creates a consumer with an exact first-party package policy.
    #[must_use]
    pub fn new(policy: WorkshopPolicy) -> Self {
        Self { policy }
    }

    /// Returns whether an item is waiting, ready, or rejected.
    #[must_use]
    pub fn inspect(&self, snapshot: WorkshopItemSnapshot) -> WorkshopReadiness {
        if snapshot.consumer_app_id != self.policy.consumer_app_id {
            return WorkshopReadiness::Reject(WorkshopReject::ConsumerAppId);
        }
        if self
            .policy
            .expected_item_id
            .is_some_and(|expected| snapshot.item_id != expected)
        {
            return WorkshopReadiness::Reject(WorkshopReject::ItemId);
        }
        if snapshot
            .download_app_id
            .is_some_and(|download_app| download_app != self.policy.consumer_app_id)
        {
            return WorkshopReadiness::Reject(WorkshopReject::DownloadAppId);
        }
        if !snapshot.subscribed {
            return WorkshopReadiness::Wait(WorkshopWaitReason::NotSubscribed);
        }
        match snapshot.state {
            WorkshopInstallState::Missing => {
                WorkshopReadiness::Wait(WorkshopWaitReason::DownloadPending)
            }
            WorkshopInstallState::DownloadPending => {
                WorkshopReadiness::Wait(WorkshopWaitReason::DownloadPending)
            }
            WorkshopInstallState::Downloading => {
                WorkshopReadiness::Wait(WorkshopWaitReason::Downloading)
            }
            WorkshopInstallState::NeedsUpdate => {
                WorkshopReadiness::Wait(WorkshopWaitReason::NeedsUpdate)
            }
            WorkshopInstallState::Corrupt => WorkshopReadiness::Reject(WorkshopReject::Corrupt),
            WorkshopInstallState::Installed => self.inspect_installed(snapshot),
        }
    }

    fn inspect_installed(&self, snapshot: WorkshopItemSnapshot) -> WorkshopReadiness {
        let Some(install_path) = snapshot.install_path else {
            return WorkshopReadiness::Reject(WorkshopReject::MissingInstallPath);
        };
        if !is_safe_absolute_path(&install_path) {
            return WorkshopReadiness::Reject(WorkshopReject::UnsafeInstallPath);
        }
        let Some(manifest_bytes) = snapshot.manifest else {
            return WorkshopReadiness::Reject(WorkshopReject::MissingManifest);
        };
        let manifest = match WorkshopManifest::parse(&manifest_bytes) {
            Ok(value) => value,
            Err(error) => return WorkshopReadiness::Reject(WorkshopReject::Manifest(error)),
        };
        if let Err(error) = manifest.validate_against(&self.policy) {
            return WorkshopReadiness::Reject(WorkshopReject::Incompatible(error));
        }
        WorkshopReadiness::Ready(Box::new(WorkshopPackage {
            item_id: snapshot.item_id,
            install_path,
            manifest,
        }))
    }
}

fn is_safe_absolute_path(value: &str) -> bool {
    let windows_drive = value.len() >= 3
        && value.as_bytes()[0].is_ascii_alphabetic()
        && value.as_bytes()[1] == b':'
        && matches!(value.as_bytes()[2], b'/' | b'\\');
    let unc = value.starts_with("//") || value.starts_with("\\\\");
    let absolute = value.starts_with('/') || windows_drive || unc;
    absolute
        && value
            .split(['/', '\\'])
            .all(|part| part != "." && part != "..")
        && !value.contains('\0')
}
