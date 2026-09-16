// SPDX-License-Identifier: MIT

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::json;
use sts2_game_mod::{ExactRestoreEngine, SecureExactRestoreStore};

use super::support::*;

#[test]
fn complete_closure_rejects_manifest_extensions_and_invalid_origin_witness()
-> Result<(), Box<dyn std::error::Error>> {
    for mutation in ["extension", "origin_generation"] {
        let mut fixture = Fixture::new(false)?;
        fixture.rewrite_manifest(|manifest| match mutation {
            "extension" => manifest["unexpected"] = json!("not part of the checkpoint envelope"),
            "origin_generation" => manifest["origin"]["generation"] = json!(0),
            _ => unreachable!(),
        })?;
        let shared_store = Arc::new(Mutex::new(StoreState::default()));
        let owner = Arc::new(Mutex::new(current_owner(&fixture.owner)));
        let mut engine = create_engine(
            shared_store,
            owner,
            RecordingApplier::unknown(Arc::new(AtomicUsize::new(0))),
        )?;
        send(
            &mut engine,
            "exact_restore_begin_request",
            fixture.begin_payload.clone(),
        )?;
        send_blob(
            &mut engine,
            &fixture,
            &fixture.manifest_bytes,
            &fixture.manifest_digest,
        )?;
        send_blob(
            &mut engine,
            &fixture,
            &fixture.canonical_bytes,
            &fixture.canonical_digest,
        )?;
        let final_chunk =
            chunk_payload(&fixture, &fixture.restore_bytes, 0, &fixture.restore_digest);
        let accepted = send(&mut engine, "exact_restore_chunk_request", final_chunk)?;
        assert_eq!(accepted["payload"]["result"], "CHUNK_ACCEPTED");
        let rejected = send(
            &mut engine,
            "exact_restore_finish_blob_request",
            json!({
                "operation_id": fixture.operation_id,
                "expected_owner": fixture.owner,
                "artifact_digest": fixture.restore_digest,
                "total_bytes": fixture.restore_bytes.len()
            }),
        )?;
        assert_eq!(rejected["payload"]["error_code"], "digest_mismatch");
        assert_eq!(engine.staged_bytes(), fixture.aggregate_closure_bytes);
    }
    Ok(())
}

#[test]
fn process_loss_after_commit_intent_recovers_unknown_without_second_effect()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(false)?;
    let shared_store = Arc::new(Mutex::new(StoreState::default()));
    let owner = Arc::new(Mutex::new(current_owner(&fixture.owner)));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut engine = create_engine(
        shared_store.clone(),
        owner.clone(),
        RecordingApplier::interrupt_after_start(calls.clone()),
    )?;
    send(
        &mut engine,
        "exact_restore_begin_request",
        fixture.begin_payload.clone(),
    )?;
    send_blob(
        &mut engine,
        &fixture,
        &fixture.manifest_bytes,
        &fixture.manifest_digest,
    )?;
    send_blob(
        &mut engine,
        &fixture,
        &fixture.canonical_bytes,
        &fixture.canonical_digest,
    )?;
    send_blob(
        &mut engine,
        &fixture,
        &fixture.restore_bytes,
        &fixture.restore_digest,
    )?;
    let commit = fixture.commit_payload();
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _ = send(&mut engine, "exact_restore_commit_request", commit.clone());
        }))
        .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    drop(engine);

    let mut reopened = create_engine(
        shared_store,
        owner,
        RecordingApplier::interrupt_after_start(calls.clone()),
    )?;
    let recovered = send(&mut reopened, "exact_restore_commit_request", commit)?;
    assert_eq!(recovered["payload"]["result"], "UNKNOWN");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let lookup = send(
        &mut reopened,
        "exact_restore_lookup_request",
        json!({
            "operation_id": fixture.operation_id,
            "expected_owner": fixture.owner
        }),
    )?;
    assert_eq!(lookup["payload"]["result"], "UNKNOWN");
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn private_store_reopens_staging_and_refuses_symlink_or_shared_directories()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let scratch = TestDirectory::new()?;
    let fixture = Fixture::new(false)?;
    let root = scratch.0.join("private");
    let shared_store = SecureExactRestoreStore::open(&root)?;
    let owner = Arc::new(Mutex::new(current_owner(&fixture.owner)));
    let mut engine = ExactRestoreEngine::new(
        shared_store,
        SharedOwner(owner.clone()),
        RecordingApplier::unknown(Arc::new(AtomicUsize::new(0))),
        PRINCIPAL.into(),
    )?;
    send(
        &mut engine,
        "exact_restore_begin_request",
        fixture.begin_payload.clone(),
    )?;
    let partial = chunk_payload(
        &fixture,
        &fixture.canonical_bytes,
        0,
        &fixture.canonical_digest,
    );
    send(&mut engine, "exact_restore_chunk_request", partial)?;
    drop(engine);

    let reopened_store = SecureExactRestoreStore::open(&root)?;
    let mut reopened = ExactRestoreEngine::new(
        reopened_store,
        SharedOwner(owner),
        RecordingApplier::unknown(Arc::new(AtomicUsize::new(0))),
        PRINCIPAL.into(),
    )?;
    let lookup = send(
        &mut reopened,
        "exact_restore_lookup_request",
        json!({
            "operation_id": fixture.operation_id,
            "expected_owner": fixture.owner,
            "artifact_digest": fixture.canonical_digest
        }),
    )?;
    assert_eq!(lookup["payload"]["state"], "STAGING");
    assert_eq!(
        lookup["payload"]["next_offset"],
        fixture.canonical_bytes.len()
    );

    let metadata = std::fs::metadata(&root)?;
    assert_eq!(metadata.permissions().mode() & 0o777, 0o700);
    let data_name = format!(
        "{}-blob-{}.data",
        fixture.operation_id,
        fixture.canonical_digest.trim_start_matches("sha256:")
    );
    for name in [
        String::from("owner.lock"),
        String::from("index.json"),
        data_name,
    ] {
        let metadata = std::fs::metadata(root.join(name))?;
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    }

    let link = scratch.0.join("private-link");
    symlink(&root, &link)?;
    assert!(SecureExactRestoreStore::open(&link).is_err());
    let shared = scratch.0.join("shared");
    std::fs::create_dir(&shared)?;
    std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o755))?;
    assert!(SecureExactRestoreStore::open(&shared).is_err());
    Ok(())
}
