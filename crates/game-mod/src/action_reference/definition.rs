// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::{
    cost::{ActionCostContributor, ActionTargetRestriction},
    eligibility::ActionEligibility,
    field::{ActionField, ActionFieldStatus, ActionText},
    kind::{ActionKind, ActionParentOperation, ActionPreviewClass},
    model::{
        ActionCatalogBinding, ActionEvidence, ActionFamilyCoverage, ActionInstanceReference,
        ActionReference, ActionSemanticReference, ActionVisibility, ActionVisibilityScope,
    },
    preview::{ActionPreview, ActionPreviewInput},
    reader::ActionReader,
    target::{ActionTarget, ActionTargetInput},
};

/// Support state recorded for one audited action or target.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionCoverageState {
    /// The target has a typed record in this slice.
    Handled,
    /// The target is known but this producer has no typed record for it.
    Unsupported,
    /// The target is known but currently unavailable from this source.
    Unavailable,
}

/// Named coverage record for one host-reported action or target.
///
/// Every action and target the host reports is either described by a typed record or named here,
/// so a new action or entry is audited explicitly instead of being silently omitted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionCoverageRecord {
    /// Host-reported identity this record covers.
    pub target_id: String,
    /// Support state for the covered target.
    pub state: ActionCoverageState,
    /// Localized explanation for an unsupported or unavailable target.
    pub reason: ActionText,
}

/// Owner-supplied legal-action definition used to construct one immutable catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionDefinitionInput {
    /// Namespaced legal-action identity.
    pub action_id: String,
    /// Operation whose legal-action frame owns this action.
    pub parent: ActionParentOperation,
    /// Exact-build legal-action family.
    pub kind: ActionKind,
    /// Localized action label.
    pub label: ActionText,
    /// Visibility of the definition.
    pub visibility: ActionVisibility,
    /// Evidence label for the definition.
    pub evidence: ActionEvidence,
    /// Generation of the legal-action frame this definition describes.
    pub instance_generation: u64,
    /// Resolved action availability and refusal reason.
    pub eligibility: ActionEligibility,
    /// Bounded cost contributors this action draws on.
    pub costs: Vec<ActionCostContributor>,
    /// Restrictions this action imposes on the target or state it accepts.
    pub restrictions: Vec<ActionTargetRestriction>,
    /// Target identities the host reports for this action.
    ///
    /// Every reported identity must be either described by a typed target or named by a coverage
    /// record, so a newly audited entry cannot be silently omitted.
    pub observed_targets: Vec<String>,
    /// Typed observed targets for this action.
    pub targets: Vec<ActionTargetInput>,
    /// Named coverage records for reported actions and targets without a typed record.
    pub coverage: Vec<ActionCoverageRecord>,
    /// Preview classifications this action supports.
    pub preview_classes: Vec<ActionPreviewClass>,
    /// Declared previews for this action.
    pub previews: Vec<ActionPreviewInput>,
    /// Transient live action-instance identity.
    ///
    /// A static slice must not carry one: the host assigns instance identities per presented frame,
    /// so an available value here is rejected rather than copied into static reference data.
    pub instance: ActionField<ActionInstanceReference>,
    /// Definitions the definition refers to.
    pub references: Vec<ActionSemanticReference>,
}

/// Immutable legal-action definition with bounded, deterministic target and preview collections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionDefinition {
    /// Exact static definition reference.
    pub reference: ActionReference,
    /// Operation whose legal-action frame owns this action.
    pub parent: ActionParentOperation,
    /// Exact-build legal-action family.
    pub kind: ActionKind,
    /// Localized action label.
    pub label: ActionText,
    /// Visibility of the definition.
    pub visibility: ActionVisibility,
    /// Evidence label for the definition.
    pub evidence: ActionEvidence,
    /// Generation of the legal-action frame this definition describes.
    pub instance_generation: u64,
    /// Resolved action availability and refusal reason.
    pub eligibility: ActionEligibility,
    /// Bounded cost contributors this action draws on.
    pub costs: Vec<ActionCostContributor>,
    /// Restrictions this action imposes on the target or state it accepts.
    pub restrictions: Vec<ActionTargetRestriction>,
    /// Number of target identities the host reported for this action.
    pub observed_target_count: usize,
    /// Typed observed targets keyed by target identity.
    pub targets: BTreeMap<String, ActionTarget>,
    /// Availability of the target list after scope withholding.
    pub targets_status: ActionFieldStatus,
    /// Named coverage records for audited targets without a typed record.
    pub coverage: Vec<ActionCoverageRecord>,
    /// Availability of the coverage list after scope withholding.
    pub coverage_status: ActionFieldStatus,
    /// Preview classifications this action supports.
    pub preview_classes: Vec<ActionPreviewClass>,
    /// Declared previews keyed by preview identity.
    pub previews: BTreeMap<String, ActionPreview>,
    /// Availability of the preview list after scope withholding.
    pub previews_status: ActionFieldStatus,
    /// Definitions the definition refers to.
    pub references: Vec<ActionSemanticReference>,
}

