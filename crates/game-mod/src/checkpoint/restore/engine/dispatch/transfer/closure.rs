// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::checkpoint::{
    CHECKPOINT_ID_DOMAIN, blob_digest, parse_canonical_text, state_id, to_canonical_bytes,
};

use super::super::super::super::wire::{canonical_json, digest, parse_unique_value};
use super::super::super::super::{
    EXACT_RESTORE_MAX_BLOB_BYTES, EXACT_RESTORE_MAX_CLOSURE_BYTES, EXACT_RESTORE_MAX_REFERENCES,
};
use super::super::super::state::RestoreOperation;
use super::super::super::{ExactRestoreError, ExactRestoreStore};

pub(super) fn verify_closure<S: ExactRestoreStore>(
    operation: &RestoreOperation,
    manifest_bytes: &[u8],
    store: &mut S,
) -> Result<(), ExactRestoreError> {
    let begin = &operation.immutable_begin;
    if digest(manifest_bytes) != string(begin, "manifest_digest")?
        || manifest_bytes.len() as u64 != number(begin, "manifest_size_bytes")?
    {
        return Err(ExactRestoreError::DigestMismatch);
    }
    let manifest = parse_unique_value(manifest_bytes).map_err(ExactRestoreError::from)?;
    if canonical_json(&manifest)
        .map_err(ExactRestoreError::from)?
        .as_slice()
        != manifest_bytes
    {
        return Err(ExactRestoreError::DigestMismatch);
    }
    validate_manifest_fields(begin, &manifest, manifest_bytes)?;
    validate_artifact_references(begin, &manifest)?;

    let manifest_progress = operation
        .blobs
        .iter()
        .find(|blob| blob.manifest)
        .ok_or(ExactRestoreError::StorageUnavailable)?;
    let manifest_key = manifest_progress.key(&operation.operation_id);
    let manifest_from_store = store.read_blob(&manifest_key, manifest_progress.total_bytes)?;
    if manifest_from_store != manifest_bytes {
        return Err(ExactRestoreError::DigestMismatch);
    }

    let artifacts = begin["artifacts"]
        .as_array()
        .ok_or(ExactRestoreError::InvalidFrame)?;
    let canonical = artifacts.first().ok_or(ExactRestoreError::InvalidFrame)?;
    let canonical_digest = string(canonical, "digest")?;
    let canonical_progress = operation
        .blobs
        .iter()
        .find(|blob| !blob.manifest && blob.digest == canonical_digest)
        .ok_or(ExactRestoreError::UnknownArtifact)?;
    let canonical_bytes = store.read_blob(
        &canonical_progress.key(&operation.operation_id),
        canonical_progress.total_bytes,
    )?;
    let parsed_state = parse_canonical_text(
        std::str::from_utf8(&canonical_bytes).map_err(|_| ExactRestoreError::DigestMismatch)?,
    )
    .map_err(|_| ExactRestoreError::DigestMismatch)?;
    if to_canonical_bytes(&parsed_state).map_err(|_| ExactRestoreError::DigestMismatch)?
        != canonical_bytes
        || blob_digest(&canonical_bytes) != canonical_digest
        || state_id(&canonical_bytes) != string(begin, "exact_state_digest")?
    {
        return Err(ExactRestoreError::DigestMismatch);
    }

    let mut closure = Sha256::new();
    closure.update(b"STS2/EXACT-RESTORE-CLOSURE/v1\0");
    update_closure_item(&mut closure, manifest_bytes)?;
    update_closure_item(&mut closure, &canonical_bytes)?;
    let mut seen = BTreeSet::new();
    seen.insert(canonical_digest.to_owned());
    for artifact in artifacts.iter().skip(1) {
        let digest_value = string(artifact, "digest")?;
        if !seen.insert(digest_value.to_owned()) {
            continue;
        }
        let progress = operation
            .blobs
            .iter()
            .find(|blob| !blob.manifest && blob.digest == digest_value)
            .ok_or(ExactRestoreError::UnknownArtifact)?;
        let bytes =
            store.read_blob(&progress.key(&operation.operation_id), progress.total_bytes)?;
        if blob_digest(&bytes) != digest_value {
            return Err(ExactRestoreError::DigestMismatch);
        }
        update_closure_item(&mut closure, &bytes)?;
    }
    let mut closure_digest = String::from("sha256:");
    for byte in closure.finalize() {
        closure_digest.push_str(&format!("{byte:02x}"));
    }
    if closure_digest != string(begin, "closure_digest")? {
        return Err(ExactRestoreError::DigestMismatch);
    }
    let total = manifest_bytes
        .len()
        .checked_add(
            operation
                .blobs
                .iter()
                .filter(|blob| !blob.manifest)
                .filter(|blob| seen.contains(&blob.digest))
                .try_fold(0_usize, |sum, blob| {
                    sum.checked_add(
                        usize::try_from(blob.total_bytes)
                            .map_err(|_| ExactRestoreError::InvalidFrame)?,
                    )
                    .ok_or(ExactRestoreError::InvalidFrame)
                })?,
        )
        .ok_or(ExactRestoreError::InvalidFrame)?;
    if total as u64 != number(begin, "aggregate_closure_bytes")?
        || total as u64 > EXACT_RESTORE_MAX_CLOSURE_BYTES
    {
        return Err(ExactRestoreError::DigestMismatch);
    }
    Ok(())
}

