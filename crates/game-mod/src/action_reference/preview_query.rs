// SPDX-License-Identifier: MIT

use super::{
    definition::ActionDefinition,
    error::ActionError,
    field::{ActionField, ActionUnavailableReason},
    kind::{ActionDispatchAuthority, ActionDispatchPrerequisite, ActionPreviewClass},
    model::{
        ActionCatalogBinding, ActionFrameReference, ActionReference, ActionSnapshotReference,
        ActionTargetReference, ActionVisibility, ActionVisibilityScope,
    },
    preview::ActionPreview,
    projection::{project_preview, visible_preview, visible_target},
};

/// Bounded preview request for one static legal-action frame.
///
/// The query separates the static frame the caller observed from the live fence it may or may not
/// hold. A query that carries no fence is a static question; a query that carries a live run,
/// instance, epoch, and snapshot fence must carry the transient instance identity too. A fence that
/// arrives without one, or an instance identity that arrives without a fence, is refused rather
/// than reconciled against a half-known frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionPreviewQuery {
    /// Exact static legal-action frame the caller is previewing.
    pub frame: ActionFrameReference,
    /// Live run/instance/epoch/snapshot fence, or an explicit non-value for a static query.
    pub fence: ActionField<ActionSnapshotReference>,
    /// Transient live action-instance identity, or an explicit non-value for a static query.
    pub instance: ActionField<String>,
    /// Target identity to preview, or an explicit non-value for an untargeted action.
    pub target_id: ActionField<String>,
    /// Visibility scope.
    pub scope: ActionVisibilityScope,
}

/// Result of one bounded preview request.
///
/// The result states what the source can document and withholds what it cannot. A subject the
/// catalog cannot resolve at all is an error; a subject the catalog resolves but the source does not
/// describe is answered with an explicitly unavailable-class result, so an absent preview is never
/// published as an empty consequence set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionPreviewResult {
    /// Catalog witness for every value.
    pub binding: ActionCatalogBinding,
    /// Exact static legal-action reference.
    pub reference: ActionReference,
    /// Legal-action generation this result is bound to.
    pub frame_generation: u64,
    /// Resolved target the preview applies to, or an explicit non-value for an untargeted action.
    pub target: ActionField<ActionTargetReference>,
    /// The declared preview, or an explicit non-value when the source describes none.
    pub preview: ActionField<ActionPreview>,
    /// Classification of how confidently the consequences are described.
    pub class: ActionPreviewClass,
    /// Whether any part of the outcome is withheld from this result.
    pub outcome_withheld: bool,
    /// Number of stated consequences across every consequence list.
    pub consequence_count: usize,
    /// Number of interactions the preview explicitly does not describe.
    pub omission_count: usize,
    /// Dispatch authority this preview carries.
    pub authority: ActionDispatchAuthority,
    /// Validation a caller must still perform before dispatching.
    pub required_prerequisites: Vec<ActionDispatchPrerequisite>,
    /// Visibility of the definition.
    pub visibility: ActionVisibility,
}

impl ActionPreviewResult {
    /// Returns whether this result states at least one consequence.
    #[must_use]
    pub const fn states_consequence(&self) -> bool {
        self.consequence_count > 0
    }

    /// Returns whether this result admits it does not describe every interaction.
    #[must_use]
    pub const fn is_incomplete(&self) -> bool {
        self.outcome_withheld
    }

    /// Returns whether the caller must re-validate before dispatching.
    ///
    /// Always true: a preview never authorizes dispatch.
    #[must_use]
    pub fn requires_fresh_validation(&self) -> bool {
        !self.required_prerequisites.is_empty()
    }
}

/// Every prerequisite a preview leaves to the caller.
fn dispatch_prerequisites() -> Vec<ActionDispatchPrerequisite> {
    vec![
        ActionDispatchPrerequisite::FreshLegalCatalog,
        ActionDispatchPrerequisite::FreshEpoch,
        ActionDispatchPrerequisite::FreshTargetValidation,
    ]
}

/// Reconciles the static frame against the live fence the query carries.
///
/// A query is either wholly static or wholly live. Declaring it static with an explicit
/// `NotApplicable` while still supplying a fence is a contradiction and is refused as an unexpected
/// fence; supplying either half of a live fence without the other is refused as a missing one. A
/// fence the source could not observe at all is a missing fence rather than a silently accepted
/// static question.
fn validate_fences(
    definition: &ActionDefinition,
    query: &ActionPreviewQuery,
) -> Result<(), ActionError> {
    let action_id = definition.reference.action_id.clone();
    let static_fence = query.fence.reason() == Some(ActionUnavailableReason::NotApplicable);
    let static_instance = query.instance.reason() == Some(ActionUnavailableReason::NotApplicable);
    if static_instance && !static_fence && query.fence.value().is_none() {
        return Err(ActionError::UnexpectedLiveFence);
    }
    match (query.fence.value(), query.instance.value()) {
        (Some(fence), Some(instance)) => {
            let incomplete = instance.is_empty()
                || fence.run_id.is_empty()
                || fence.instance_id.is_empty()
                || fence.snapshot_id.is_empty();
            if incomplete {
                return Err(ActionError::MissingLiveFence { action_id });
            }
            Ok(())
        }
        (None, None) if static_fence && static_instance => Ok(()),
        _ => Err(ActionError::MissingLiveFence { action_id }),
    }
}

/// Builds one bounded preview result for a resolved definition.
pub(super) fn preview_of(
    definition: &ActionDefinition,
    query: &ActionPreviewQuery,
) -> Result<ActionPreviewResult, ActionError> {
    validate_fences(definition, query)?;
    let action_id = definition.reference.action_id.clone();
    let requested = query.target_id.clone();
    let (subject, target) = match requested.value() {
        Some(target_id) => {
            let observed = definition.targets.get(target_id).ok_or_else(|| {
                ActionError::NoSupportedPreview {
                    action_id: action_id.clone(),
                }
            })?;
            if !visible_target(observed, query.scope) {
                return Err(ActionError::ExcludedByScope);
            }
            (
                Some(target_id.as_str()),
                ActionField::available(observed.reference.clone()),
            )
        }
        None => match requested.reason() {
            Some(ActionUnavailableReason::NotApplicable) => (
                None,
                ActionField::unavailable(ActionUnavailableReason::NotApplicable),
            ),
            Some(reason) => return Err(ActionError::UnavailableField(reason)),
            None => (
                None,
                ActionField::unavailable(ActionUnavailableReason::NotApplicable),
            ),
        },
    };
    let mut result = ActionPreviewResult {
        binding: definition.reference.catalog.clone(),
        reference: definition.reference.clone(),
        frame_generation: definition.instance_generation,
        target,
        preview: ActionField::unavailable(ActionUnavailableReason::Undescribed),
        class: ActionPreviewClass::Unavailable,
        outcome_withheld: true,
        consequence_count: 0,
        omission_count: 0,
        authority: ActionDispatchAuthority::NotGranted,
        required_prerequisites: dispatch_prerequisites(),
        visibility: definition.visibility,
    };
    let Some(preview) = definition.preview_for(subject) else {
        return Ok(result);
    };
    if !visible_preview(preview, definition, query.scope) {
        return Err(ActionError::ExcludedByScope);
    }
    let projected = project_preview(preview, definition, query.scope);
    result.preview = ActionField::available(projected);
    result.class = preview.class;
    result.outcome_withheld = preview.is_incomplete();
    result.consequence_count = preview.consequence_count();
    result.omission_count = preview.omissions.len();
    Ok(result)
}
