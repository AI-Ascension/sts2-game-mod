// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

#[path = "support/rng_audit.rs"]
mod fixture;

use fixture::{binding, controlled_wall_clock, gameplay_stream, independent_gameplay_stream};
use sts2_game_mod::{
    ExternalInputControl, ExternalInputDeclaration, RngAuditError, RngAuditWitness,
    RngCoverageStatus, RngCursorEvidence, RngSeedOrigin,
};

#[test]
fn debug_redacts_private_nested_evidence() {
    const SENTINEL: &str = "PRIVATE_SENTINEL";

    let mut hidden_binding = binding();
    hidden_binding.game_build = SENTINEL.to_owned();
    hidden_binding.adapter_compatibility = SENTINEL.to_owned();
    hidden_binding.supported_mode = SENTINEL.to_owned();
    hidden_binding.profile_compatibility = SENTINEL.to_owned();
    hidden_binding.content_manifest = Some(SENTINEL.to_owned());
    hidden_binding.canonical_seed = SENTINEL.to_owned();
    hidden_binding.seed_derivation_version = SENTINEL.to_owned();
    hidden_binding.seeded_boundary = SENTINEL.to_owned();

    let mut hidden_stream = gameplay_stream();
    hidden_stream.stream_id = SENTINEL.to_owned();
    hidden_stream.owner = SENTINEL.to_owned();
    hidden_stream.algorithm_version = Some(SENTINEL.to_owned());
    hidden_stream.seed_origin = RngSeedOrigin::MasterDerived {
        derivation_version: SENTINEL.to_owned(),
    };
    hidden_stream.creation_boundary = SENTINEL.to_owned();
    hidden_stream.reset_boundary = SENTINEL.to_owned();
    hidden_stream.call_categories = vec![SENTINEL.to_owned()];
    hidden_stream.evidence = SENTINEL.to_owned();

    let mut hidden_input = controlled_wall_clock();
    hidden_input.owner = SENTINEL.to_owned();
    hidden_input.evidence = SENTINEL.to_owned();
    let hidden_witness = RngAuditWitness::new(
        hidden_binding.clone(),
        RngCoverageStatus::Complete,
        vec![hidden_stream.clone()],
        vec![hidden_input.clone()],
    )
    .expect("sentinel witness should validate");

    let rendered = [
        format!("{:?}", hidden_binding),
        format!("{:?}", hidden_stream),
        format!("{:?}", hidden_input),
        format!("{:?}", hidden_witness),
        format!(
            "{:?}",
            RngCursorEvidence::Known {
                state_digest: SENTINEL.to_owned(),
                cursor: 9_876,
            }
        ),
        format!(
            "{:?}",
            RngSeedOrigin::Independent {
                source: SENTINEL.to_owned(),
            }
        ),
    ]
    .join("\n");

    assert!(
        !rendered.contains(SENTINEL),
        "private audit evidence leaked through Debug: {rendered}"
    );
}

#[test]
fn independent_gameplay_seed_requires_audited_controlled_input() {
    let mut none_observed = binding();
    none_observed.external_input_declaration = ExternalInputDeclaration::NoneObserved;
    assert_eq!(
        RngAuditWitness::new(
            none_observed,
            RngCoverageStatus::Complete,
            vec![independent_gameplay_stream()],
            vec![],
        ),
        Err(RngAuditError::IndependentSeedNotLinked {
            stream_id: "encounter/main".to_owned(),
            source: "wall_clock".to_owned(),
        })
    );

    let mut uncontrolled = controlled_wall_clock();
    uncontrolled.control = ExternalInputControl::Unknown;
    assert_eq!(
        RngAuditWitness::new(
            binding(),
            RngCoverageStatus::Complete,
            vec![independent_gameplay_stream()],
            vec![uncontrolled],
        ),
        Err(RngAuditError::UncontrolledExternalInput {
            kind: sts2_game_mod::ExternalInputKind::WallClock,
        })
    );

    assert!(
        RngAuditWitness::new(
            binding(),
            RngCoverageStatus::Complete,
            vec![independent_gameplay_stream()],
            vec![controlled_wall_clock()],
        )
        .is_ok(),
        "a controlled wall-clock binding should satisfy the independent seed link"
    );
}
