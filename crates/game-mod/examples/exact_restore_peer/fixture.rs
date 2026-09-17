// SPDX-License-Identifier: MIT

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sts2_game_mod::{
    ExactRestoreCapability, RestoreApplyOutcome, RestoreClosureView, RestoreHostApplier,
    blob_digest, state_id,
};

pub(crate) enum PeerApplier {
    Unsupported,
    Synthetic {
        output_dir: PathBuf,
        unknown: bool,
        calls: AtomicUsize,
    },
}

impl PeerApplier {
    pub(crate) fn synthetic(output_dir: PathBuf, unknown: bool) -> Self {
        Self::Synthetic {
            output_dir,
            unknown,
            calls: AtomicUsize::new(0),
        }
    }
}

impl RestoreHostApplier for PeerApplier {
    fn capability(&self) -> ExactRestoreCapability {
        match self {
            Self::Unsupported => ExactRestoreCapability::Unavailable,
            Self::Synthetic { .. } => ExactRestoreCapability::Available,
        }
    }

    fn restore(&mut self, request: &Value, closure: RestoreClosureView<'_>) -> RestoreApplyOutcome {
        let Self::Synthetic {
            output_dir,
            unknown,
            calls,
        } = self
        else {
            return RestoreApplyOutcome::Unknown;
        };
        let count = calls.fetch_add(1, Ordering::SeqCst) + 1;
        let Some(manifest_digest) = request["manifest_digest"].as_str() else {
            return RestoreApplyOutcome::Unknown;
        };
        let Some(exact_digest) = request["exact_state_digest"].as_str() else {
            return RestoreApplyOutcome::Unknown;
        };
        if blob_digest(closure.manifest) != manifest_digest {
            return RestoreApplyOutcome::Unknown;
        }
        let Ok(manifest) = serde_json::from_slice::<Value>(closure.manifest) else {
            return RestoreApplyOutcome::Unknown;
        };
        let Some(canonical_digest) = manifest["canonical_payload"]["digest"].as_str() else {
            return RestoreApplyOutcome::Unknown;
        };
        let Some((_, canonical)) = closure
            .blobs
            .iter()
            .find(|(digest, _)| *digest == canonical_digest)
        else {
            return RestoreApplyOutcome::Unknown;
        };
        if state_id(canonical) != exact_digest {
            return RestoreApplyOutcome::Unknown;
        }
        let count_path = output_dir.join("synthetic-applier-counter");
        let output_path = output_dir.join("synthetic-applier-output.json");
        let _ = std::fs::create_dir_all(output_dir);
        let _ = std::fs::write(&count_path, format!("{count}\n"));
        let evidence = json!({
            "mode": "synthetic",
            "counter": count,
            "manifest_digest": manifest_digest,
            "exact_state_digest": exact_digest,
            "manifest_bytes": closure.manifest.len(),
            "blobs": closure.blobs.iter().map(|(digest, bytes)| json!({
                "digest": digest,
                "bytes": bytes.len(),
            })).collect::<Vec<_>>(),
        });
        let _ = std::fs::write(
            &output_path,
            serde_json::to_vec(&evidence).unwrap_or_default(),
        );
        if *unknown {
            RestoreApplyOutcome::Unknown
        } else {
            RestoreApplyOutcome::Verified {
                recaptured_exact_state_digest: exact_digest.to_owned(),
            }
        }
    }
}

#[allow(dead_code)]
pub(crate) fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
