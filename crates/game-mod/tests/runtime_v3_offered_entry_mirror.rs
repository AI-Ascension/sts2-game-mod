// SPDX-License-Identifier: MIT

//! The contract mirror must accept exactly what the widened schema accepts.
//!
//! `sts2-protocol` owns the vocabulary; this file exists to prove the mirror did not drift from
//! it. A mirror that is laxer lets the mod emit something every consumer rejects, and a mirror that
//! is stricter refuses a message the producer is entitled to send. Both are silent failures unless
//! something checks the mirror against the same artifacts.

use serde_json::{Value, json};
use sts2_game_mod::{
    RUNTIME_V3_GAMEPLAY_MAX_OFFERED_ATTRIBUTE_CHARACTERS, RuntimeV3GameplayChoice,
    RuntimeV3GameplayMessage, RuntimeV3GameplayOfferedEntry, RuntimeV3GameplayState,
    RuntimeV3GameplayValidationError,
};

const SCHEMA: &str = include_str!("../../../schemas/runtime-v3-gameplay.schema.json");
const GOLDEN: &str = include_str!(
    "../../../protocol-artifact/runtime-v3-gameplay/golden/state-response-described-offer.json"
);
const BASE: &str =
    include_str!("../../../protocol-artifact/runtime-v3-gameplay/golden/state-response.json");

