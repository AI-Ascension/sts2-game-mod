// SPDX-License-Identifier: MIT

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use sts2_game_mod::{EXACT_RESTORE_SCHEMA_DIGEST, exact_restore_unavailable_response};

#[path = "checkpoint_restore/support/mod.rs"]
mod support;
use support::*;

#[path = "checkpoint_restore/recovery.rs"]
mod recovery;

const EXPECTED_SCHEMA_HEX: &str =
    "2289d888c33eac46873408303c4423eab762e3f7bd6132ae8ae88d0d3b1858e4";

#[test]
fn copied_protocol_artifact_and_manifest_checksums_match_the_pin()
-> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(EXACT_RESTORE_SCHEMA_DIGEST, EXPECTED_SCHEMA_HEX);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../protocol-artifact/exact-restore-v1");
    let sums = std::fs::read_to_string(root.join("SHA256SUMS"))?;
    let mut files = 0;
    for line in sums.lines() {
        let (expected, relative) = line
            .split_once("  ")
            .ok_or("malformed exact-restore SHA256SUMS entry")?;
        let actual = hex_digest(&std::fs::read(root.join(relative))?);
        assert_eq!(actual, expected, "protocol artifact file {relative}");
        files += 1;
    }
    assert!(files >= 5);
    assert_eq!(
        hex_digest(&std::fs::read(root.join("schema.json"))?),
        EXACT_RESTORE_SCHEMA_DIGEST
    );
    Ok(())
}

#[test]
fn all_protocol_golden_requests_are_closed_and_correlated_refusals()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../protocol-artifact/exact-restore-v1");
    let golden: Value = serde_json::from_slice(&std::fs::read(root.join("golden/frames.json"))?)?;
    for frame in golden["frames"]
        .as_array()
        .ok_or("missing frame vectors")?
        .iter()
        .filter(|frame| {
            frame["kind"]
                .as_str()
                .is_some_and(|kind| kind.ends_with("_request"))
        })
    {
        let owner = frame["payload"]["expected_owner"].clone();
        let authorization = authorization_for(frame, PRINCIPAL)?;
        let body = serde_json::to_vec(frame)?;
        let kind = frame["kind"].as_str().ok_or("missing golden kind")?;
        let response = exact_restore_unavailable_response(&body, &authorization, PRINCIPAL, kind)?;
        assert!(response.body.len() <= 16_384);
        let response: Value = serde_json::from_slice(&response.body)?;
        assert_eq!(response["correlation_id"], frame["message_id"]);
        assert_eq!(
            response["payload"]["operation_id"],
            frame["payload"]["operation_id"]
        );
        assert_eq!(response["payload"]["expected_owner"], owner);
        assert_eq!(
            response["payload"]["request_digest"].as_str().map(str::len),
            Some(71)
        );
        assert_eq!(response["payload"]["host_effect"], "not_started");
    }
    Ok(())
}

