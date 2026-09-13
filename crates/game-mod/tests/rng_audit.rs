// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

use std::cell::Cell;

use serde_json::Value;
use sts2_game_mod::{
    ExternalInputControl, ExternalInputDeclaration, ExternalInputEvidence, ExternalInputKind,
    GameplayImpact, RngAuditBinding, RngAuditError, RngAuditPort, RngAuditReadError,
    RngAuditUnavailableReason, RngAuditWitness, RngCoverageStatus, RngCursorEvidence,
    RngSeedOrigin, RngSerialization, RngStateAvailability, RngStreamCategory, RngStreamEvidence,
    UnavailableRngAudit,
};

const FIXTURE: &str = include_str!("fixtures/rng-audit-v1.json");

fn binding() -> RngAuditBinding {
    RngAuditBinding {
        game_build: "synthetic-build:0.1".to_owned(),
        adapter_compatibility: "adapter-v1".to_owned(),
        supported_mode: "practice".to_owned(),
        profile_compatibility: "profile-v1".to_owned(),
        content_manifest: Some("manifest:synthetic".to_owned()),
        canonical_seed: "ALPHA-123".to_owned(),
        seed_derivation_version: "host-seed-v1".to_owned(),
        seeded_boundary: "run_initialized".to_owned(),
        external_input_declaration: ExternalInputDeclaration::Enumerated,
    }
}

fn gameplay_stream() -> RngStreamEvidence {
    RngStreamEvidence {
        stream_id: "encounter/main".to_owned(),
        category: RngStreamCategory::EncounterEnemy,
        owner: "synthetic::EncounterRng".to_owned(),
        algorithm_version: Some("xorshift-v1".to_owned()),
        seed_origin: RngSeedOrigin::MasterDerived {
            derivation_version: "host-seed-v1".to_owned(),
        },
        initial_state: RngCursorEvidence::Known {
            state_digest: "sha256:0000000000000000000000000000000000000000000000000000000000000001"
                .to_owned(),
            cursor: 0,
        },
        creation_boundary: "run_initialized".to_owned(),
        reset_boundary: "act_started".to_owned(),
        call_categories: vec!["enemy_roll".to_owned(), "encounter_pick".to_owned()],
        serialization: RngSerialization::Available,
        gameplay: GameplayImpact::AffectsGameplay,
        evidence: "synthetic source fixture".to_owned(),
    }
}

fn cosmetic_stream() -> RngStreamEvidence {
    RngStreamEvidence {
        stream_id: "cosmetic/particles".to_owned(),
        category: RngStreamCategory::OtherGameplay,
        owner: "synthetic::ParticleRng".to_owned(),
        algorithm_version: Some("visual-v1".to_owned()),
        seed_origin: RngSeedOrigin::Independent {
            source: "visual_seed".to_owned(),
        },
        initial_state: RngCursorEvidence::Unavailable,
        creation_boundary: "run_initialized".to_owned(),
        reset_boundary: "scene_loaded".to_owned(),
        call_categories: vec!["particle_jitter".to_owned()],
        serialization: RngSerialization::Unsupported,
        gameplay: GameplayImpact::CosmeticOnly,
        evidence: "fixture proves no game-state or action-order reads".to_owned(),
    }
}

fn external_inputs() -> Vec<ExternalInputEvidence> {
    vec![
        ExternalInputEvidence {
            kind: ExternalInputKind::Locale,
            owner: "synthetic::LocaleProvider".to_owned(),
            gameplay: GameplayImpact::AffectsGameplay,
            control: ExternalInputControl::Controlled,
            evidence: "fixed locale is bound by the fixture".to_owned(),
        },
        ExternalInputEvidence {
            kind: ExternalInputKind::FrameTiming,
            owner: "synthetic::Renderer".to_owned(),
            gameplay: GameplayImpact::CosmeticOnly,
            control: ExternalInputControl::Unknown,
            evidence: "renderer-only timing does not enter game state".to_owned(),
        },
    ]
}

fn witness() -> RngAuditWitness {
    RngAuditWitness::new(
        binding(),
        RngCoverageStatus::Complete,
        vec![cosmetic_stream(), gameplay_stream()],
        external_inputs(),
    )
    .expect("base fixture should validate")
}

