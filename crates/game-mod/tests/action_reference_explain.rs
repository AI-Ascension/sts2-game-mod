// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/action_reference.rs"]
mod support;

use sts2_game_mod::{
    ActionAvailabilityExplanation, ActionAvailabilityQuery, ActionCatalog, ActionDefinitionInput,
    ActionDispatchAuthority, ActionKind, ActionRefusalReason, ActionRestrictionKind,
    ActionUnavailableReason, ActionVisibilityScope, ContentManifest,
};
use support::{
    GENERATION, blocking, definition, fixture, frame, manifest, produce, refused, snapshot,
};

/// Explains one fixture action from the public scope at the generation the fixture presents.
fn explain(catalog: &ActionCatalog, action_id: &str) -> ActionAvailabilityExplanation {
    catalog
        .reader(ActionVisibilityScope::Public)
        .explain(&ActionAvailabilityQuery {
            frame: frame(catalog, action_id, GENERATION),
            scope: ActionVisibilityScope::Public,
        })
        .expect("explanation")
}

/// Catalog of two actions the host refuses for the target it cannot accept.
fn refused_target_catalog() -> (ContentManifest, ActionCatalog) {
    let invalid = ActionDefinitionInput {
        eligibility: refused(
            ActionRefusalReason::InvalidTarget,
            "a card is not a legal target for this action",
        ),
        restrictions: vec![blocking(
            "restriction.valid",
            ActionRestrictionKind::RequiresValidTarget,
            ActionRefusalReason::InvalidTarget,
        )],
        ..definition("action.invalid", ActionKind::PlayCard)
    };
    let dead = ActionDefinitionInput {
        eligibility: refused(
            ActionRefusalReason::DeadTarget,
            "that enemy is no longer alive",
        ),
        restrictions: vec![blocking(
            "restriction.living",
            ActionRestrictionKind::RequiresLivingTarget,
            ActionRefusalReason::DeadTarget,
        )],
        ..definition("action.dead", ActionKind::PlayCard)
    };
    let manifest = manifest(&["action.dead", "action.invalid"]);
    let catalog = produce(&manifest, snapshot(&manifest, vec![dead, invalid])).expect("catalog");
    (manifest, catalog)
}

#[test]
fn an_available_action_explains_itself_and_grants_no_dispatch_authority() {
    let (_, catalog) = fixture();
    let explanation = explain(&catalog, "action.alpha");
    assert!(explanation.is_available());
    assert_eq!(explanation.refusal(), None);
    assert!(explanation.explains_refusal());
    assert!(explanation.blocking_costs.is_empty());
    assert!(explanation.blocking_restrictions.is_empty());
    assert!(!explanation.reason_text().is_available());
    assert_eq!(explanation.authority, ActionDispatchAuthority::NotGranted);
    assert_eq!(explanation.instance_generation, GENERATION);
}

#[test]
fn an_unaffordable_cost_explains_the_insufficient_resource_refusal() {
    let (_, catalog) = fixture();
    let explanation = explain(&catalog, "action.beta");
    assert!(!explanation.is_available());
    assert!(explanation.explains_refusal());
    assert_eq!(
        explanation.refusal(),
        Some(&ActionRefusalReason::InsufficientResource)
    );
    assert_eq!(
        explanation.reason_text().value(),
        Some("not enough potion charges for this action")
    );
    assert_eq!(explanation.blocking_costs, vec!["cost.charge".to_owned()]);
    assert!(explanation.blocking_restrictions.is_empty());
    let charge = explanation
        .blocking_cost("cost.charge")
        .expect("blocking cost");
    assert!(!charge.affordable);
    assert_eq!(charge.required.value(), Some(&2));
    assert_eq!(charge.available.value(), Some(&1));
    assert!(explanation.blocking_cost("cost.energy").is_none());
}

