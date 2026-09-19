// SPDX-License-Identifier: MIT

use super::{REFERENCE_TEXT_MAX_IDENTITY_BYTES, ReferenceTextError};

/// Presentation tokens that would let stored reference text execute inside a structured consumer.
///
/// Reference text is inert data, so a value carrying one of these is rejected rather than
/// repaired.  Ordinary `<`, `>` and `&` remain allowed: this slice preserves the owner's
/// formatting semantics, it does not strip markup.
const EXECUTABLE_TOKENS: [&str; 4] = ["<script", "javascript:", "data:text/html", "vbscript:"];

/// Text shapes that would make a reference read look like an instruction to its consumer.
///
/// A reference read is data an agent is free to ignore.  A stored value that addresses the
/// consumer directly is refused so that game and mod text can never become authority, matching
/// the requirement that text stays inert even when it is phrased imperatively.
const INSTRUCTION_TOKENS: [&str; 4] = [
    "ignore previous",
    "ignore all previous",
    "system prompt",
    "you must now",
];

/// Returns whether a stored value carries executable or otherwise unsafe presentation.
///
/// Unicode, right-to-left text and reference links are preserved.  Only control characters that
/// cannot appear in rendered text, the executable tokens above, and the instruction shapes above
/// are refused.
#[must_use]
pub(super) fn unsafe_reason(value: &str) -> Option<&'static str> {
    if value
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
    {
        return Some("control_character");
    }
    let lowered = value.to_ascii_lowercase();
    if EXECUTABLE_TOKENS
        .iter()
        .any(|token| lowered.contains(token))
    {
        return Some("executable_presentation");
    }
    if INSTRUCTION_TOKENS
        .iter()
        .any(|token| lowered.contains(token))
    {
        return Some("instruction_bearing");
    }
    None
}

/// Validates one literal stored value without changing its meaning.
pub(super) fn validate_inert_text(
    value: &str,
    field: &'static str,
) -> Result<(), ReferenceTextError> {
    match unsafe_reason(value) {
        None => Ok(()),
        Some("instruction_bearing") => Err(ReferenceTextError::InstructionBearingText(field)),
        Some(_) => Err(ReferenceTextError::UnsafePresentation(field)),
    }
}

/// Validates one searchable keyword.
pub(super) fn validate_keyword(value: &str) -> Result<(), ReferenceTextError> {
    if value.is_empty() || value.len() > REFERENCE_TEXT_MAX_IDENTITY_BYTES {
        return Err(ReferenceTextError::InvalidInput("keyword"));
    }
    validate_inert_text(value, "keyword")
}