#[test]
fn fixture_binds_source_only_schema_and_negative_cases() {
    let fixture: Value = serde_json::from_str(FIXTURE).expect("fixture JSON");
    assert_eq!(fixture["schema"], "ascension.rng_audit.v1");
    assert_eq!(fixture["profile"], "asc-rng-audit-private-v1");
    assert_eq!(fixture["evidence"], "synthetic-source-only");
    assert_eq!(fixture["binding"]["game_build"], "synthetic-build:0.1");
    assert_eq!(
        fixture["binding"]["external_input_declaration"],
        "enumerated"
    );
    assert_eq!(
        witness().fingerprint(),
        fixture["expected_fingerprint"]
            .as_str()
            .expect("fingerprint")
    );
    assert_eq!(
        fixture["negative_cases"].as_array().expect("cases").len(),
        8
    );
}

#[test]
fn witness_is_deterministic_and_orders_inventory() {
    let first = witness();
    let mut reversed_streams = vec![gameplay_stream(), cosmetic_stream()];
    reversed_streams.reverse();
    let reversed = RngAuditWitness::new(
        binding(),
        RngCoverageStatus::Complete,
        reversed_streams,
        external_inputs().into_iter().rev().collect(),
    )
    .expect("reordered fixture should validate");

    assert_eq!(first.fingerprint(), reversed.fingerprint());
    assert_eq!(
        first
            .projection()
            .streams
            .iter()
            .map(|stream| stream.stream_id.as_str())
            .collect::<Vec<_>>(),
        vec!["cosmetic/particles", "encounter/main"]
    );
    assert_eq!(
        first
            .projection()
            .external_inputs
            .iter()
            .map(|input| input.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["frame_timing", "locale"]
    );
}

#[test]
fn private_cursor_changes_are_detectable_without_changing_projection() {
    let first = witness();
    let mut changed_stream = gameplay_stream();
    changed_stream.initial_state = RngCursorEvidence::Known {
        state_digest: "sha256:0000000000000000000000000000000000000000000000000000000000000001"
            .to_owned(),
        cursor: 7,
    };
    let changed = RngAuditWitness::new(
        binding(),
        RngCoverageStatus::Complete,
        vec![cosmetic_stream(), changed_stream],
        external_inputs(),
    )
    .expect("changed cursor fixture should validate");

    assert_ne!(first.fingerprint(), changed.fingerprint());
    assert_eq!(first.projection(), changed.projection());
}

#[test]
fn projection_contains_no_private_state_or_evidence() {
    let projection = witness().projection();
    assert_eq!(
        projection.streams[0].state,
        RngStateAvailability::Unavailable
    );
    assert_eq!(projection.streams[1].state, RngStateAvailability::Known);
    let encoded =
        String::from_utf8(projection.to_json_bytes().expect("projection JSON")).expect("UTF-8");
    assert!(!encoded.contains("cursor"));
    assert!(!encoded.contains("state_digest"));
    assert!(!encoded.contains("independent_source"));
    assert!(!encoded.contains("synthetic source fixture"));
}

#[test]
fn known_zero_state_is_distinct_from_unavailable_state() {
    let mut zero_stream = gameplay_stream();
    zero_stream.initial_state = RngCursorEvidence::KnownZero { cursor: 0 };
    let zero = RngAuditWitness::new(
        binding(),
        RngCoverageStatus::Complete,
        vec![zero_stream],
        external_inputs(),
    )
    .expect("known-zero fixture should validate");
    assert_eq!(
        zero.projection().streams[0].state,
        RngStateAvailability::KnownZero
    );
    assert_eq!(
        witness().projection().streams[0].state,
        RngStateAvailability::Unavailable
    );
}

#[test]
fn repeated_port_reads_are_read_only_and_stable() {
    #[derive(Debug)]
    struct StaticPort {
        witness: RngAuditWitness,
        reads: Cell<usize>,
        draws: Cell<usize>,
    }

    impl RngAuditPort for StaticPort {
        fn read_initialization_witness(&self) -> Result<RngAuditWitness, RngAuditReadError> {
            self.reads.set(self.reads.get() + 1);
            Ok(self.witness.clone())
        }
    }

    let port = StaticPort {
        witness: witness(),
        reads: Cell::new(0),
        draws: Cell::new(0),
    };
    let first = port
        .read_initialization_witness()
        .expect("first read should validate");
    let second = port
        .read_initialization_witness()
        .expect("second read should validate");
    assert_eq!(port.reads.get(), 2);
    assert_eq!(port.draws.get(), 0);
    assert_eq!(first, second);
    assert_eq!(first.fingerprint(), second.fingerprint());
}

#[test]
fn fail_closed_validation_rejects_missing_binding_coverage_and_state() {
    let mut missing_build = binding();
    missing_build.game_build.clear();
    assert_eq!(
        RngAuditWitness::new(
            missing_build,
            RngCoverageStatus::Complete,
            vec![gameplay_stream()],
            vec![],
        ),
        Err(RngAuditError::MissingBuildBinding)
    );

    assert_eq!(
        RngAuditWitness::new(
            binding(),
            RngCoverageStatus::Unknown,
            vec![gameplay_stream()],
            vec![],
        ),
        Err(RngAuditError::IncompleteCoverage {
            status: RngCoverageStatus::Unknown
        })
    );

    let mut missing_state = gameplay_stream();
    missing_state.initial_state = RngCursorEvidence::Unavailable;
    assert_eq!(
        RngAuditWitness::new(
            binding(),
            RngCoverageStatus::Complete,
            vec![missing_state],
            vec![],
        ),
        Err(RngAuditError::MissingStateEvidence {
            stream_id: "encounter/main".to_owned()
        })
    );
}

#[test]
fn fail_closed_validation_rejects_duplicate_and_malformed_evidence() {
    assert_eq!(
        RngAuditWitness::new(
            binding(),
            RngCoverageStatus::Complete,
            vec![gameplay_stream(), gameplay_stream()],
            vec![],
        ),
        Err(RngAuditError::DuplicateStreamIdentity {
            stream_id: "encounter/main".to_owned()
        })
    );

    let mut malformed = gameplay_stream();
    malformed.initial_state = RngCursorEvidence::Known {
        state_digest: "not-a-digest".to_owned(),
        cursor: 0,
    };
    assert_eq!(
        RngAuditWitness::new(
            binding(),
            RngCoverageStatus::Complete,
            vec![malformed],
            vec![],
        ),
        Err(RngAuditError::MalformedState {
            stream_id: "encounter/main".to_owned()
        })
    );

    let mut unknown_external = external_inputs();
    unknown_external[0].control = ExternalInputControl::Unknown;
    assert_eq!(
        RngAuditWitness::new(
            binding(),
            RngCoverageStatus::Complete,
            vec![gameplay_stream()],
            unknown_external,
        ),
        Err(RngAuditError::UncontrolledExternalInput {
            kind: ExternalInputKind::Locale
        })
    );
}

#[test]
fn over_limit_inventory_is_rejected() {
    let mut streams = Vec::new();
    for index in 0..=sts2_game_mod::RNG_AUDIT_MAX_STREAMS {
        let mut stream = gameplay_stream();
        stream.stream_id = format!("stream/{index:02}");
        streams.push(stream);
    }
    assert_eq!(
        RngAuditWitness::new(binding(), RngCoverageStatus::Complete, streams, vec![],),
        Err(RngAuditError::TooManyStreams {
            limit: sts2_game_mod::RNG_AUDIT_MAX_STREAMS
        })
    );
}

#[test]
fn over_limit_private_witness_is_rejected() {
    let mut streams = Vec::new();
    for index in 0..sts2_game_mod::RNG_AUDIT_MAX_STREAMS {
        let mut stream = gameplay_stream();
        stream.stream_id = format!("stream/{index:02}");
        stream.evidence = "e".repeat(sts2_game_mod::RNG_AUDIT_MAX_TEXT_BYTES);
        stream.call_categories = (0..sts2_game_mod::RNG_AUDIT_MAX_CALL_CATEGORIES)
            .map(|category| format!("call-{category}-{}", "c".repeat(240)))
            .collect();
        streams.push(stream);
    }
    let mut large_binding = binding();
    large_binding.external_input_declaration = ExternalInputDeclaration::NoneObserved;
    let result = RngAuditWitness::new(large_binding, RngCoverageStatus::Complete, streams, vec![]);
    assert!(matches!(result, Err(RngAuditError::WitnessTooLarge { .. })));
}

#[test]
fn unavailable_port_does_not_claim_exact_host_support() {
    assert_eq!(
        UnavailableRngAudit
            .read_initialization_witness()
            .expect_err("exact-host evidence is not present"),
        RngAuditReadError::Unavailable(RngAuditUnavailableReason::ExactHostEvidenceRequired)
    );
}