impl ActionDefinition {
    /// Returns whether this action may be dispatched with fresh validation.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.eligibility.is_available()
    }

    /// Returns whether the source declares one preview classification as supported.
    #[must_use]
    pub fn declares_class(&self, class: ActionPreviewClass) -> bool {
        self.preview_classes.contains(&class)
    }

    /// Returns the declared preview for one target identity, or the untargeted default preview.
    ///
    /// `None` means this action declares no preview for the requested subject, so the caller is
    /// answered with an explicitly unavailable preview rather than an invented consequence.
    #[must_use]
    pub fn preview_for(&self, target_id: Option<&str>) -> Option<&ActionPreview> {
        self.previews.values().find(|preview| {
            preview
                .target
                .value()
                .map(|target| target.target_id.as_str())
                == target_id
        })
    }

    /// Returns the cost contributors that currently block this action.
    pub fn blocking_costs(&self) -> impl Iterator<Item = &ActionCostContributor> {
        self.costs.iter().filter(|cost| !cost.affordable)
    }

    /// Returns the restrictions that currently block this action.
    pub fn blocking_restrictions(&self) -> impl Iterator<Item = &ActionTargetRestriction> {
        self.restrictions.iter().filter(|rule| !rule.is_satisfied())
    }
}

/// Immutable, read-only legal-action and preview reference catalog.
///
/// The catalog owns no host handle, no queue, no epoch lease, and no transient action identity, and
/// exposes no dispatch entry point: every method here only reads what the source already copied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionCatalog {
    /// Catalog witness for every entry.
    pub binding: ActionCatalogBinding,
    /// Explicit family coverage for the action family.
    pub family: ActionFamilyCoverage,
    definitions: BTreeMap<String, ActionDefinition>,
}

impl ActionCatalog {
    /// Creates a catalog from validated parts.
    pub(super) fn from_parts(
        binding: ActionCatalogBinding,
        family: ActionFamilyCoverage,
        definitions: BTreeMap<String, ActionDefinition>,
    ) -> Self {
        Self {
            binding,
            family,
            definitions,
        }
    }

    /// Returns the locale every localized value was copied for.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.binding.locale
    }

    /// Returns the number of legal-action definitions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Returns whether the catalog carries no legal-action definition.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Returns one legal-action definition by identity.
    #[must_use]
    pub fn definition(&self, action_id: &str) -> Option<&ActionDefinition> {
        self.definitions.get(action_id)
    }

    /// Returns one observed target by its action and target identity.
    #[must_use]
    pub fn target(&self, action_id: &str, target_id: &str) -> Option<&ActionTarget> {
        self.definition(action_id)
            .and_then(|definition| definition.targets.get(target_id))
    }

    /// Returns one declared preview by its action and preview identity.
    #[must_use]
    pub fn preview(&self, action_id: &str, preview_id: &str) -> Option<&ActionPreview> {
        self.definition(action_id)
            .and_then(|definition| definition.previews.get(preview_id))
    }

    /// Returns every bound definition for same-catalog resolution.
    #[must_use]
    pub(super) fn definitions(&self) -> &BTreeMap<String, ActionDefinition> {
        &self.definitions
    }

    /// Returns an independent reader over this catalog under one visibility scope.
    #[must_use]
    pub fn reader(&self, scope: ActionVisibilityScope) -> ActionReader<'_> {
        ActionReader::new(self, scope)
    }
}
