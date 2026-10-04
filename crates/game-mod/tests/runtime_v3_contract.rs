// SPDX-License-Identifier: MIT

use serde_json::{Value, json};
use std::{error::Error, path::Path, process::Command};
use sts2_game_mod::{
    RUNTIME_V3_GAMEPLAY_ARTIFACT, RUNTIME_V3_GAMEPLAY_GENERATOR, RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST,
    RUNTIME_V3_GAMEPLAY_SCHEMA_SOURCE, RuntimeV3GameplayAction, RuntimeV3GameplayEnemyIntent,
    RuntimeV3GameplayMessage, RuntimeV3GameplayPotionTargetMode, RuntimeV3GameplayState,
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
const DESCRIBED_BELT: &str = include_str!(
    "../../../protocol-artifact/runtime-v3-gameplay/golden/state-response-described-belt.json"
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
    // Counted from the manifest rather than hardcoded, so adding a golden does not require editing
    // this assertion in every consumer that mirrors the artifact.
    let manifest: Value = serde_json::from_str(include_str!(
        "../../../protocol-artifact/runtime-v3-gameplay/manifest.json"
    ))?;
    let listed_goldens = manifest["goldens"].as_array().map(Vec::len).unwrap_or(0);
    // Plus the case file, the normative schema, the manifest, and the artifact schema.
    assert_eq!(
        String::from_utf8(output.stdout)?.lines().count(),
        listed_goldens + 4
    );
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
        "0ae1d4d1525162da3059c028dcdb70df1d4d2dcf9620c5edd9b543e5f04aacc2"
    );
    for golden in [
        REQUEST,
        STATE,
        DISPATCH,
        SETTLED,
        include_str!(
            "../../../protocol-artifact/runtime-v3-gameplay/golden/dispatch-proceed-request.json"
        ),
        include_str!(
            "../../../protocol-artifact/runtime-v3-gameplay/golden/dispatch-confirm-selection-request.json"
        ),
        include_str!(
            "../../../protocol-artifact/runtime-v3-gameplay/golden/dispatch-cancel-selection-request.json"
        ),
        include_str!(
            "../../../protocol-artifact/runtime-v3-gameplay/golden/state-response-described-offer.json"
        ),
        DESCRIBED_BELT,
    ] {
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
fn described_belt_golden_round_trips_through_the_shipped_contract() -> Result<(), Box<dyn Error>> {
    let message: RuntimeV3GameplayMessage = serde_json::from_str(DESCRIBED_BELT)?;
    message.validate()?;
    let observation = message.observation.as_ref().ok_or("missing observation")?;
    let player = &observation.player;
    assert_eq!(player.hand[0].description.as_deref(), Some("Gain 8 Block."));
    let relics = player.relics.as_ref().ok_or("missing observed relics")?;
    assert_eq!(
        relics[0].description.as_deref(),
        Some("Heal 2 HP at the end of each turn.")
    );
    assert_eq!(relics[1].description, None);
    let potions = player.potions.as_ref().ok_or("missing observed potions")?;
    assert_eq!(
        potions[0].target_mode,
        Some(RuntimeV3GameplayPotionTargetMode::AnyEnemy)
    );
    assert_eq!(
        potions[2].target_mode,
        Some(RuntimeV3GameplayPotionTargetMode::Unknown)
    );
    assert_eq!(player.potion_slots, Some(3));
    assert_eq!(player.max_potion_slots, Some(3));
    let actions = message
        .legal_actions
        .as_ref()
        .ok_or("missing legal actions")?;
    assert!(matches!(
        actions[1].action,
        RuntimeV3GameplayAction::UsePotion { ref potion_id, target_id: Some(ref target_id) }
            if potion_id == "potion:1:Fire" && target_id == "enemy-1"
    ));
    assert!(matches!(
        actions[2].action,
        RuntimeV3GameplayAction::DiscardPotion { ref potion_id }
            if potion_id == "potion:3:Unknown"
    ));
    assert_eq!(serde_json::to_string(&message)?, DESCRIBED_BELT.trim());
    Ok(())
}

#[test]
fn optional_player_attributes_distinguish_absent_from_empty_and_reject_null()
-> Result<(), Box<dyn Error>> {
    let mut message: Value = serde_json::from_str(STATE)?;
    {
        let player = message["observation"]["player"]
            .as_object()
            .ok_or("expected player")?;
        assert!(!player.contains_key("relics"));
        assert!(!player.contains_key("potions"));
    }
    let absent: RuntimeV3GameplayMessage = serde_json::from_value(message.clone())?;
    let absent_player = &absent
        .observation
        .as_ref()
        .ok_or("missing observation")?
        .player;
    assert!(absent_player.relics.is_none());
    assert!(absent_player.potions.is_none());
    assert!(
        serde_json::to_value(&absent)?["observation"]["player"]
            .get("relics")
            .is_none()
    );

    {
        let player = message["observation"]["player"]
            .as_object_mut()
            .ok_or("expected player")?;
        player.insert("relics".to_owned(), json!([]));
        player.insert("potions".to_owned(), json!([]));
        player.insert("potion_slots".to_owned(), json!(0));
    }
    let empty: RuntimeV3GameplayMessage = serde_json::from_value(message.clone())?;
    let empty_player = &empty
        .observation
        .as_ref()
        .ok_or("missing observation")?
        .player;
    assert_eq!(empty_player.relics.as_ref().map(Vec::len), Some(0));
    assert_eq!(empty_player.potions.as_ref().map(Vec::len), Some(0));
    assert_eq!(empty_player.potion_slots, Some(0));
    let encoded_empty = serde_json::to_value(&empty)?;
    assert_eq!(encoded_empty["observation"]["player"]["relics"], json!([]));

    for field in ["relics", "potions", "potion_slots", "max_potion_slots"] {
        let mut invalid = message.clone();
        invalid["observation"]["player"][field] = Value::Null;
        assert!(
            serde_json::from_value::<RuntimeV3GameplayMessage>(invalid).is_err(),
            "{field}"
        );
    }
    let mut invalid_card: Value = serde_json::from_str(DESCRIBED_BELT)?;
    invalid_card["observation"]["player"]["hand"][0]["description"] = Value::Null;
    assert!(serde_json::from_value::<RuntimeV3GameplayMessage>(invalid_card).is_err());
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
    text["observation"]["visible_seed"] = json!("é".repeat(513));
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
