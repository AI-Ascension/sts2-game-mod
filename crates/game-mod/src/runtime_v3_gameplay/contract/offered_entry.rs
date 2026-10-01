// SPDX-License-Identifier: MIT

//! A host-offered choice that the host may name or describe.
//!
//! An offered set is a list of [`RuntimeV3GameplayChoice`], and a choice is either the bare
//! identity the host named or a [`RuntimeV3GameplayOfferedEntry`] describing it. Both forms stay
//! valid in the same list, so widening the contract to describe an offer does not invalidate a
//! producer that only names one, and one list may mix the two forms where the host disclosed some
//! entries and not others.
//!
//! Every attribute beyond the identity is independently omittable, and an omitted attribute stays
//! omitted. Nothing here is defaulted: an entry the host did not describe as rare has no rarity,
//! and a reader must not read that absence as `common`. In particular a rarity is never derived
//! from a generation pool or weight — the host supplies the value it holds or the attribute is
//! absent.

use serde::{Deserialize, Serialize};

use super::offered_attribute::valid_offered_attribute;
use super::{RUNTIME_V3_GAMEPLAY_MAX_ENTITIES, RuntimeV3GameplayValidationError, valid_identity};

/// One element of a host-offered set: the identity it was named by, or a description of it.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RuntimeV3GameplayChoice {
    /// The identity alone, which is what every producer emitted before an offer could be described.
    Named(String),
    /// The identity plus whatever the host actually supplied about it.
    Described(RuntimeV3GameplayOfferedEntry),
}

impl RuntimeV3GameplayChoice {
    /// The identity this choice was offered under, described or not.
    #[must_use]
    pub fn choice_id(&self) -> &str {
        match self {
            Self::Named(identity) => identity,
            Self::Described(entry) => &entry.choice_id,
        }
    }

    /// The description, when the host supplied one.
    #[must_use]
    pub const fn described(&self) -> Option<&RuntimeV3GameplayOfferedEntry> {
        match self {
            Self::Named(_) => None,
            Self::Described(entry) => Some(entry),
        }
    }

    /// Validates one choice, named or described.
    fn validate_at_depth(
        &self,
        depth: ChoiceDepth,
    ) -> Result<(), RuntimeV3GameplayValidationError> {
        match self {
            Self::Named(identity) => {
                if valid_identity(identity) {
                    Ok(())
                } else {
                    Err(RuntimeV3GameplayValidationError::InvalidIdentity)
                }
            }
            Self::Described(entry) => entry.validate_at_depth(depth),
        }
    }
}

/// A host-offered choice described by exactly the attributes the host supplied.
///
/// `choice_id` is required because it is the identity a legal action refers to; every other member
/// is optional and absent means the host said nothing about it. `contents` is what taking this
/// entry would present next, so a decision to open it can be made knowing what is inside.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayOfferedEntry {
    /// Stable identity this entry is offered and acted on under.
    pub choice_id: String,
    /// Host-supplied label.
    #[serde(
        default,
        deserialize_with = "super::offered_attribute::optional_offered_text",
        skip_serializing_if = "Option::is_none"
    )]
    pub name: Option<String>,
    /// Host-supplied cost.
    #[serde(
        default,
        deserialize_with = "super::offered_attribute::optional_offered_cost",
        skip_serializing_if = "Option::is_none"
    )]
    pub cost: Option<u8>,
    /// Whether the host states this entry is an upgraded form.
    #[serde(
        default,
        deserialize_with = "super::offered_attribute::optional_offered_upgraded",
        skip_serializing_if = "Option::is_none"
    )]
    pub upgraded: Option<bool>,
    /// Host-supplied rarity, never inferred from a generation pool or weight.
    #[serde(
        default,
        deserialize_with = "super::offered_attribute::optional_offered_text",
        skip_serializing_if = "Option::is_none"
    )]
    pub rarity: Option<String>,
    /// Host-supplied visible text.
    #[serde(
        default,
        deserialize_with = "super::offered_attribute::optional_offered_text",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<String>,
    /// What taking this entry would present next, where the host discloses it.
    ///
    /// Disclosure is one level deep. Each disclosed element is named or described by the same
    /// attributes, but a disclosed element carries no `contents` of its own, so the contract bounds
    /// what a reader can be asked to hold rather than leaving the depth to whichever producer nests
    /// deepest.
    #[serde(
        default,
        deserialize_with = "super::offered_attribute::optional_offered_contents",
        skip_serializing_if = "Option::is_none"
    )]
    pub contents: Option<Vec<RuntimeV3GameplayChoice>>,
}