#[test]
fn duplicate_members_and_outer_correlation_mismatch_fail_closed()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(false)?;
    let frame = request("exact_restore_begin_request", fixture.begin_payload.clone());
    let body = serde_json::to_vec(&frame)?;
    let mut authorization = authorization_for(&frame, PRINCIPAL)?;
    authorization.correlation_id = uuid();
    assert!(
        exact_restore_unavailable_response(
            &body,
            &authorization,
            PRINCIPAL,
            "exact_restore_begin_request"
        )
        .is_err()
    );

    let duplicate = br#"{"contract":"sts2-exact-restore-v1","contract":"sts2-exact-restore-v1"}"#;
    assert!(
        exact_restore_unavailable_response(
            duplicate,
            &authorization,
            PRINCIPAL,
            "exact_restore_begin_request"
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn exact_closure_stages_verifies_and_synthetic_applier_receipts_once()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(false)?;
    let shared_store = Arc::new(Mutex::new(StoreState::default()));
    let owner = Arc::new(Mutex::new(current_owner(&fixture.owner)));
    let calls = Arc::new(AtomicUsize::new(0));
    let applier = RecordingApplier::verified(calls.clone(), fixture.exact_state_digest.clone());
    let mut engine = create_engine(shared_store.clone(), owner, applier)?;

    let begin = send(
        &mut engine,
        "exact_restore_begin_request",
        fixture.begin_payload.clone(),
    )?;
    assert_eq!(begin["payload"]["result"], "CREATED", "{begin:?}");
    assert_eq!(begin["payload"]["state"], "STAGING");
    let duplicate = send(
        &mut engine,
        "exact_restore_begin_request",
        fixture.begin_payload.clone(),
    )?;
    assert_eq!(duplicate["payload"]["result"], "EXISTING");
    assert_eq!(engine.operation_count(), 1);

    let mut second_begin = fixture.begin_payload.clone();
    second_begin["operation_id"] = json!("cccccccc-cccc-4ccc-8ccc-cccccccccccc");
    let full = send(&mut engine, "exact_restore_begin_request", second_begin)?;
    assert_eq!(full["payload"]["error_code"], "capacity_full");
    assert_eq!(engine.operation_count(), 1);
    assert_eq!(engine.staged_bytes(), 0);

    let gap = chunk_payload(
        &fixture,
        &fixture.canonical_bytes,
        1,
        &fixture.canonical_digest,
    );
    let gap_response = send(&mut engine, "exact_restore_chunk_request", gap)?;
    assert_eq!(gap_response["payload"]["error_code"], "chunk_conflict");
    assert_eq!(engine.staged_bytes(), 0);

    let canonical_chunk = chunk_payload(
        &fixture,
        &fixture.canonical_bytes,
        0,
        &fixture.canonical_digest,
    );
    let accepted_chunk = send(
        &mut engine,
        "exact_restore_chunk_request",
        canonical_chunk.clone(),
    )?;
    assert_eq!(accepted_chunk["payload"]["result"], "CHUNK_ACCEPTED");
    let append_count = shared_store
        .lock()
        .map_err(|_| "store lock poisoned")?
        .append_calls;
    let duplicate_chunk = send(&mut engine, "exact_restore_chunk_request", canonical_chunk)?;
    assert_eq!(duplicate_chunk["payload"]["result"], "CHUNK_ACCEPTED");
    assert_eq!(
        shared_store
            .lock()
            .map_err(|_| "store lock poisoned")?
            .append_calls,
        append_count
    );
    let changed_duplicate = chunk_payload(&fixture, b"changed", 0, &fixture.canonical_digest);
    let conflict = send(
        &mut engine,
        "exact_restore_chunk_request",
        changed_duplicate,
    )?;
    assert_eq!(conflict["payload"]["error_code"], "chunk_conflict");
    let finished_canonical = send(
        &mut engine,
        "exact_restore_finish_blob_request",
        json!({
            "operation_id": fixture.operation_id,
            "expected_owner": fixture.owner,
            "artifact_digest": fixture.canonical_digest,
            "total_bytes": fixture.canonical_bytes.len()
        }),
    )?;
    assert_eq!(finished_canonical["payload"]["result"], "BLOB_VERIFIED");

    send_blob(
        &mut engine,
        &fixture,
        &fixture.manifest_bytes,
        &fixture.manifest_digest,
    )?;
    let accepted = send_blob(
        &mut engine,
        &fixture,
        &fixture.restore_bytes,
        &fixture.restore_digest,
    )?;
    assert_eq!(accepted["payload"]["state"], "CLOSURE_VERIFIED");
    assert_eq!(engine.staged_bytes(), fixture.aggregate_closure_bytes);

    let lookup = send(
        &mut engine,
        "exact_restore_lookup_request",
        json!({
            "operation_id": fixture.operation_id,
            "expected_owner": fixture.owner,
            "artifact_digest": fixture.canonical_digest
        }),
    )?;
    assert_eq!(lookup["payload"]["result"], "CLOSURE_VERIFIED");
    assert_eq!(lookup["payload"]["verified"], true);
    assert_eq!(
        lookup["payload"]["next_offset"],
        fixture.canonical_bytes.len() as u64
    );

    let commit_payload = fixture.commit_payload();
    let committed = send(
        &mut engine,
        "exact_restore_commit_request",
        commit_payload.clone(),
    )?;
    assert_eq!(committed["payload"]["result"], "RESTORE_VERIFIED");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let receipt = committed["payload"]["receipt"].clone();
    assert_eq!(
        receipt["recaptured_exact_state_digest"],
        fixture.exact_state_digest
    );
    let mut receipt_without_digest = receipt.clone();
    receipt_without_digest
        .as_object_mut()
        .ok_or("receipt must be object")?
        .remove("receipt_digest");
    assert_eq!(
        receipt["receipt_digest"],
        sha256(&serde_json::to_vec(&receipt_without_digest)?)
    );
    let duplicate_commit = send(&mut engine, "exact_restore_commit_request", commit_payload)?;
    assert_eq!(duplicate_commit["payload"]["result"], "RESTORE_VERIFIED");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    Ok(())
}

#[test]
fn aliased_references_count_once_in_closure_bytes_but_keep_both_references()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(true)?;
    assert_eq!(fixture.begin_payload["artifact_reference_count"], 2);
    assert_eq!(fixture.begin_payload["distinct_blob_count"], 1);
    let shared_store = Arc::new(Mutex::new(StoreState::default()));
    let owner = Arc::new(Mutex::new(current_owner(&fixture.owner)));
    let mut engine = create_engine(
        shared_store,
        owner,
        RecordingApplier::unknown(Arc::new(AtomicUsize::new(0))),
    )?;
    let begun = send(
        &mut engine,
        "exact_restore_begin_request",
        fixture.begin_payload.clone(),
    )?;
    assert_eq!(begun["payload"]["state"], "STAGING", "{begun:?}");
    send_blob(
        &mut engine,
        &fixture,
        &fixture.manifest_bytes,
        &fixture.manifest_digest,
    )?;
    let finished = send_blob(
        &mut engine,
        &fixture,
        &fixture.canonical_bytes,
        &fixture.canonical_digest,
    )?;
    assert_eq!(finished["payload"]["state"], "CLOSURE_VERIFIED");
    assert_eq!(engine.staged_bytes(), fixture.aggregate_closure_bytes);
    Ok(())
}