#[test]
fn a_full_destination_explains_the_full_capacity_refusal() {
    let (_, catalog) = fixture();
    let explanation = explain(&catalog, "action.gamma");
    assert_eq!(
        explanation.refusal(),
        Some(&ActionRefusalReason::FullCapacity)
    );
    assert_eq!(
        explanation.reason_text().value(),
        Some("the discard pile has no free capacity")
    );
    assert_eq!(
        explanation.blocking_restrictions,
        vec!["restriction.capacity".to_owned()]
    );
    assert!(explanation.blocking_costs.is_empty());
    let capacity = explanation
        .blocking_restriction("restriction.capacity")
        .expect("blocking restriction");
    assert_eq!(capacity.kind, ActionRestrictionKind::RequiresFreeCapacity);
    assert!(!capacity.satisfied);
    assert!(explanation.explains_refusal());
}

#[test]
fn a_required_selection_explains_the_selection_requirement() {
    let (_, catalog) = fixture();
    let explanation = explain(&catalog, "action.delta");
    assert_eq!(
        explanation.refusal(),
        Some(&ActionRefusalReason::SelectionRequired)
    );
    assert_eq!(
        explanation.reason_text().value(),
        Some("choose a card before answering this event")
    );
    let selection = explanation
        .blocking_restriction("restriction.selection")
        .expect("blocking restriction");
    assert_eq!(selection.kind, ActionRestrictionKind::RequiresSelection);
    assert!(explanation.explains_refusal());
}

#[test]
fn a_disabled_option_explains_itself_through_its_own_cost_and_mode_rule() {
    let (_, catalog) = fixture();
    let explanation = explain(&catalog, "action.epsilon");
    assert_eq!(
        explanation.refusal(),
        Some(&ActionRefusalReason::DisabledOption)
    );
    assert_eq!(
        explanation.reason_text().value(),
        Some("this rest option is disabled in the current mode")
    );
    let health = explanation
        .blocking_cost("cost.health")
        .expect("blocking cost");
    assert!(!health.affordable);
    assert_eq!(
        health.available.reason(),
        Some(ActionUnavailableReason::NotObserved)
    );
    let mode = explanation
        .blocking_restriction("restriction.mode")
        .expect("blocking restriction");
    assert_eq!(
        mode.kind,
        ActionRestrictionKind::Custom("mode.gated".to_owned())
    );
    assert!(explanation.explains_refusal());
}

#[test]
fn an_invalid_or_dead_target_refusal_names_the_rule_that_produces_it() {
    let (_, catalog) = refused_target_catalog();
    let invalid = explain(&catalog, "action.invalid");
    assert_eq!(invalid.refusal(), Some(&ActionRefusalReason::InvalidTarget));
    assert_eq!(
        invalid
            .blocking_restriction("restriction.valid")
            .expect("blocking restriction")
            .kind,
        ActionRestrictionKind::RequiresValidTarget
    );
    assert_eq!(
        invalid.reason_text().value(),
        Some("a card is not a legal target for this action")
    );
    assert!(invalid.explains_refusal());
    let dead = explain(&catalog, "action.dead");
    assert_eq!(dead.refusal(), Some(&ActionRefusalReason::DeadTarget));
    assert_eq!(
        dead.blocking_restriction("restriction.living")
            .expect("blocking restriction")
            .kind,
        ActionRestrictionKind::RequiresLivingTarget
    );
    assert_eq!(
        dead.reason_text().value(),
        Some("that enemy is no longer alive")
    );
    assert!(dead.explains_refusal());
}

#[test]
fn every_explanation_of_every_refusal_withholds_dispatch_authority() {
    let (_, catalog) = fixture();
    for action_id in [
        "action.alpha",
        "action.beta",
        "action.gamma",
        "action.delta",
        "action.epsilon",
    ] {
        assert_eq!(
            explain(&catalog, action_id).authority,
            ActionDispatchAuthority::NotGranted
        );
    }
    let (_, refused_catalog) = refused_target_catalog();
    for action_id in ["action.dead", "action.invalid"] {
        assert_eq!(
            explain(&refused_catalog, action_id).authority,
            ActionDispatchAuthority::NotGranted
        );
    }
}
