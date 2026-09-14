// SPDX-License-Identifier: MIT

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::context::{CanonicalContext, Compatibility, IdentityDigest, ProfileBaseline};
use crate::request::{StartRequestParams, build_start_request};

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn sample_context() -> CanonicalContext {
    CanonicalContext {
        context_id: "ctx-1".to_owned(),
        game_mode: "standard".to_owned(),
        character: "ironclad".to_owned(),
        ascension: 0,
        modifiers: Vec::new(),
        acts: vec!["act-1".to_owned()],
        selection_policy: "standard_default".to_owned(),
        profile_baseline: ProfileBaseline {
            kind: "fresh".to_owned(),
            identity: "prof-1".to_owned(),
            digest: digest('a'),
        },
        save_policy: "enabled".to_owned(),
        compatibility: Compatibility {
            game: IdentityDigest {
                identity: "sts2/1.0.0.0".to_owned(),
                digest: digest('b'),
            },
            mod_identity: IdentityDigest {
                identity: "AIAscensionSTS2GameMod/1.0.0.0".to_owned(),
                digest: digest('c'),
            },
        },
    }
}

fn sample_params() -> StartRequestParams {
    StartRequestParams {
        schema_digest: digest('d'),
        correlation_id: "corr-1".to_owned(),
        instance_id: "inst-1".to_owned(),
        session_id: "sess-1".to_owned(),
        lease_id: "lease-1".to_owned(),
        lease_epoch: 1,
        generation: 0,
        operation_id: "op-1".to_owned(),
        requested_seed: "seed-1".to_owned(),
        run_mode: "seeded_training".to_owned(),
        context: sample_context(),
    }
}

#[test]
fn canonical_digest_matches_contract_vector() {
    assert_eq!(
        sample_context().digest().unwrap(),
        "f9cc2c6c60624cb1da738999b3851dcd1bc1fdd15881bcaf8306171a9b4c660b"
    );
}

#[test]
fn request_carries_digest_and_required_fields() {
    let request = build_start_request(&sample_params()).unwrap();
    let json = request.to_json().unwrap();
    for key in [
        "protocol_version",
        "schema_digest",
        "provenance",
        "correlation_id",
        "instance_id",
        "session_id",
        "lease_id",
        "lease_epoch",
        "generation",
        "kind",
        "operation_id",
        "requested_seed",
        "run_mode",
        "context_digest",
        "selected_context",
        "status",
        "canonical_seed",
        "observation",
        "effect_witness",
        "error_code",
    ] {
        assert!(json.contains(&format!("\"{key}\"")), "missing {key}");
    }
    assert!(json.contains("\"status\":null"));
    assert!(json.contains("\"kind\":\"start_request\""));
    assert!(json.contains(&format!(
        "\"context_digest\":\"{}\"",
        request.context_digest
    )));
}

#[test]
fn validation_rejects_out_of_contract_inputs() {
    let mut context = sample_context();
    context.ascension = 21;
    assert!(context.validate().is_err());

    let mut context = sample_context();
    context.modifiers = vec!["b".to_owned(), "a".to_owned()];
    assert!(context.validate().is_err());

    let mut context = sample_context();
    context.game_mode = "daily".to_owned();
    assert!(context.validate().is_err());
}

#[test]
fn request_rejects_invalid_seed_and_identity() {
    let mut params = sample_params();
    params.requested_seed = String::new();
    assert!(build_start_request(&params).is_err());

    let mut params = sample_params();
    params.operation_id = "bad id".to_owned();
    assert!(build_start_request(&params).is_err());

    let mut params = sample_params();
    params.schema_digest = "nothex".to_owned();
    assert!(build_start_request(&params).is_err());
}
