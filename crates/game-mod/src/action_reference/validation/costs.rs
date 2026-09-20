// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::super::{
    cost::ActionCostContributor,
    definition::ActionDefinitionInput,
    error::ActionError,
    field::ActionField,
    identity::{validate_identity, validate_text, validate_text_value},
    kind::{ActionEligibilityState, ActionRefusalReason, ActionRestrictionKind},
    model::{ACTION_MAX_COST_CONTRIBUTORS, ACTION_MAX_RESTRICTIONS},
};
use super::RefContext;

/// Validates every cost contributor of one action.
///
/// A contributor never claims affordability it cannot settle: when the required or available amount
/// was not observed, `affordable` must be `false` rather than an assumed yes.
pub(super) fn validate_costs(
    input: &ActionDefinitionInput,
    context: &RefContext<'_>,
) -> Result<(), ActionError> {
    if input.costs.len() > ACTION_MAX_COST_CONTRIBUTORS {
        return Err(ActionError::InvalidInput("costs"));
    }
    let mut seen = BTreeSet::new();
    for cost in &input.costs {
        validate_identity(&cost.cost_id, "cost_id")?;
        if !seen.insert(cost.cost_id.as_str()) {
            return Err(ActionError::InvalidInput("cost_id"));
        }
        validate_text_value(&cost.label, "label")?;
        validate_text_value(&cost.unit, "unit")?;
        if let super::super::kind::ActionCostKind::Custom(kind) = &cost.kind {
            validate_text(kind, "cost_kind")?;
        }
        if cost.affordable != settled_affordability(cost) {
            return Err(ActionError::InvalidCostContributor {
                action_id: input.action_id.clone(),
                cost_id: cost.cost_id.clone(),
            });
        }
        context.check_optional(&cost.resource, input.visibility)?;
        context.check_all(&cost.references, input.visibility)?;
    }
    Ok(())
}

/// Returns whether one contributor permits the action on the amounts it actually observed.
fn settled_affordability(cost: &ActionCostContributor) -> bool {
    match (cost.required.value(), cost.available.value()) {
        (Some(required), Some(available)) => *available >= *required,
        _ => false,
    }
}

/// Validates every target restriction of one action.
///
/// A restriction states whether it admits the action, so an unsatisfied restriction must name the
/// refusal it produces and a satisfied one must not.
pub(super) fn validate_restrictions(input: &ActionDefinitionInput) -> Result<(), ActionError> {
    if input.restrictions.len() > ACTION_MAX_RESTRICTIONS {
        return Err(ActionError::InvalidInput("restrictions"));
    }
    let mut seen = BTreeSet::new();
    for restriction in &input.restrictions {
        validate_identity(&restriction.restriction_id, "restriction_id")?;
        if !seen.insert(restriction.restriction_id.as_str()) {
            return Err(ActionError::InvalidInput("restriction_id"));
        }
        validate_text_value(&restriction.label, "label")?;
        if let ActionRestrictionKind::Custom(kind) = &restriction.kind {
            validate_text(kind, "restriction_kind")?;
        }
        if let ActionField::Available(target_id) = &restriction.target
            && !input.observed_targets.contains(target_id)
        {
            return Err(ActionError::DanglingTarget {
                action_id: input.action_id.clone(),
                target_id: target_id.clone(),
            });
        }
        let contradicts = restriction.satisfied == restriction.reason.is_available();
        if contradicts {
            return Err(ActionError::InvalidRestriction {
                action_id: input.action_id.clone(),
                restriction_id: restriction.restriction_id.clone(),
            });
        }
    }
    Ok(())
}

/// Requires an unavailable action to name a supported cause an agent can act on.
///
/// A refusal that only reports a token without the unaffordable contributor or the unsatisfied
/// restriction behind it is the defect this slice exists to remove, so the acceptance cases of
/// insufficient resource, invalid or dead target, full capacity, disabled option, and required
/// selection must each point at the state that produced them.
pub(super) fn validate_refusal_support(input: &ActionDefinitionInput) -> Result<(), ActionError> {
    let ActionField::Available(reason) = &input.eligibility.reason else {
        return Ok(());
    };
    if input.eligibility.state != ActionEligibilityState::Unavailable {
        return Ok(());
    }
    let supported = match reason {
        ActionRefusalReason::InsufficientResource => {
            !input.costs.iter().all(|cost| cost.affordable)
                || blocks(input, &[ActionRestrictionKind::RequiresAffordableCost])
        }
        ActionRefusalReason::InvalidTarget => {
            blocks(input, &[ActionRestrictionKind::RequiresValidTarget])
        }
        ActionRefusalReason::DeadTarget => {
            blocks(input, &[ActionRestrictionKind::RequiresLivingTarget])
        }
        ActionRefusalReason::FullCapacity => {
            blocks(input, &[ActionRestrictionKind::RequiresFreeCapacity])
        }
        ActionRefusalReason::SelectionRequired => {
            blocks(input, &[ActionRestrictionKind::RequiresSelection])
        }
        ActionRefusalReason::DisabledOption => {
            blocks(
                input,
                &[
                    ActionRestrictionKind::ForbidsTarget,
                    ActionRestrictionKind::Unknown,
                ],
            ) || input.restrictions.iter().any(|rule| {
                !rule.satisfied && matches!(rule.kind, ActionRestrictionKind::Custom(_))
            })
        }
        _ => true,
    };
    if !supported {
        return Err(ActionError::InvalidRefusalSupport {
            action_id: input.action_id.clone(),
            reason: reason.clone(),
        });
    }
    Ok(())
}

/// Returns whether one of the named restriction kinds currently blocks the action.
fn blocks(input: &ActionDefinitionInput, kinds: &[ActionRestrictionKind]) -> bool {
    input
        .restrictions
        .iter()
        .any(|rule| !rule.satisfied && kinds.contains(&rule.kind))
}
