// SPDX-License-Identifier: MIT

use super::{
    field::{ActionField, ActionText},
    kind::{
        ActionEffectKind, ActionOmissionKind, ActionPile, ActionPreviewClass,
        ActionPreviewProvenance, ActionStatusTransition,
    },
    model::{
        ActionEvidence, ActionPreviewReference, ActionSemanticReference, ActionTargetReference,
    },
};

/// One target-specific consequence a preview states, with the unit its amounts use.
///
/// A consequence never folds several contributors into an invented total, and a consequence that
/// would change nothing is refused rather than published as a change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionPreviewChange {
    /// Stable change identity within its preview.
    pub change_id: String,
    /// Localized change description.
    pub label: ActionText,
    /// Kind of consequence.
    pub kind: ActionEffectKind,
    /// Localized unit the amounts are stated in, or an explicit non-value.
    pub unit: ActionText,
    /// Definition the change affects, or an explicit non-value.
    pub target: ActionField<ActionSemanticReference>,
    /// Documented value before the action, or an explicit non-value.
    pub before: ActionField<String>,
    /// Documented value after the action, or an explicit non-value.
    pub after: ActionField<String>,
    /// Documented signed change, or an explicit non-value.
    pub delta: ActionField<i64>,
    /// Definitions this change refers to.
    pub references: Vec<ActionSemanticReference>,
}

/// One status or power transition a preview states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionPreviewStatus {
    /// Stable status identity within its preview.
    pub status_id: String,
    /// Localized status description.
    pub label: ActionText,
    /// Definition of the status or power.
    pub status: ActionSemanticReference,
    /// Direction of the transition.
    pub transition: ActionStatusTransition,
    /// Magnitude of the transition, or an explicit non-value when the source states none.
    pub magnitude: ActionField<i64>,
    /// Definition the status applies to, or an explicit non-value.
    pub target: ActionField<ActionSemanticReference>,
    /// Definitions this transition refers to.
    pub references: Vec<ActionSemanticReference>,
}

/// One card movement a preview states.
///
/// Every movement states both endpoints, so an agent can tell a draw from a discard instead of
/// inferring the destination from the pile it saw.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionPreviewMovement {
    /// Stable movement identity within its preview.
    pub movement_id: String,
    /// Localized movement description.
    pub label: ActionText,
    /// Definition of the card that moves.
    pub card: ActionSemanticReference,
    /// Pile the movement starts from.
    pub from: ActionPile,
    /// Pile the movement ends in.
    pub to: ActionPile,
    /// Number of copies that move.
    pub count: u32,
    /// Definitions this movement refers to.
    pub references: Vec<ActionSemanticReference>,
}

/// One selection the action still requires, named by identity rather than described in prose.
///
/// The named selection resolves in the same manifest as the action, so a caller can hand it to the
/// selection reference reader instead of guessing what the prompt will ask for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionPreviewSelection {
    /// Stable selection-requirement identity within its preview.
    pub requirement_id: String,
    /// Localized selection description.
    pub label: ActionText,
    /// Definition of the selection the action requires.
    pub selection: ActionSemanticReference,
    /// Picks the selection requires before the action may be dispatched.
    pub required: u32,
    /// Picks the selection accepts at most, or an explicit non-value.
    pub maximum: ActionField<u32>,
    /// Definitions this requirement refers to.
    pub references: Vec<ActionSemanticReference>,
}

/// One stated condition a preview's consequences depend on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionPreviewAssumption {
    /// Stable assumption identity within its preview.
    pub assumption_id: String,
    /// Localized assumption description.
    pub label: ActionText,
    /// Definitions this assumption refers to.
    pub references: Vec<ActionSemanticReference>,
}

/// One interaction a preview explicitly does not describe.
///
/// A random or unsupported chain must name why it is not described, so an incomplete consequence
/// set is published as incomplete rather than as a complete answer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionPreviewOmission {
    /// Stable omission identity within its preview.
    pub omission_id: String,
    /// Class of interaction that is not described.
    pub kind: ActionOmissionKind,
    /// Localized omission description.
    pub label: ActionText,
    /// Definitions this omission refers to.
    pub references: Vec<ActionSemanticReference>,
}

/// Owner-supplied preview used to construct one immutable catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionPreviewInput {
    /// Stable preview identity within its action.
    pub preview_id: String,
    /// Localized preview description.
    pub label: ActionText,
    /// Classification of how confidently this preview describes its consequences.
    pub class: ActionPreviewClass,
    /// Target identity this preview applies to, or an explicit non-value for an untargeted action.
    pub target: ActionField<String>,
    /// Target identities the preview states are affected.
    pub affected: Vec<String>,
    /// Target-specific consequences.
    pub changes: Vec<ActionPreviewChange>,
    /// Status and power transitions.
    pub statuses: Vec<ActionPreviewStatus>,
    /// Card movements.
    pub movements: Vec<ActionPreviewMovement>,
    /// Selections the action still requires.
    pub selections: Vec<ActionPreviewSelection>,
    /// Conditions the consequences depend on.
    pub assumptions: Vec<ActionPreviewAssumption>,
    /// Interactions this preview does not describe.
    pub omissions: Vec<ActionPreviewOmission>,
    /// Where the consequences came from.
    pub provenance: ActionPreviewProvenance,
    /// Evidence label for the preview.
    pub evidence: ActionEvidence,
    /// Definitions the preview refers to.
    pub references: Vec<ActionSemanticReference>,
}

/// Immutable preview bound to one exact legal-action definition and target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionPreview {
    /// Exact static preview reference.
    pub reference: ActionPreviewReference,
    /// Localized preview description.
    pub label: ActionText,
    /// Classification of how confidently this preview describes its consequences.
    pub class: ActionPreviewClass,
    /// Exact target this preview applies to, or an explicit non-value for an untargeted action.
    pub target: ActionField<ActionTargetReference>,
    /// Exact targets the preview states are affected.
    pub affected: Vec<ActionTargetReference>,
    /// Target-specific consequences.
    pub changes: Vec<ActionPreviewChange>,
    /// Status and power transitions.
    pub statuses: Vec<ActionPreviewStatus>,
    /// Card movements.
    pub movements: Vec<ActionPreviewMovement>,
    /// Selections the action still requires.
    pub selections: Vec<ActionPreviewSelection>,
    /// Conditions the consequences depend on.
    pub assumptions: Vec<ActionPreviewAssumption>,
    /// Interactions this preview does not describe.
    pub omissions: Vec<ActionPreviewOmission>,
    /// Where the consequences came from.
    pub provenance: ActionPreviewProvenance,
    /// Evidence label for the preview.
    pub evidence: ActionEvidence,
    /// Definitions the preview refers to.
    pub references: Vec<ActionSemanticReference>,
}

impl ActionPreview {
    /// Returns the number of stated consequences across every consequence list.
    #[must_use]
    pub fn consequence_count(&self) -> usize {
        self.changes.len() + self.statuses.len() + self.movements.len() + self.selections.len()
    }

    /// Returns whether this preview states any consequence at all.
    #[must_use]
    pub fn states_consequence(&self) -> bool {
        self.consequence_count() > 0
    }

    /// Returns whether this preview admits it does not describe every interaction.
    #[must_use]
    pub fn is_incomplete(&self) -> bool {
        !self.omissions.is_empty() || !matches!(self.class, ActionPreviewClass::DeterministicExact)
    }
}
