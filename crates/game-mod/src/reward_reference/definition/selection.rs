// SPDX-License-Identifier: MIT

use super::super::model::{
    RewardField, RewardFieldStatus, RewardSemanticReference, RewardText, RewardVisibility,
};

/// Coarse reward action category available for a selection group.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RewardActionKind {
    /// Claim or choose an offered item.
    Choose,
    /// Skip an optional reward.
    Skip,
    /// Replace an existing item to make room.
    Replace,
    /// Claim an unconditional grant.
    Claim,
    /// Owner-defined action.
    Custom(String),
    /// An action is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the action.
    Unknown,
}

/// One legal action the host may offer for a selection group.
///
/// This is static vocabulary; the transient action identity a live host assigns is a separate
/// [`super::super::model::RewardActionReference`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardLegalAction {
    /// Action category.
    pub action_kind: RewardActionKind,
    /// Localized/source-defined action label.
    pub label: RewardText,
    /// Optional condition identity that gates the action.
    pub condition: RewardField<String>,
    /// Visibility of the static action.
    pub visibility: RewardVisibility,
}

/// Selection group and choose/skip constraints before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardSelectionInput {
    /// Stable selection-group identity scoped by the reward definition.
    pub group_id: String,
    /// Localized/source-defined group label.
    pub label: RewardText,
    /// Minimum number of items the caller must choose.
    pub choose_min: RewardField<u32>,
    /// Maximum number of items the caller may choose.
    pub choose_max: RewardField<u32>,
    /// Whether the whole reward may be skipped.
    pub optional_skip: RewardField<bool>,
    /// Legal actions offered for this group.
    pub legal_actions: Vec<RewardLegalAction>,
    /// Typed rule/content links.
    pub references: Vec<RewardSemanticReference>,
    /// Visibility of the static selection group.
    pub visibility: RewardVisibility,
}

/// Selection group bound to its owning reward definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardSelection {
    /// Stable selection-group identity.
    pub group_id: String,
    /// Localized/source-defined group label.
    pub label: RewardText,
    /// Minimum number of items the caller must choose.
    pub choose_min: RewardField<u32>,
    /// Maximum number of items the caller may choose.
    pub choose_max: RewardField<u32>,
    /// Whether the whole reward may be skipped.
    pub optional_skip: RewardField<bool>,
    /// Legal actions offered for this group.
    pub legal_actions: Vec<RewardLegalAction>,
    /// Availability of the legal actions after scope withholding.
    pub legal_actions_status: RewardFieldStatus,
    /// Typed rule/content links.
    pub references: Vec<RewardSemanticReference>,
    /// Visibility of the static selection group.
    pub visibility: RewardVisibility,
}

impl RewardSelection {
    pub(super) fn from_input(input: RewardSelectionInput) -> Self {
        Self {
            group_id: input.group_id,
            label: input.label,
            choose_min: input.choose_min,
            choose_max: input.choose_max,
            optional_skip: input.optional_skip,
            legal_actions: input.legal_actions,
            legal_actions_status: RewardFieldStatus::Available,
            references: input.references,
            visibility: input.visibility,
        }
    }
}

/// Whether accepting a reward requires replacing an existing item.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RewardReplacementPolicy {
    /// No replacement is required.
    NotRequired,
    /// The destination is full and a replacement must be chosen.
    Required,
    /// A replacement may be chosen but is not mandatory.
    Optional,
    /// Source could not classify the replacement behaviour.
    Unknown,
}

/// Static support for the distinct reward-offer states.
///
/// This records which states a definition can produce. It is not a live observation: a live
/// [`super::super::model::RewardOfferState`] is bound to a run/room/snapshot and is never derived
/// from this policy alone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardStatePolicy {
    /// Maximum number of times this reward may be claimed, when known.
    pub claim_limit: RewardField<u32>,
    /// Destination capacity that can block the reward, when known.
    pub capacity: RewardField<u32>,
    /// Replacement behaviour required by this reward.
    pub replacement: RewardField<RewardReplacementPolicy>,
    /// Number of stages for a multi-stage grant, when known.
    pub multi_stage: RewardField<u32>,
    /// Visibility of the static state policy.
    pub visibility: RewardVisibility,
}