#[test]
fn every_phase_rechecks_the_independent_live_owner_fence() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = Fixture::new(false)?;
    let shared_store = Arc::new(Mutex::new(StoreState::default()));
    let live_owner = Arc::new(Mutex::new(current_owner(&fixture.owner)));
    let mut engine = create_engine(
        shared_store,
        live_owner.clone(),
        RecordingApplier::unknown(Arc::new(AtomicUsize::new(0))),
    )?;
    send(
        &mut engine,
        "exact_restore_begin_request",
        fixture.begin_payload.clone(),
    )?;
    *live_owner.lock().map_err(|_| "owner lock poisoned")? =
        current_owner(&owner_with_generation(&fixture.owner, 4)?);
    let chunk = chunk_payload(
        &fixture,
        &fixture.canonical_bytes,
        0,
        &fixture.canonical_digest,
    );
    let response = send(&mut engine, "exact_restore_chunk_request", chunk)?;
    assert_eq!(response["payload"]["outcome"], "STALE_OWNER");
    assert_eq!(response["payload"]["host_effect"], "not_started");
    assert_eq!(engine.staged_bytes(), 0);
    Ok(())
}

#[test]
fn changed_begin_identity_and_noncanonical_base64_are_rejected()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(false)?;
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
    let mut changed = fixture.begin_payload.clone();
    changed["branch"]["metadata_revision"] = json!(2);
    let conflict = send(&mut engine, "exact_restore_begin_request", changed)?;
    assert_eq!(conflict["payload"]["error_code"], "operation_conflict");

    let mut invalid = chunk_payload(&fixture, b"f", 0, &fixture.canonical_digest);
    invalid["total_bytes"] = json!(fixture.canonical_bytes.len());
    invalid["chunk_digest"] = json!(sts2_game_mod::blob_digest(b"f"));
    invalid["data_base64"] = json!("Zh==");
    let bad_base64 = send(&mut engine, "exact_restore_chunk_request", invalid)?;
    assert_eq!(bad_base64["payload"]["error_code"], "invalid_frame");
    let oversize = chunk_payload(&fixture, &vec![0_u8; 8_193], 0, &fixture.canonical_digest);
    let oversize_response = send(&mut engine, "exact_restore_chunk_request", oversize)?;
    assert_eq!(oversize_response["payload"]["error_code"], "invalid_frame");
    assert_eq!(engine.staged_bytes(), 0);
    Ok(())
}
