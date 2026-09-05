// SPDX-License-Identifier: MIT

use serde_json::{Value, json};
use std::{error::Error, path::Path, process::Command};
use sts2_game_mod::{
    RUNTIME_V3_GAMEPLAY_ARTIFACT, RUNTIME_V3_GAMEPLAY_GENERATOR, RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST,
    RUNTIME_V3_GAMEPLAY_SCHEMA_SOURCE, RuntimeV3GameplayAction, RuntimeV3GameplayEnemyIntent,
    RuntimeV3GameplayMessage, RuntimeV3GameplayState,
};

const STATE: &str =
    include_str!("../../../protocol-artifact/runtime-v3-gameplay/golden/state-response.json");
const REQUEST: &str =
    include_str!("../../../protocol-artifact/runtime-v3-gameplay/golden/state-request.json");
const DISPATCH: &str = include_str!(
    "../../../protocol-artifact/runtime-v3-gameplay/golden/dispatch-action-request.json"
);
const SETTLED: &str = include_str!(
    "../../../protocol-artifact/runtime-v3-gameplay/golden/dispatch-action-settled.json"
);

#[test]
fn canonical_artifact_bytes_and_provenance_match() -> Result<(), Box<dyn Error>> {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../protocol-artifact/runtime-v3-gameplay");
    let output = Command::new("sha256sum")
        .args(["--check", "--strict", "SHA256SUMS"])
        .current_dir(&root)
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(String::from_utf8(output.stdout)?.lines().count(), 8);
    let manifest: Value = serde_json::from_str(include_str!(
        "../../../protocol-artifact/runtime-v3-gameplay/manifest.json"
    ))?;
    assert_eq!(manifest["schema_digest"], RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST);
    assert_eq!(manifest["artifact"], RUNTIME_V3_GAMEPLAY_ARTIFACT);
    assert_eq!(
        manifest["provenance"]["source"],
        RUNTIME_V3_GAMEPLAY_SCHEMA_SOURCE
    );
    assert_eq!(
        manifest["provenance"]["generator"],
        RUNTIME_V3_GAMEPLAY_GENERATOR
    );
    // Pin the authoritative producer, not merely a consumer's self-consistent manifest.
    assert_eq!(
        RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST,
        "b37c80f583aeaf4f81ede2083bcfb4129196baf5eb092470e8738173c4b7226c"
    );
    for golden in [REQUEST, STATE, DISPATCH, SETTLED] {
        let message: RuntimeV3GameplayMessage = serde_json::from_str(golden)?;
        message.validate()?;
        assert_eq!(
            serde_json::to_value(message)?,
            serde_json::from_str::<Value>(golden)?
        );
    }
    Ok(())
}

#[test]
fn every_nullable_envelope_field_is_required() -> Result<(), Box<dyn Error>> {
    let request: Value = serde_json::from_str(REQUEST)?;
    for field in [
        "state_id",
        "operation_id",
        "observation",
        "legal_actions",
        "action",
        "status",
        "transition",
        "error_code",
        "wait_for_millis",
        "wait_outcome",
        "recovery",
    ] {
        let mut missing = request.clone();
        missing
            .as_object_mut()
            .ok_or("expected object")?
            .remove(field);
        assert!(
            serde_json::from_value::<RuntimeV3GameplayMessage>(missing).is_err(),
            "{field}"
        );
    }
    let mut state: Value = serde_json::from_str(STATE)?;
    state["observation"]
        .as_object_mut()
        .ok_or("expected observation")?
        .remove("visible_seed");
    assert!(serde_json::from_value::<RuntimeV3GameplayMessage>(state).is_err());
    assert!(
        serde_json::from_value::<RuntimeV3GameplayState>(json!({"state":"map","options":[]}))
            .is_err()
    );
    assert!(serde_json::from_value::<RuntimeV3GameplayState>(json!({"state":"defeat"})).is_err());
    assert!(
        serde_json::from_value::<RuntimeV3GameplayAction>(
            json!({"kind":"play_card","card_id":"c"})
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn empty_and_populated_tagged_variants_are_closed() {
    for kind in [
        "end_turn",
        "skip_reward",
        "rest",
        "confirm_victory",
        "save_quit",
    ] {
        assert!(serde_json::from_value::<RuntimeV3GameplayAction>(json!({"kind":kind})).is_ok());
        assert!(
            serde_json::from_value::<RuntimeV3GameplayAction>(json!({"kind":kind,"hidden":1}))
                .is_err()
        );
    }
    for kind in ["defend", "buff", "debuff", "unknown"] {
        assert!(
            serde_json::from_value::<RuntimeV3GameplayEnemyIntent>(json!({"kind":kind,"hidden":1}))
                .is_err()
        );
    }
    assert!(
        serde_json::from_value::<RuntimeV3GameplayState>(json!({"state":"victory","hidden":1}))
            .is_err()
    );
    assert!(
        serde_json::from_value::<RuntimeV3GameplayAction>(
            json!({"kind":"play_card","card_id":"c","target_id":null,"hidden":1})
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<RuntimeV3GameplayAction>(r#"{"kind":"end_turn","kind":"rest"}"#)
            .is_err()
    );
}

#[test]
fn semantic_checks_reject_structurally_valid_contradictions() -> Result<(), Box<dyn Error>> {
    let state: Value = serde_json::from_str(STATE)?;
    let mut cases = Vec::new();
    let mut mismatch = state.clone();
    mismatch["generation"] = json!(999);
    cases.push(mismatch);
    let mut hp = state.clone();
    hp["observation"]["player"]["hp"] = json!(65535);
    hp["observation"]["player"]["max_hp"] = json!(1);
    cases.push(hp);
    let mut duplicate = state.clone();
    let action = json!({"action_id":"end-turn", "action":{"kind":"end_turn"}});
    duplicate["legal_actions"] = json!([action, action]);
    cases.push(duplicate);
    let mut text = state;
    text["observation"]["visible_seed"] = json!("é".repeat(257));
    cases.push(text);
    let mut transition: Value = serde_json::from_str(SETTLED)?;
    transition["transition"]["from_generation"] = transition["transition"]["to_generation"].clone();
    cases.push(transition);
    let mut old: Value = serde_json::from_str(REQUEST)?;
    old["schema_digest"] =
        json!("fbfb18279b0c7ebb350ef0ce0d56547fa11e83985b13380cb2b0f1dba4cb56e9");
    cases.push(old);
    let mut provenance: Value = serde_json::from_str(REQUEST)?;
    provenance["provenance"]["generator"] = json!("other");
    cases.push(provenance);
    for case in cases {
        assert!(
            serde_json::from_value::<RuntimeV3GameplayMessage>(case)?
                .validate()
                .is_err()
        );
    }
    Ok(())
}
