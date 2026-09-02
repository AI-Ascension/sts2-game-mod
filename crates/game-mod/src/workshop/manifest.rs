// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{
    WORKSHOP_ENTRYPOINT, WORKSHOP_LOADER_CONTRACT, WORKSHOP_MANIFEST_SCHEMA_VERSION,
    WORKSHOP_MAX_FILE_BYTES, WORKSHOP_MAX_FILES, WORKSHOP_MAX_MANIFEST_BYTES,
    WORKSHOP_MAX_TEXT_BYTES, WORKSHOP_PACKAGE_ID,
};

/// The only executable content type accepted by this first-party package policy.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkshopContentKind {
    FirstPartyExecutable,
}

/// The role of a file in the first-party package allowlist.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkshopFileRole {
    ManagedAssembly,
    NativeLibrary,
    LoaderManifest,
}

/// A hashed payload entry in a Workshop manifest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkshopFile {
    pub path: String,
    pub role: WorkshopFileRole,
    pub size_bytes: u64,
    pub sha256: String,
}

/// The checked-in/runtime-consumed Workshop package descriptor.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkshopManifest {
    pub schema_version: String,
    pub package_id: String,
    pub package_version: String,
    pub consumer_app_id: u32,
    pub published_file_id: u64,
    pub game_version: String,
    pub platform: String,
    pub loader_contract: String,
    pub content_kind: WorkshopContentKind,
    pub entrypoint: String,
    pub files: Vec<WorkshopFile>,
    pub content_digest: String,
    pub source_revision: String,
}

impl WorkshopManifest {
    /// Parses and validates one bounded JSON manifest.
    pub fn parse(bytes: &[u8]) -> Result<Self, WorkshopManifestError> {
        if bytes.len() > WORKSHOP_MAX_MANIFEST_BYTES {
            return Err(WorkshopManifestError::ManifestTooLarge);
        }
        let manifest: Self =
            serde_json::from_slice(bytes).map_err(|_| WorkshopManifestError::MalformedJson)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validates manifest shape without selecting an app, item, or host version.
    pub fn validate(&self) -> Result<(), WorkshopManifestError> {
        if self.schema_version != WORKSHOP_MANIFEST_SCHEMA_VERSION {
            return Err(WorkshopManifestError::SchemaVersion);
        }
        validate_token(&self.package_id, false)?;
        validate_token(&self.package_version, false)?;
        if self.consumer_app_id == 0 {
            return Err(WorkshopManifestError::ConsumerAppId);
        }
        validate_token(&self.game_version, false)?;
        validate_token(&self.platform, false)?;
        if self.loader_contract != WORKSHOP_LOADER_CONTRACT {
            return Err(WorkshopManifestError::LoaderContract);
        }
        if self.content_kind != WorkshopContentKind::FirstPartyExecutable {
            return Err(WorkshopManifestError::ContentKind);
        }
        if self.entrypoint != WORKSHOP_ENTRYPOINT {
            return Err(WorkshopManifestError::Entrypoint);
        }
        if self.files.is_empty() || self.files.len() > WORKSHOP_MAX_FILES {
            return Err(WorkshopManifestError::FileCount);
        }

        let mut seen_paths = BTreeSet::new();
        let mut previous_path: Option<&str> = None;
        for file in &self.files {
            validate_file(file)?;
            if previous_path.is_some_and(|previous| file.path.as_str() <= previous) {
                return Err(WorkshopManifestError::FileInventoryNotSorted);
            }
            if !seen_paths.insert(file.path.to_ascii_lowercase()) {
                return Err(WorkshopManifestError::DuplicatePath);
            }
            previous_path = Some(file.path.as_str());
        }

        let Some(entrypoint) = self.files.iter().find(|file| file.path == self.entrypoint) else {
            return Err(WorkshopManifestError::EntrypointMissing);
        };
        if entrypoint.role != WorkshopFileRole::LoaderManifest {
            return Err(WorkshopManifestError::EntrypointRole);
        }
        validate_digest(&self.content_digest)?;
        validate_token(&self.source_revision, true)?;
        Ok(())
    }

    /// Checks this manifest against the exact first-party package policy.
    pub fn validate_against(
        &self,
        policy: &WorkshopPolicy,
    ) -> Result<(), WorkshopCompatibilityError> {
        if self.consumer_app_id != policy.consumer_app_id {
            return Err(WorkshopCompatibilityError::ConsumerAppId);
        }
        if let Some(expected_item_id) = policy.expected_item_id
            && self.published_file_id != expected_item_id
        {
            return Err(WorkshopCompatibilityError::PublishedFileId);
        }
        if self.package_id != policy.package_id
            || self.game_version != policy.game_version
            || self.platform != policy.platform
            || self.loader_contract != policy.loader_contract
            || self.entrypoint != policy.entrypoint
        {
            return Err(WorkshopCompatibilityError::PackageCompatibility);
        }
        if self.content_kind != WorkshopContentKind::FirstPartyExecutable
            || self.files.len() != policy.allowed_files.len()
            || self.files.iter().any(|file| {
                !policy
                    .allowed_files
                    .iter()
                    .any(|allowed| allowed.path == file.path && allowed.role == file.role)
            })
        {
            return Err(WorkshopCompatibilityError::FileAllowlist);
        }
        Ok(())
    }
}

/// A path/role pair permitted by the first-party package policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AllowedWorkshopFile {
    pub path: String,
    pub role: WorkshopFileRole,
}

