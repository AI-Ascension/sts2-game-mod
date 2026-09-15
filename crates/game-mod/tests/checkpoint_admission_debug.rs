// SPDX-License-Identifier: MIT

//! Redaction checks for the source-only checkpoint admission owner surfaces.

#![allow(clippy::expect_used)]

use std::error::Error;

use sts2_game_mod::{
    CheckpointBoundary, CheckpointCaptureAdmission, CheckpointCaptureIdentity,
    CheckpointCaptureProducer, CheckpointCaptureRequest, CheckpointSettlement,
    FixtureCheckpointProducer, blob_digest, state_id,
};

const PAYLOAD: &[u8] = b"{\"a\":1,\"b\":2}";
const COMBAT: CheckpointBoundary = CheckpointBoundary::StablePlayerTurnCombat;

fn request(operation_id: &str) -> CheckpointCaptureRequest {
    CheckpointCaptureRequest::new(
        CheckpointCaptureIdentity::new(
            "instance-1",
            "session-1",
            "lease-1",
            7,
            "run-1",
            "profile-1",
            operation_id,
        )
        .expect("identity"),
        COMBAT,
    )
}

#[test]
fn new_owner_debug_output_omits_private_payloads_and_exact_digests() -> Result<(), Box<dyn Error>> {
    let mut producer = FixtureCheckpointProducer::new(vec![PAYLOAD.to_vec()]);
    let produced = producer.produce(&request("op-debug"))?;
    let produced_debug = format!("{produced:?}");
    assert!(produced_debug.contains("canonical_bytes_len"));

    let mut admission = CheckpointCaptureAdmission::new(producer);
    admission.capture(CheckpointSettlement::Quiescent, request("op-debug"))?;
    assert_eq!(admission.ledger().len(), 1);

    let owner_debug = format!(
        "{produced_debug}{:?}{:?}",
        admission.ledger(),
        admission.producer()
    );
    let payload_render = format!("{:?}", PAYLOAD.to_vec());
    for private in [payload_render, state_id(PAYLOAD), blob_digest(PAYLOAD)] {
        assert!(!owner_debug.contains(&private));
    }
    Ok(())
}