impl RuntimeV3GameplayOfferedEntry {
    /// A described entry carrying only its identity, which is what a bare choice already says.
    #[must_use]
    pub fn named(choice_id: impl Into<String>) -> Self {
        Self {
            choice_id: choice_id.into(),
            name: None,
            cost: None,
            upgraded: None,
            rarity: None,
            description: None,
            contents: None,
        }
    }

    /// Validates one described entry and everything it discloses.
    ///
    /// The depth is what bounds disclosure. A top-level entry may disclose; an entry inside a
    /// disclosed set may not, and saying it does is refused rather than silently ignored, so a
    /// producer cannot smuggle a third level past a reader that only bound two. The schema agrees:
    /// `#/$defs/disclosed_entry` has no `contents` member at all, so a second level fails
    /// validation there and here for the same reason.
    fn validate_at_depth(
        &self,
        depth: ChoiceDepth,
    ) -> Result<(), RuntimeV3GameplayValidationError> {
        if !valid_identity(&self.choice_id) {
            return Err(RuntimeV3GameplayValidationError::InvalidIdentity);
        }
        for attribute in [
            self.name.as_deref(),
            self.rarity.as_deref(),
            self.description.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if !valid_offered_attribute(attribute) {
                return Err(RuntimeV3GameplayValidationError::InvalidText);
            }
        }
        if let Some(contents) = &self.contents {
            // Only a top-level entry may disclose. A disclosed element that carries `contents` is
            // refused here for the same reason the schema refuses it: `#/$defs/disclosed_entry`
            // has no such member and sets `additionalProperties: false`. Letting it through would
            // let a producer nest a level the contract never bounded.
            if depth != ChoiceDepth::Top {
                return Err(RuntimeV3GameplayValidationError::OfferedDisclosureTooDeep);
            }
            validate_choice_elements(contents, depth.child())?;
        }
        Ok(())
    }
}

/// How deep into a disclosed set a choice sits, which is what bounds the recursion.
#[derive(Clone, Copy, Eq, PartialEq)]
enum ChoiceDepth {
    /// A choice in the state's offered set, which may disclose its own contents.
    Top,
    /// A choice inside a disclosed set, which may name or describe itself but not disclose further.
    Disclosure,
    /// A third level, which this contract does not admit.
    Refused,
}

impl ChoiceDepth {
    /// The depth of a choice inside this depth's own disclosed set.
    const fn child(self) -> Self {
        match self {
            Self::Top => Self::Disclosure,
            Self::Disclosure | Self::Refused => Self::Refused,
        }
    }
}

/// Validates one offered set: every entry is well formed and no identity is offered twice.
pub(crate) fn validate_choices(
    choices: &[RuntimeV3GameplayChoice],
) -> Result<(), RuntimeV3GameplayValidationError> {
    validate_choice_elements(choices, ChoiceDepth::Top)
}

/// Validates one offered set whose elements sit at `depth`.
fn validate_choice_elements(
    choices: &[RuntimeV3GameplayChoice],
    depth: ChoiceDepth,
) -> Result<(), RuntimeV3GameplayValidationError> {
    if choices.len() > RUNTIME_V3_GAMEPLAY_MAX_ENTITIES {
        return Err(RuntimeV3GameplayValidationError::CollectionBounds);
    }
    let mut seen: Vec<&str> = Vec::with_capacity(choices.len());
    for choice in choices {
        choice.validate_at_depth(depth)?;
        let choice_id = choice.choice_id();
        if seen.contains(&choice_id) {
            return Err(RuntimeV3GameplayValidationError::DuplicateChoice);
        }
        seen.push(choice_id);
    }
    Ok(())
}
