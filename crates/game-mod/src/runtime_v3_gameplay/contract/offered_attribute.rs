// SPDX-License-Identifier: MIT

//! How a described offered entry is read and bounded.
//!
//! These are the parser's rules for the attributes a host may supply about an offer. They live
//! apart from the message shape because they answer two different questions: the shape decides what
//! an offered entry *is*, and this module decides which attribute strings and explicit nulls are
//! legal once one is there.

use super::RuntimeV3GameplayChoice;

/// Maximum character length of one host-supplied offered-entry attribute.
///
/// Mirrored from `sts2-protocol`: JSON Schema's `maxLength` counts Unicode characters, not bytes,
/// so a byte bound here would refuse a 512-character non-ASCII name the schema admits.
pub const RUNTIME_V3_GAMEPLAY_MAX_OFFERED_ATTRIBUTE_CHARACTERS: usize = 512;

/// Validates one host-supplied offered-entry attribute.
///
/// The schema counts these attributes in characters (`maxLength`) and rejects control characters,
/// so this counts characters too. Every offered attribute is bounded this way rather than by
/// [`super::RUNTIME_V3_GAMEPLAY_MAX_TEXT_BYTES`] because that constant is a byte bound for the
/// fields that were already in the contract, and adopting it here would make the parser stricter
/// than the schema it is written against.
pub(crate) fn valid_offered_attribute(value: &str) -> bool {
    !value.is_empty()
        && value.chars().count() <= RUNTIME_V3_GAMEPLAY_MAX_OFFERED_ATTRIBUTE_CHARACTERS
        && !value.chars().any(char::is_control)
}

/// Deserializes an offered-entry text attribute that may be absent but must never be null.
///
/// `#[serde(default)]` alone would accept an explicit `null`, which the schema's optional
/// `#/$defs/text` member does not. Rejecting it here keeps absence and an explicit null distinct:
/// absent means the host said nothing, and null is not a way to say it.
pub(crate) fn optional_offered_text<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    optional_offered_member(deserializer, "a JSON string")
}

/// Deserializes an offered-entry cost that may be absent but must never be null.
pub(crate) fn optional_offered_cost<'de, D>(deserializer: D) -> Result<Option<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    optional_offered_member(deserializer, "a JSON integer in 0..=255")
}

/// Deserializes an offered-entry upgrade flag that may be absent but must never be null.
pub(crate) fn optional_offered_upgraded<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    optional_offered_member(deserializer, "a JSON boolean")
}

/// Deserializes an offered-entry `contents` member that may be absent but must never be null.
pub(crate) fn optional_offered_contents<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<RuntimeV3GameplayChoice>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    optional_offered_member(deserializer, "a JSON array")
}

/// Deserializes one optional offered-entry member, refusing an explicit `null`.
pub(crate) fn optional_offered_member<'de, D, T>(
    deserializer: D,
    expected: &str,
) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let value: serde_json::Value = serde::Deserialize::deserialize(deserializer)?;
    match value {
        serde_json::Value::Null => Err(serde::de::Error::custom(format!(
            "offered-entry member must not be null; omit the member instead, expected {expected}"
        ))),
        serde_json::Value::Bool(flag) => serde_json::from_value(serde_json::Value::Bool(flag))
            .map(Some)
            .map_err(serde::de::Error::custom),
        serde_json::Value::Number(number) => {
            serde_json::from_value(serde_json::Value::Number(number))
                .map(Some)
                .map_err(serde::de::Error::custom)
        }
        serde_json::Value::String(text) => serde_json::from_value(serde_json::Value::String(text))
            .map(Some)
            .map_err(serde::de::Error::custom),
        serde_json::Value::Array(items) => serde_json::from_value(serde_json::Value::Array(items))
            .map(Some)
            .map_err(serde::de::Error::custom),
        _ => Err(serde::de::Error::custom(format!(
            "offered-entry member must be {expected}"
        ))),
    }
}