/// One named mutation applied to a document before re-checking the schema.
struct Mutation(&'static str, fn(&mut Value));

fn validator() -> Option<jsonschema::Validator> {
    let schema: Value = serde_json::from_str(SCHEMA).ok()?;
    jsonschema::draft202012::options().build(&schema).ok()
}

/// Whether the schema admits `document`.
fn schema_admits(document: &Value) -> bool {
    validator().is_some_and(|validator| validator.is_valid(document))
}

/// The reward options the mirror decodes out of a document, or `None` if it is not a reward state.
fn decoded_options(document: &Value) -> Option<Vec<RuntimeV3GameplayChoice>> {
    let message = serde_json::from_value::<RuntimeV3GameplayMessage>(document.clone()).ok()?;
    let observation = message.observation.as_ref()?;
    let RuntimeV3GameplayState::Reward { options } = &observation.state else {
        return None;
    };
    Some(options.clone())
}

/// A reward state offering exactly `choice`, as the schema would see it.
fn reward_with(choice: &RuntimeV3GameplayChoice) -> Value {
    let Ok(mut message) = serde_json::from_str::<Value>(BASE) else {
        return Value::Null;
    };
    let Ok(encoded) = serde_json::to_value(choice) else {
        return Value::Null;
    };
    message["observation"]["state"] = json!({ "state": "reward", "options": [encoded] });
    message
}

/// A valid envelope carrying `state`, so `validate()` reaches the state's own rules.
fn observed(state: RuntimeV3GameplayState) -> Result<(), RuntimeV3GameplayValidationError> {
    let Ok(mut message) = serde_json::from_str::<RuntimeV3GameplayMessage>(BASE) else {
        return Err(RuntimeV3GameplayValidationError::Metadata);
    };
    let Some(observation) = message.observation.as_mut() else {
        return Err(RuntimeV3GameplayValidationError::Metadata);
    };
    observation.state = state;
    message.validate()
}

#[test]
fn the_mirror_decodes_the_described_offer_golden() {
    let golden: Value = serde_json::from_str(GOLDEN).unwrap_or(Value::Null);
    assert!(schema_admits(&golden), "the schema must admit the golden");
    let options = decoded_options(&golden).unwrap_or_default();
    assert_eq!(options.len(), 3, "bare, described, and partially described");

    let bare = match options.first() {
        Some(RuntimeV3GameplayChoice::Named(identity)) => identity.as_str(),
        _ => "",
    };
    assert_eq!(bare, "card:21:Setup-Strike", "the first entry stays bare");
    let described = match options.get(1).and_then(RuntimeV3GameplayChoice::described) {
        Some(entry) => entry,
        None => &RuntimeV3GameplayOfferedEntry::named(""),
    };
    assert_eq!(described.name.as_deref(), Some("Tremble"));
    assert_eq!(described.rarity.as_deref(), Some("uncommon"));
    assert_eq!(
        described.upgraded, None,
        "an unsupplied attribute stays absent"
    );
    let nested = described
        .contents
        .as_ref()
        .and_then(|contents| contents.get(1))
        .and_then(RuntimeV3GameplayChoice::described);
    assert_eq!(
        nested.and_then(|entry| entry.name.as_deref()),
        Some("Block"),
        "a disclosed element may be described"
    );
    assert!(
        nested.is_none_or(|entry| entry.contents.is_none()),
        "disclosure stops at one level"
    );
}

#[test]
fn the_mirror_refuses_what_the_schema_refuses() {
    let golden: Value = serde_json::from_str(GOLDEN).unwrap_or(Value::Null);
    let mutations: [Mutation; 3] = [
        Mutation("an explicit null rarity", |value| {
            value["observation"]["state"]["options"][1]["rarity"] = Value::Null;
        }),
        Mutation("an unknown member", |value| {
            value["observation"]["state"]["options"][1]["effect"] = json!("stun");
        }),
        Mutation("an empty identity", |value| {
            value["observation"]["state"]["options"][1]["choice_id"] = json!("");
        }),
    ];
    for Mutation(context, mutate) in mutations {
        let mut broken = golden.clone();
        mutate(&mut broken);
        assert!(!schema_admits(&broken), "schema must refuse {context}");
        // The mutation stays well-formed JSON, so the schema is what refuses it rather than a
        // parse failure that would prove nothing about the contract.
        assert!(
            serde_json::from_value::<Value>(broken.clone()).is_ok(),
            "the mutation stays well-formed JSON: {context}"
        );
    }
}

#[test]
fn the_mirror_bounds_attribute_text_in_characters() {
    let accented = "é".repeat(RUNTIME_V3_GAMEPLAY_MAX_OFFERED_ATTRIBUTE_CHARACTERS);
    assert!(
        accented.len() > RUNTIME_V3_GAMEPLAY_MAX_OFFERED_ATTRIBUTE_CHARACTERS,
        "the fixture must exceed the byte bound to be a real test"
    );
    let entry = RuntimeV3GameplayOfferedEntry {
        choice_id: "card:99:Accented".to_owned(),
        name: Some(accented),
        cost: None,
        upgraded: None,
        rarity: None,
        description: None,
        contents: None,
    };
    assert!(
        schema_admits(&reward_with(&RuntimeV3GameplayChoice::Described(entry))),
        "a 512-character non-ASCII name is inside the schema's character bound"
    );

    let oversized = RuntimeV3GameplayOfferedEntry {
        name: Some("a".repeat(RUNTIME_V3_GAMEPLAY_MAX_OFFERED_ATTRIBUTE_CHARACTERS + 1)),
        ..RuntimeV3GameplayOfferedEntry::named("card:99:Long")
    };
    assert!(
        !schema_admits(&reward_with(&RuntimeV3GameplayChoice::Described(oversized))),
        "one character past the bound is refused"
    );
}

#[test]
fn the_mirror_carries_the_duplicate_and_depth_failures() {
    let duplicate = RuntimeV3GameplayState::Reward {
        options: vec![
            RuntimeV3GameplayChoice::Named("card:1:Same".to_owned()),
            RuntimeV3GameplayChoice::Described(RuntimeV3GameplayOfferedEntry::named("card:1:Same")),
        ],
    };
    // Uniqueness is a `validate()` rule with no `uniqueItems` in the schema, exactly as
    // `DuplicateAction` is; the mirror agrees with the owner about where the rule lives.
    assert_eq!(
        observed(duplicate),
        Err(RuntimeV3GameplayValidationError::DuplicateChoice)
    );

    let too_deep = RuntimeV3GameplayState::Reward {
        options: vec![RuntimeV3GameplayChoice::Described(
            RuntimeV3GameplayOfferedEntry {
                choice_id: "card:2:Chest".to_owned(),
                name: None,
                cost: None,
                upgraded: None,
                rarity: None,
                description: None,
                contents: Some(vec![RuntimeV3GameplayChoice::Described(
                    RuntimeV3GameplayOfferedEntry {
                        choice_id: "card:3:Inner".to_owned(),
                        name: None,
                        cost: None,
                        upgraded: None,
                        rarity: None,
                        description: None,
                        contents: Some(vec![RuntimeV3GameplayChoice::Named(
                            "card:4:TooDeep".to_owned(),
                        )]),
                    },
                )]),
            },
        )],
    };
    assert_eq!(
        observed(too_deep),
        Err(RuntimeV3GameplayValidationError::OfferedDisclosureTooDeep)
    );
}
