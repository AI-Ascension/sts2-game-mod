// SPDX-License-Identifier: MIT

use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sts2_game_mod::{
    CheckpointArtifactDescriptor, CheckpointManifest, CheckpointManifestBoundary,
    CheckpointManifestParts, CheckpointOrigin, EXACT_RESTORE_SCHEMA_DIGEST,
    ExactRestoreAuthorization, ExactRestoreError, ExactRestoreOwnerFence, blob_digest, state_id,
};

static FRAME_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) struct Fixture {
    pub(crate) owner: ExactRestoreOwnerFence,
    pub(crate) operation_id: String,
    pub(crate) branch: Value,
    pub(crate) exact_state_digest: String,
    pub(crate) checkpoint_id: String,
    pub(crate) manifest_bytes: Vec<u8>,
    pub(crate) manifest_digest: String,
    pub(crate) canonical_bytes: Vec<u8>,
    pub(crate) canonical_digest: String,
    pub(crate) restore_bytes: Vec<u8>,
    pub(crate) restore_digest: String,
    pub(crate) aggregate_closure_bytes: u64,
    pub(crate) begin_payload: Value,
}

impl Fixture {
    pub(crate) fn new(
        alias_restore_to_canonical: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let owner = fixture_owner()?;
        let operation_id = String::from("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb");
        let canonical_bytes = br#"{"hp":1,"rng":{"kind":"uint64","value":"5"}}"#.to_vec();
        let exact_state_digest = state_id(&canonical_bytes);
        let canonical_digest = blob_digest(&canonical_bytes);
        let restore_bytes = if alias_restore_to_canonical {
            canonical_bytes.clone()
        } else {
            b"native-restore-data".to_vec()
        };
        let restore_digest = blob_digest(&restore_bytes);
        let compatibility_digest = blob_digest(b"compatibility:fixture");
        let coverage_digest = blob_digest(b"coverage:fixture");
        let canonical_descriptor = CheckpointArtifactDescriptor::new(
            "asc-jcs-state-v1",
            canonical_digest.clone(),
            "exact_state_payload",
            canonical_bytes.len() as u64,
        )?;
        let restore_descriptor = CheckpointArtifactDescriptor::new(
            if alias_restore_to_canonical {
                "asc-jcs-state-v1"
            } else {
                "native-v1"
            },
            restore_digest.clone(),
            "fixture_snapshot",
            restore_bytes.len() as u64,
        )?;
        let manifest = CheckpointManifest::from_parts(CheckpointManifestParts {
            exact_state_digest: exact_state_digest.clone(),
            canonical_payload: canonical_descriptor,
            restore_artifacts: vec![restore_descriptor],
            compatibility_digest: compatibility_digest.clone(),
            coverage_contract_digest: coverage_digest.clone(),
            boundary: CheckpointManifestBoundary::new("decision", "stable", Some(7))?,
            origin: CheckpointOrigin::new("run:fixture", 1)?,
            parent_checkpoint_id: None,
        })?;
        let manifest_bytes = manifest.to_canonical_bytes()?;
        let checkpoint_id = manifest.exact_checkpoint_id()?;
        let manifest_digest = blob_digest(&manifest_bytes);
        let branch = json!({
            "experiment_id": "experiment:fixture",
            "branch_id": "branch:fixture",
            "metadata_revision": 1,
            "run_id": "run:fixture",
            "episode_id": "episode:fixture",
            "trajectory_id": "trajectory:fixture"
        });
        let owner_value = serde_json::to_value(&owner)?;
        let mut artifacts = vec![json!({
            "role": "canonical-state",
            "digest": canonical_digest,
            "size_bytes": canonical_bytes.len(),
            "codec": "asc-jcs-state-v1"
        })];
        artifacts.push(json!({
            "role": "fixture_snapshot",
            "digest": restore_digest,
            "size_bytes": restore_bytes.len(),
            "codec": if alias_restore_to_canonical { "asc-jcs-state-v1" } else { "native-v1" }
        }));
        let distinct_count = if canonical_digest == restore_digest {
            1
        } else {
            2
        };
        let aggregate_closure_bytes = manifest_bytes.len() as u64
            + canonical_bytes.len() as u64
            + if canonical_digest == restore_digest {
                0
            } else {
                restore_bytes.len() as u64
            };
        let closure_blobs = if canonical_digest == restore_digest {
            vec![(canonical_digest.as_str(), canonical_bytes.as_slice())]
        } else {
            vec![
                (canonical_digest.as_str(), canonical_bytes.as_slice()),
                (restore_digest.as_str(), restore_bytes.as_slice()),
            ]
        };
        let closure = closure_digest(&manifest_bytes, &closure_blobs);
        let begin_payload = json!({
            "operation_id": operation_id,
            "expected_owner": owner_value,
            "branch": branch,
            "checkpoint_id": checkpoint_id,
            "exact_state_digest": exact_state_digest,
            "manifest_digest": manifest_digest,
            "manifest_size_bytes": manifest_bytes.len(),
            "closure_digest": closure,
            "compatibility_digest": compatibility_digest,
            "coverage_contract_digest": coverage_digest,
            "boundary": {
                "kind": "decision",
                "phase": "stable",
                "game_tick": 7
            },
            "artifacts": artifacts,
            "artifact_reference_count": 2,
            "distinct_blob_count": distinct_count,
            "aggregate_closure_bytes": aggregate_closure_bytes
        });
        Ok(Self {
            owner,
            operation_id,
            branch,
            exact_state_digest,
            checkpoint_id,
            manifest_bytes,
            manifest_digest,
            canonical_bytes,
            canonical_digest,
            restore_bytes,
            restore_digest,
            aggregate_closure_bytes,
            begin_payload,
        })
    }

