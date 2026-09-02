// SPDX-License-Identifier: MIT

mod consumer;
mod manifest;

pub use consumer::{
    WorkshopConsumer, WorkshopInstallState, WorkshopItemSnapshot, WorkshopPackage,
    WorkshopReadiness, WorkshopReject, WorkshopWaitReason,
};
pub use manifest::{
    AllowedWorkshopFile, WorkshopCompatibilityError, WorkshopContentKind, WorkshopFile,
    WorkshopFileRole, WorkshopManifest, WorkshopManifestError, WorkshopPolicy,
};

/// Version of the owner-local Workshop package manifest.
pub const WORKSHOP_MANIFEST_SCHEMA_VERSION: &str = "sts2-workshop-manifest-v1";
/// Managed loader contract required by the first-party package.
pub const WORKSHOP_LOADER_CONTRACT: &str = "sts2-managed-loader-v1";
/// Stable identity of the first-party STS2 mod package.
pub const WORKSHOP_PACKAGE_ID: &str = "ai-ascension.sts2-game-mod";
/// Maximum encoded manifest size accepted by the validator.
pub const WORKSHOP_MAX_MANIFEST_BYTES: usize = 64 * 1024;
/// Maximum number of payload files accepted by one Workshop item.
pub const WORKSHOP_MAX_FILES: usize = 16;
/// Maximum size of one payload file represented by the manifest.
pub const WORKSHOP_MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
/// Maximum length of an item/package path or metadata token.
pub const WORKSHOP_MAX_TEXT_BYTES: usize = 256;

pub(crate) const WORKSHOP_ENTRYPOINT: &str = "AIAscensionSTS2Poc.json";