/// Exact compatibility and trust policy for one first-party Workshop item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkshopPolicy {
    pub consumer_app_id: u32,
    pub expected_item_id: Option<u64>,
    pub package_id: String,
    pub game_version: String,
    pub platform: String,
    pub loader_contract: String,
    pub entrypoint: String,
    pub allowed_files: Vec<AllowedWorkshopFile>,
}

impl WorkshopPolicy {
    /// Creates a policy for a first-party executable package.
    #[must_use]
    pub fn first_party(
        consumer_app_id: u32,
        expected_item_id: Option<u64>,
        game_version: &str,
        platform: &str,
        allowed_files: Vec<AllowedWorkshopFile>,
    ) -> Self {
        Self {
            consumer_app_id,
            expected_item_id,
            package_id: WORKSHOP_PACKAGE_ID.to_owned(),
            game_version: game_version.to_owned(),
            platform: platform.to_owned(),
            loader_contract: WORKSHOP_LOADER_CONTRACT.to_owned(),
            entrypoint: WORKSHOP_ENTRYPOINT.to_owned(),
            allowed_files,
        }
    }
}

impl std::fmt::Display for WorkshopManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for WorkshopManifestError {}

/// Shape failures for the manifest itself.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkshopManifestError {
    ManifestTooLarge,
    MalformedJson,
    SchemaVersion,
    InvalidToken,
    ConsumerAppId,
    LoaderContract,
    ContentKind,
    Entrypoint,
    FileCount,
    InvalidFilePath,
    InvalidFileSize,
    InvalidFileDigest,
    FileInventoryNotSorted,
    DuplicatePath,
    EntrypointMissing,
    EntrypointRole,
    InvalidContentDigest,
}

/// Policy mismatch failures after manifest shape validation succeeds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkshopCompatibilityError {
    ConsumerAppId,
    PublishedFileId,
    PackageCompatibility,
    FileAllowlist,
}

impl std::fmt::Display for WorkshopCompatibilityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for WorkshopCompatibilityError {}

fn validate_file(file: &WorkshopFile) -> Result<(), WorkshopManifestError> {
    validate_relative_path(&file.path)?;
    if file.size_bytes == 0 || file.size_bytes > WORKSHOP_MAX_FILE_BYTES {
        return Err(WorkshopManifestError::InvalidFileSize);
    }
    validate_digest(&file.sha256).map_err(|_| WorkshopManifestError::InvalidFileDigest)?;
    let valid_extension = match file.role {
        WorkshopFileRole::ManagedAssembly => file.path.ends_with(".dll"),
        WorkshopFileRole::NativeLibrary => {
            file.path.ends_with(".dll")
                || file.path.ends_with(".so")
                || file.path.ends_with(".dylib")
        }
        WorkshopFileRole::LoaderManifest => file.path.ends_with(".json"),
    };
    if !valid_extension {
        return Err(WorkshopManifestError::InvalidFilePath);
    }
    Ok(())
}

fn validate_relative_path(value: &str) -> Result<(), WorkshopManifestError> {
    if value.is_empty()
        || value.len() > WORKSHOP_MAX_TEXT_BYTES
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.contains('\\')
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-/".contains(&byte))
    {
        return Err(WorkshopManifestError::InvalidFilePath);
    }
    Ok(())
}

fn validate_token(value: &str, allow_slash: bool) -> Result<(), WorkshopManifestError> {
    if value.is_empty()
        || value.len() > WORKSHOP_MAX_TEXT_BYTES
        || value.contains("..")
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || b"._:-".contains(&byte) || (allow_slash && byte == b'/')
        })
    {
        return Err(WorkshopManifestError::InvalidToken);
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), WorkshopManifestError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(WorkshopManifestError::InvalidContentDigest);
    }
    Ok(())
}