    pub(crate) fn rewrite_manifest(
        &mut self,
        mutate: impl FnOnce(&mut Value),
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut manifest: Value = serde_json::from_slice(&self.manifest_bytes)?;
        mutate(&mut manifest);
        self.manifest_bytes = serde_json::to_vec(&manifest)?;
        self.manifest_digest = blob_digest(&self.manifest_bytes);
        let mut checkpoint_id = Sha256::new();
        checkpoint_id.update(b"AI-ASCENSION/CHECKPOINT/v1\0");
        checkpoint_id.update(&self.manifest_bytes);
        self.checkpoint_id = format!(
            "asc-checkpoint:v1:sha256:{}",
            checkpoint_id
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        self.aggregate_closure_bytes = self.manifest_bytes.len() as u64
            + self.canonical_bytes.len() as u64
            + if self.canonical_digest == self.restore_digest {
                0
            } else {
                self.restore_bytes.len() as u64
            };
        let closure_blobs = if self.canonical_digest == self.restore_digest {
            vec![(
                self.canonical_digest.as_str(),
                self.canonical_bytes.as_slice(),
            )]
        } else {
            vec![
                (
                    self.canonical_digest.as_str(),
                    self.canonical_bytes.as_slice(),
                ),
                (self.restore_digest.as_str(), self.restore_bytes.as_slice()),
            ]
        };
        self.begin_payload["checkpoint_id"] = json!(self.checkpoint_id);
        self.begin_payload["manifest_digest"] = json!(self.manifest_digest);
        self.begin_payload["manifest_size_bytes"] = json!(self.manifest_bytes.len());
        self.begin_payload["aggregate_closure_bytes"] = json!(self.aggregate_closure_bytes);
        self.begin_payload["closure_digest"] =
            json!(closure_digest(&self.manifest_bytes, &closure_blobs));
        Ok(())
    }

    pub(crate) fn commit_payload(&self) -> Value {
        json!({
            "operation_id": self.operation_id,
            "expected_owner": self.owner,
            "branch": self.branch,
            "checkpoint_id": self.checkpoint_id,
            "closure_digest": self.begin_payload["closure_digest"],
            "exact_state_digest": self.exact_state_digest,
            "manifest_digest": self.manifest_digest
        })
    }
}

pub(crate) fn fixture_owner() -> Result<ExactRestoreOwnerFence, ExactRestoreError> {
    ExactRestoreOwnerFence::new(
        "11111111-1111-4111-8111-111111111111".into(),
        "22222222-2222-4222-8222-222222222222".into(),
        "33333333-3333-4333-8333-333333333333".into(),
        "44444444-4444-4444-8444-444444444444".into(),
        3,
        "55555555-5555-4555-8555-555555555555".into(),
        7,
        "66666666-6666-4666-8666-666666666666".into(),
        8,
        "session:fixture".into(),
        1_900_000_000_000,
    )
}

pub(crate) fn uuid() -> String {
    let number = FRAME_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("aaaaaaaa-aaaa-4aaa-8aaa-{number:012x}")
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{}", hex_digest(bytes))
}

pub(crate) fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn closure_digest(manifest: &[u8], blobs: &[(&str, &[u8])]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"STS2/EXACT-RESTORE-CLOSURE/v1\0");
    for bytes in std::iter::once(manifest).chain(blobs.iter().map(|(_, bytes)| *bytes)) {
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    format!(
        "sha256:{}",
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

pub(crate) fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied();
        let third = chunk.get(2).copied();
        output.push(ALPHABET[(first >> 2) as usize] as char);
        output
            .push(ALPHABET[(((first & 0x03) << 4) | (second.unwrap_or(0) >> 4)) as usize] as char);
        match (second, third) {
            (Some(second), Some(third)) => {
                output.push(ALPHABET[(((second & 0x0f) << 2) | (third >> 6)) as usize] as char);
                output.push(ALPHABET[(third & 0x3f) as usize] as char);
            }
            (Some(second), None) => {
                output.push(ALPHABET[((second & 0x0f) << 2) as usize] as char);
                output.push('=');
            }
            (None, _) => output.push_str("=="),
        }
    }
    output
}

pub(crate) fn request(kind: &str, payload: Value) -> Value {
    json!({
        "contract": "sts2-exact-restore-v1",
        "schema_digest": EXACT_RESTORE_SCHEMA_DIGEST,
        "message_id": uuid(),
        "correlation_id": uuid(),
        "kind": kind,
        "payload": payload
    })
}

pub(crate) fn authorization_for(
    frame: &Value,
    principal: &str,
) -> Result<ExactRestoreAuthorization, ExactRestoreError> {
    let owner = &frame["payload"]["expected_owner"];
    ExactRestoreAuthorization::new(
        principal.to_owned(),
        frame["correlation_id"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        owner["instance_id"].as_str().unwrap_or_default().to_owned(),
        owner["session_id"].as_str().unwrap_or_default().to_owned(),
        owner["lease_id"].as_str().unwrap_or_default().to_owned(),
        owner["lease_epoch"].as_u64().unwrap_or_default(),
    )
}

pub(crate) fn chunk_payload(
    fixture: &Fixture,
    bytes: &[u8],
    offset: u64,
    artifact_digest: &str,
) -> Value {
    json!({
        "operation_id": fixture.operation_id,
        "expected_owner": fixture.owner,
        "artifact_digest": artifact_digest,
        "offset": offset,
        "total_bytes": if artifact_digest == fixture.manifest_digest {
            fixture.manifest_bytes.len()
        } else if artifact_digest == fixture.restore_digest {
            fixture.restore_bytes.len()
        } else {
            fixture.canonical_bytes.len()
        },
        "chunk_digest": blob_digest(bytes),
        "data_base64": base64_encode(bytes)
    })
}

#[cfg(target_os = "linux")]
pub(crate) struct TestDirectory(pub(crate) std::path::PathBuf);

#[cfg(target_os = "linux")]
impl TestDirectory {
    pub(crate) fn new() -> std::io::Result<Self> {
        let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .canonicalize()?;
        let path = target.join(format!("exact-restore-test-{}", uuid()));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }
}

#[cfg(target_os = "linux")]
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
