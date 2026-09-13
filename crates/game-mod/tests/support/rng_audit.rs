// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]
#![allow(dead_code)]

use sts2_game_mod::{
    ExternalInputControl, ExternalInputDeclaration, ExternalInputEvidence, ExternalInputKind,
    GameplayImpact, RngAuditBinding, RngAuditWitness, RngCoverageStatus, RngCursorEvidence,
    RngSeedOrigin, RngSerialization, RngStreamCategory, RngStreamEvidence,
};

pub fn binding() -> RngAuditBinding {
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

pub fn gameplay_stream() -> RngStreamEvidence {
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

pub fn cosmetic_stream() -> RngStreamEvidence {
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

pub fn external_inputs() -> Vec<ExternalInputEvidence> {
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

pub fn controlled_wall_clock() -> ExternalInputEvidence {
    ExternalInputEvidence {
        kind: ExternalInputKind::WallClock,
        owner: "synthetic::Clock".to_owned(),
        gameplay: GameplayImpact::AffectsGameplay,
        control: ExternalInputControl::Controlled,
        evidence: "fixture binds the clock value".to_owned(),
    }
}

pub fn independent_gameplay_stream() -> RngStreamEvidence {
    let mut stream = gameplay_stream();
    stream.seed_origin = RngSeedOrigin::Independent {
        source: "wall_clock".to_owned(),
    };
    stream
}

pub fn witness() -> RngAuditWitness {
    RngAuditWitness::new(
        binding(),
        RngCoverageStatus::Complete,
        vec![cosmetic_stream(), gameplay_stream()],
        external_inputs(),
    )
    .expect("base fixture should validate")
}