fn validate_manifest_fields(
    begin: &Value,
    manifest: &Value,
    manifest_bytes: &[u8],
) -> Result<(), ExactRestoreError> {
    if !has_exact_fields(
        manifest,
        &[
            "boundary",
            "canonical_payload",
            "canonical_profile",
            "compatibility_digest",
            "coverage_contract_digest",
            "exact_state_digest",
            "origin",
            "parent_checkpoint_id",
            "restore_artifacts",
            "schema",
        ],
        &[
            "boundary",
            "canonical_payload",
            "canonical_profile",
            "compatibility_digest",
            "coverage_contract_digest",
            "exact_state_digest",
            "origin",
            "restore_artifacts",
            "schema",
        ],
    ) || !has_exact_fields(
        &manifest["origin"],
        &["generation", "run_id"],
        &["generation", "run_id"],
    ) || !has_exact_fields(
        &manifest["boundary"],
        &["game_tick", "kind", "phase"],
        &["kind", "phase"],
    ) || !valid_descriptor(&manifest["canonical_payload"])
        || !manifest["restore_artifacts"]
            .as_array()
            .is_some_and(|items| items.iter().all(valid_descriptor))
        || manifest["schema"].as_str() != Some("ascension.checkpoint_manifest.v1")
        || manifest["canonical_profile"].as_str() != Some("asc-jcs-state-v1")
        || manifest["exact_state_digest"] != begin["exact_state_digest"]
        || manifest["compatibility_digest"] != begin["compatibility_digest"]
        || manifest["coverage_contract_digest"] != begin["coverage_contract_digest"]
        || manifest["boundary"] != begin["boundary"]
        || manifest["origin"]["run_id"] != begin["branch"]["run_id"]
        || manifest["origin"]["generation"]
            .as_u64()
            .is_none_or(|value| value == 0 || value > 9_007_199_254_740_991)
        || manifest["origin"]["run_id"]
            .as_str()
            .is_none_or(|value| value.is_empty() || value.len() > 256)
        || manifest["boundary"]["game_tick"]
            .as_u64()
            .is_some_and(|value| value > 9_007_199_254_740_991)
        || (!manifest["boundary"]["game_tick"].is_null()
            && manifest["boundary"]["game_tick"].as_u64().is_none())
        || !valid_optional_checkpoint_id(&manifest["parent_checkpoint_id"])
    {
        return Err(ExactRestoreError::DigestMismatch);
    }
    let mut checkpoint = Sha256::new();
    checkpoint.update(CHECKPOINT_ID_DOMAIN);
    checkpoint.update(manifest_bytes);
    let checkpoint = checkpoint
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if format!("asc-checkpoint:v1:sha256:{checkpoint}") != string(begin, "checkpoint_id")? {
        return Err(ExactRestoreError::DigestMismatch);
    }
    Ok(())
}

fn has_exact_fields(value: &Value, allowed: &[&str], required: &[&str]) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    object.keys().all(|key| allowed.contains(&key.as_str()))
        && required.iter().all(|key| object.contains_key(*key))
}

fn valid_descriptor(value: &Value) -> bool {
    has_exact_fields(
        value,
        &["codec", "digest", "role", "size_bytes"],
        &["codec", "digest", "role", "size_bytes"],
    ) && value["codec"].as_str().is_some_and(|text| !text.is_empty())
        && value["digest"].as_str().is_some_and(valid_blob_digest)
        && value["role"].as_str().is_some_and(|text| !text.is_empty())
        && value["size_bytes"]
            .as_u64()
            .is_some_and(|size| size <= EXACT_RESTORE_MAX_BLOB_BYTES)
}

fn valid_blob_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_optional_checkpoint_id(value: &Value) -> bool {
    if value.is_null() {
        return true;
    }
    value
        .as_str()
        .and_then(|text| text.strip_prefix("asc-checkpoint:v1:sha256:"))
        .is_some_and(|hex| {
            hex.len() == 64
                && hex
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        })
}

fn validate_artifact_references(begin: &Value, manifest: &Value) -> Result<(), ExactRestoreError> {
    let references = begin["artifacts"]
        .as_array()
        .ok_or(ExactRestoreError::InvalidFrame)?;
    let restore = manifest["restore_artifacts"]
        .as_array()
        .ok_or(ExactRestoreError::DigestMismatch)?;
    if references.len() != restore.len() + 1
        || references.len() > EXACT_RESTORE_MAX_REFERENCES
        || !artifact_matches(&references[0], &manifest["canonical_payload"], true)
    {
        return Err(ExactRestoreError::DigestMismatch);
    }
    for (reference, descriptor) in references.iter().skip(1).zip(restore) {
        if !artifact_matches(reference, descriptor, false) {
            return Err(ExactRestoreError::DigestMismatch);
        }
    }
    Ok(())
}

fn artifact_matches(reference: &Value, descriptor: &Value, canonical: bool) -> bool {
    let role_matches = if canonical {
        reference["role"].as_str() == Some("canonical-state")
    } else {
        reference["role"] == descriptor["role"]
    };
    role_matches
        && reference["digest"] == descriptor["digest"]
        && reference["size_bytes"] == descriptor["size_bytes"]
        && reference["codec"] == descriptor["codec"]
}

fn update_closure_item(hasher: &mut Sha256, bytes: &[u8]) -> Result<(), ExactRestoreError> {
    let length = u64::try_from(bytes.len()).map_err(|_| ExactRestoreError::InvalidFrame)?;
    hasher.update(length.to_be_bytes());
    hasher.update(bytes);
    Ok(())
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, ExactRestoreError> {
    value[key].as_str().ok_or(ExactRestoreError::InvalidFrame)
}

fn number(value: &Value, key: &str) -> Result<u64, ExactRestoreError> {
    value[key].as_u64().ok_or(ExactRestoreError::InvalidFrame)
}
