// SPDX-License-Identifier: MIT

use super::{
    cost::{ActionCostContributor, ActionTargetRestriction},
    definition::ActionDefinition,
    eligibility::ActionEligibility,
    field::{ActionFieldStatus, ActionText},
    kind::{
        ActionDispatchAuthority, ActionKind, ActionParentOperation, ActionPreviewClass,
        ActionRefusalReason,
    },
    model::{
        ActionCatalogBinding, ActionEvidence, ActionFrameReference, ActionReference,
        ActionVisibility, ActionVisibilityScope,
    },
    projection::map_status,
    target::ActionTarget,
};

/// Bounded availability-explanation request for one static legal-action frame.
///
/// The frame names the legal-action generation the caller observed, so an explanation asked for
/// after a relevant transition is refused rather than answered with the current frame's reason.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionAvailabilityQuery {
    /// Exact static legal-action frame the caller is asking about.
    pub frame: ActionFrameReference,
    /// Visibility scope.
    pub scope: ActionVisibilityScope,
}

/// Owner-local explanation of why one action is, or is not, available right now.
///
/// The explanation never invents a reason: the refusal code and its text are the host's own
/// statement, and every record that currently blocks the action is reported alongside the amounts
/// the source observed, so an unmet resource, an invalid or dead target, a full destination, a
/// disabled option, and a required selection each name the record that produces them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionAvailabilityExplanation {
    /// Catalog witness for every value.
    pub binding: ActionCatalogBinding,
    /// Exact static legal-action reference.
    pub reference: ActionReference,
    /// Operation whose frame owns this action.
    pub parent: ActionParentOperation,
    /// Exact-build legal-action family.
    pub kind: ActionKind,
    /// Localized action label.
    pub label: ActionText,
    /// Visibility of the definition.
    pub visibility: ActionVisibility,
    /// Evidence label for the definition.
    pub evidence: ActionEvidence,
    /// Observed legal-action generation this explanation is bound to.
    pub instance_generation: u64,
    /// Resolved availability and the host's refusal reason.
    pub eligibility: ActionEligibility,
    /// Every bounded cost contributor the action draws on.
    pub costs: Vec<ActionCostContributor>,
    /// Every restriction the action imposes on its target or state.
    pub restrictions: Vec<ActionTargetRestriction>,
    /// Identities of the cost contributors that currently block the action.
    pub blocking_costs: Vec<String>,
    /// Identities of the restrictions that currently block the action.
    pub blocking_restrictions: Vec<String>,
    /// Preview classifications the action declares as supported.
    pub preview_classes: Vec<ActionPreviewClass>,
    /// Number of visible observed targets.
    pub target_count: usize,
    /// Availability of the target list after scope withholding.
    pub targets_status: ActionFieldStatus,
    /// Number of visible declared previews.
    pub preview_count: usize,
    /// Availability of the preview list after scope withholding.
    pub previews_status: ActionFieldStatus,
    /// Dispatch authority an explanation carries.
    ///
    /// Explaining an action is evidence about its availability, not permission to take it.
    pub authority: ActionDispatchAuthority,
}

impl ActionAvailabilityExplanation {
    /// Returns whether the action may be dispatched with fresh validation.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.eligibility.is_available()
    }

    /// Returns the host's refusal reason, when the action is refused for a stated reason.
    #[must_use]
    pub fn refusal(&self) -> Option<&ActionRefusalReason> {
        self.eligibility.refusal()
    }

    /// Returns the host's own reason text for a refused action.
    #[must_use]
    pub const fn reason_text(&self) -> &ActionText {
        &self.eligibility.reason_text
    }

    /// Returns whether this explanation actually accounts for the reported refusal.
    ///
    /// An available action has nothing to explain. A refused one is explained only when the host
    /// stated a reason and at least one blocking cost contributor or unsatisfied restriction
    /// produces it, so a refusal is never published as a bare disabled label.
    #[must_use]
    pub fn explains_refusal(&self) -> bool {
        if self.is_available() {
            return true;
        }
        self.refusal().is_some()
            && (!self.blocking_costs.is_empty() || !self.blocking_restrictions.is_empty())
    }

    /// Returns one blocking cost contributor by identity.
    #[must_use]
    pub fn blocking_cost(&self, cost_id: &str) -> Option<&ActionCostContributor> {
        self.blocking_costs
            .iter()
            .find(|id| id.as_str() == cost_id)
            .and_then(|id| self.costs.iter().find(|cost| &cost.cost_id == id))
    }

    /// Returns one blocking restriction by identity.
    #[must_use]
    pub fn blocking_restriction(&self, restriction_id: &str) -> Option<&ActionTargetRestriction> {
        self.blocking_restrictions
            .iter()
            .find(|id| id.as_str() == restriction_id)
            .and_then(|id| {
                self.restrictions
                    .iter()
                    .find(|rule| &rule.restriction_id == id)
            })
    }
}

/// Builds the bounded availability explanation for one validated definition.
pub(super) fn explain_availability(
    definition: &ActionDefinition,
    scope: ActionVisibilityScope,
) -> ActionAvailabilityExplanation {
    ActionAvailabilityExplanation {
        binding: definition.reference.catalog.clone(),
        reference: definition.reference.clone(),
        parent: definition.parent.clone(),
        kind: definition.kind.clone(),
        label: definition.label.clone(),
        visibility: definition.visibility,
        evidence: definition.evidence,
        instance_generation: definition.instance_generation,
        eligibility: definition.eligibility.clone(),
        costs: definition.costs.clone(),
        restrictions: definition.restrictions.clone(),
        blocking_costs: definition
            .blocking_costs()
            .map(|cost| cost.cost_id.clone())
            .collect(),
        blocking_restrictions: definition
            .blocking_restrictions()
            .map(|rule| rule.restriction_id.clone())
            .collect(),
        preview_classes: definition.preview_classes.clone(),
        target_count: definition
            .targets
            .values()
            .filter(|target| super::projection::visible_target(target, scope))
            .count(),
        targets_status: map_status(
            definition.targets.values(),
            definition.targets_status,
            |target: &ActionTarget| super::projection::visible_target(target, scope),
        ),
        preview_count: definition
            .previews
            .values()
            .filter(|preview| super::projection::visible_preview(preview, definition, scope))
            .count(),
        previews_status: map_status(
            definition.previews.values(),
            definition.previews_status,
            |preview| super::projection::visible_preview(preview, definition, scope),
        ),
        authority: ActionDispatchAuthority::NotGranted,
    }
}
