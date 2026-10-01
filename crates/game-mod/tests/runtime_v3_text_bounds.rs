// SPDX-License-Identifier: MIT

//! The text and identity bounds must match the schema: text counts characters, identity bytes.
//!
//! `maxLength` in JSON Schema counts Unicode characters, so a 512-character non-ASCII name is
//! admitted. A `str::len()` bound would refuse it, making the mirror stricter than the schema.

use serde_json::Value;
use sts2_game_mod::{
    RUNTIME_V3_GAMEPLAY_MAX_IDENTITY_BYTES, RUNTIME_V3_GAMEPLAY_MAX_TEXT_CHARACTERS,
    RuntimeV3GameplayMessage, RuntimeV3GameplayObservation, RuntimeV3GameplayValidationError,
};

const SCHEMA: &str = include_str!("../../../schemas/runtime-v3-gameplay.schema.json");
const STATE: &str =
    include_str!("../../../protocol-artifact/runtime-v3-gameplay/golden/state-response.json");

fn golden() -> Result<RuntimeV3GameplayObservation, RuntimeV3GameplayValidationError> {
    serde_json::from_str::<RuntimeV3GameplayMessage>(STATE)
        .ok()
        .and_then(|message| message.observation)
        .ok_or(RuntimeV3GameplayValidationError::Metadata)
}

/// The golden observation with `text` as its visible seed, a free-form `text` field.
fn with_seed(text: &str) -> Result<(), RuntimeV3GameplayValidationError> {
    let mut observation = golden()?;
    observation.visible_seed = Some(text.to_owned());
    observation.validate()
}

/// The golden observation with `identity` as its state identity.
fn with_state_id(identity: &str) -> Result<(), RuntimeV3GameplayValidationError> {
    let mut observation = golden()?;
    observation.state_id = identity.to_owned();
    observation.validate()
}

#[test]
fn the_golden_observation_is_valid_before_any_mutation() {
    assert_eq!(
        golden().and_then(|observation| observation.validate()),
        Ok(())
    );
}

#[test]
fn the_constants_match_the_schema_bounds() {
    let schema: Value = serde_json::from_str(SCHEMA).unwrap_or(Value::Null);
    let text = &schema["$defs"]["text"];
    assert_eq!(
        text["maxLength"], RUNTIME_V3_GAMEPLAY_MAX_TEXT_CHARACTERS,
        "text maxLength is in characters"
    );
    assert_eq!(text["minLength"], 1);
    let pattern = schema["$defs"]["identity"]["pattern"]
        .as_str()
        .unwrap_or("");
    assert_eq!(
        pattern,
        format!("^[A-Za-z0-9_.:/-]{{1,{RUNTIME_V3_GAMEPLAY_MAX_IDENTITY_BYTES}}}$"),
        "identity is ASCII, so its byte and character bounds coincide"
    );
}

#[test]
fn text_admits_the_maximum_in_characters_even_when_multibyte() {
    let accented = "é".repeat(RUNTIME_V3_GAMEPLAY_MAX_TEXT_CHARACTERS);
    assert!(
        accented.len() > RUNTIME_V3_GAMEPLAY_MAX_TEXT_CHARACTERS,
        "the fixture must exceed the byte bound to be a real test"
    );
    assert_eq!(with_seed(&accented), Ok(()));
    let astral = "🂡".repeat(RUNTIME_V3_GAMEPLAY_MAX_TEXT_CHARACTERS);
    assert_eq!(
        with_seed(&astral),
        Ok(()),
        "a four-byte character counts once"
    );
}

#[test]
fn text_refuses_one_character_past_the_maximum() {
    let accented = "é".repeat(RUNTIME_V3_GAMEPLAY_MAX_TEXT_CHARACTERS + 1);
    assert_eq!(
        with_seed(&accented),
        Err(RuntimeV3GameplayValidationError::InvalidText)
    );
    let ascii = "a".repeat(RUNTIME_V3_GAMEPLAY_MAX_TEXT_CHARACTERS + 1);
    assert_eq!(
        with_seed(&ascii),
        Err(RuntimeV3GameplayValidationError::InvalidText)
    );
}

#[test]
fn text_still_refuses_empty_and_control_characters() {
    assert_eq!(
        with_seed("a".repeat(RUNTIME_V3_GAMEPLAY_MAX_TEXT_CHARACTERS).as_str()),
        Ok(())
    );
    for refused in [
        "",
        "tab\there",
        "line\nbreak",
        "nul\0",
        "\u{7f}",
        "c1\u{85}",
    ] {
        assert_eq!(
            with_seed(refused),
            Err(RuntimeV3GameplayValidationError::InvalidText),
            "{refused:?}"
        );
    }
}

#[test]
fn identity_keeps_its_ascii_byte_bound_and_charset() {
    let longest = "a".repeat(RUNTIME_V3_GAMEPLAY_MAX_IDENTITY_BYTES);
    assert_eq!(with_state_id(&longest), Ok(()));
    let over = "a".repeat(RUNTIME_V3_GAMEPLAY_MAX_IDENTITY_BYTES + 1);
    assert_eq!(
        with_state_id(&over),
        Err(RuntimeV3GameplayValidationError::InvalidIdentity)
    );
    for refused in ["", "é", "has space", "ctl\n"] {
        assert_eq!(
            with_state_id(refused),
            Err(RuntimeV3GameplayValidationError::InvalidIdentity),
            "{refused:?}"
        );
    }
}

#[test]
fn text_counts_unicode_scalar_values_not_graphemes() {
    let max = RUNTIME_V3_GAMEPLAY_MAX_TEXT_CHARACTERS;
    // One grapheme, two scalars: `e` plus a combining acute accent.
    let combining = "e\u{301}".repeat(max / 2);
    assert_eq!(combining.chars().count(), max);
    assert_eq!(with_seed(&combining), Ok(()));
    assert_eq!(
        with_seed(&format!("{combining}e\u{301}")),
        Err(RuntimeV3GameplayValidationError::InvalidText),
        "each combining mark is its own scalar"
    );
    // A ZWJ family emoji is one grapheme but seven scalars; 73 of them are 511 scalars.
    let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}";
    assert_eq!(family.chars().count(), 7);
    let zwj = family.repeat(max / 7);
    assert_eq!(with_seed(&zwj), Ok(()));
    assert_eq!(
        with_seed(&format!("{zwj}{family}")),
        Err(RuntimeV3GameplayValidationError::InvalidText)
    );
}

#[test]
#[allow(deprecated)]
fn the_deprecated_alias_keeps_its_legacy_value() {
    assert_eq!(sts2_game_mod::RUNTIME_V3_GAMEPLAY_MAX_TEXT_BYTES, 512);
}
