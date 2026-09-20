// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/action_reference.rs"]
mod support;

use sts2_game_mod::{
    ACTION_MAX_CHANGES, ACTION_MAX_DEFINITION_BYTES, ACTION_MAX_DEFINITIONS,
    ACTION_MAX_IDENTITY_BYTES, ACTION_MAX_PREVIEWS, ACTION_MAX_TEXT_BYTES,
    ACTION_REFERENCE_ENEMY_KIND, ActionCatalogProducer, ActionDefinitionInput, ActionEligibility,
    ActionEligibilityState, ActionError, ActionField, ActionKind, ActionOmissionKind,
    ActionPreviewClass, ActionPreviewProvenance, ActionReferenceKind, ActionRefusalReason,
    ActionRestrictionKind, ActionSemanticReference, ActionTargetRestriction, ActionVisibility,
    FailingActionSource, FixtureActionFailure,
};
use support::{
    GENERATION, alpha, blocking, change, definition, fixture_manifest, instance, manifest,
    manifest_of, no_text, none, not_observed, omission, preview, produce, refused, snapshot, text,
};

/// Produces one catalog and returns the single refusal the producer reported.
fn reject(definitions: Vec<ActionDefinitionInput>, ids: &[&str]) -> ActionError {
    let manifest = manifest(ids);
    produce(&manifest, snapshot(&manifest, definitions)).expect_err("refused")
}

/// Base fixture action with one field edited, so each case isolates one declaration.
fn alpha_with(edit: impl FnOnce(&mut ActionDefinitionInput)) -> ActionDefinitionInput {
    let mut input = alpha();
    edit(&mut input);
    input
}

#[test]
fn a_refusal_token_without_the_state_behind_it_is_rejected() {
    for reason in [
        ActionRefusalReason::InsufficientResource,
        ActionRefusalReason::InvalidTarget,
        ActionRefusalReason::DeadTarget,
        ActionRefusalReason::FullCapacity,
        ActionRefusalReason::SelectionRequired,
        ActionRefusalReason::DisabledOption,
    ] {
        let mut input = definition("action.alpha", ActionKind::PlayCard);
        input.eligibility = refused(reason.clone(), "the host refuses this action");
        assert_eq!(
            reject(vec![input], &["action.alpha"]),
            ActionError::InvalidRefusalSupport {
                action_id: "action.alpha".to_owned(),
                reason,
            }
        );
    }
}

#[test]
fn a_refusal_that_states_no_reason_and_no_text_is_rejected() {
    let input = alpha_with(|input| {
        input.eligibility = ActionEligibility {
            state: ActionEligibilityState::Unavailable,
            reason: none(),
            reason_text: no_text(),
            references: Vec::new(),
        };
    });
    assert_eq!(
        reject(vec![input], &["action.alpha"]),
        ActionError::InvalidEligibility {
            action_id: "action.alpha".to_owned(),
        }
    );
}

#[test]
fn an_offered_target_that_reports_a_refusal_is_rejected() {
    let input = alpha_with(|input| {
        input.targets[0].eligibility = ActionEligibility {
            state: ActionEligibilityState::Available,
            reason: ActionField::available(ActionRefusalReason::InvalidTarget),
            reason_text: text("the host refuses this target"),
            references: Vec::new(),
        };
    });
    assert_eq!(
        reject(vec![input], &["action.alpha"]),
        ActionError::InvalidTargetEligibility {
            action_id: "action.alpha".to_owned(),
            target_id: "target.slime".to_owned(),
        }
    );
}

#[test]
fn a_visible_subject_with_a_withheld_refusal_is_rejected() {
    let input = alpha_with(|input| {
        input.eligibility = refused(ActionRefusalReason::Withheld, "no reason is published");
    });
    assert_eq!(
        reject(vec![input], &["action.alpha"]),
        ActionError::InvalidEligibility {
            action_id: "action.alpha".to_owned(),
        }
    );
}

#[test]
fn an_affordable_claim_over_an_unobserved_amount_is_rejected() {
    let input = alpha_with(|input| {
        input.costs[0].available = ActionField::unavailable(not_observed());
    });
    assert_eq!(
        reject(vec![input], &["action.alpha"]),
        ActionError::InvalidCostContributor {
            action_id: "action.alpha".to_owned(),
            cost_id: "cost.energy".to_owned(),
        }
    );
}

#[test]
fn a_satisfied_restriction_that_states_a_reason_is_rejected() {
    let input = alpha_with(|input| {
        let mut restriction = input.restrictions[0].clone();
        restriction.reason = ActionField::available(ActionRefusalReason::DeadTarget);
        input.restrictions[0] = restriction;
    });
    assert_eq!(
        reject(vec![input], &["action.alpha"]),
        ActionError::InvalidRestriction {
            action_id: "action.alpha".to_owned(),
            restriction_id: "restriction.living".to_owned(),
        }
    );
}

#[test]
fn an_approximated_preview_is_refused_by_name() {
    let input = alpha_with(|input| {
        input.previews[0].provenance = ActionPreviewProvenance::SimulatedFromPendingState;
    });
    assert_eq!(
        reject(vec![input], &["action.alpha"]),
        ActionError::SimulatedPreview {
            action_id: "action.alpha".to_owned(),
            preview_id: "preview.alpha.slime".to_owned(),
        }
    );
}

#[test]
fn a_random_chain_reported_as_exact_is_refused_by_name() {
    let input = alpha_with(|input| {
        input.previews[0].omissions = vec![omission(
            "omission.random",
            ActionOmissionKind::RandomOutcome,
        )];
    });
    assert_eq!(
        reject(vec![input], &["action.alpha"]),
        ActionError::UncertainPreview {
            action_id: "action.alpha".to_owned(),
            preview_id: "preview.alpha.slime".to_owned(),
            class: ActionPreviewClass::DeterministicExact,
        }
    );
}

#[test]
fn a_preview_change_that_changes_nothing_is_rejected() {
    let input = alpha_with(|input| {
        input.previews[0].changes[0].delta = ActionField::available(0);
    });
    assert_eq!(
        reject(vec![input], &["action.alpha"]),
        ActionError::InvalidPreviewChange {
            action_id: "action.alpha".to_owned(),
            preview_id: "preview.alpha.slime".to_owned(),
            change_id: "change.damage".to_owned(),
        }
    );
}

#[test]
fn a_preview_class_the_definition_does_not_declare_is_rejected() {
    let input = alpha_with(|input| {
        input.previews[1].class = ActionPreviewClass::RangeDistribution;
    });
    assert_eq!(
        reject(vec![input], &["action.alpha"]),
        ActionError::UndeclaredPreviewClass {
            action_id: "action.alpha".to_owned(),
            preview_id: "preview.alpha.default".to_owned(),
            class: ActionPreviewClass::RangeDistribution,
        }
    );
}

#[test]
fn a_preview_naming_a_target_the_action_does_not_present_is_rejected() {
    let input = alpha_with(|input| {
        input.previews[0].target = ActionField::available("target.ghost".to_owned());
    });
    assert_eq!(
        reject(vec![input], &["action.alpha"]),
        ActionError::DanglingTarget {
            action_id: "action.alpha".to_owned(),
            target_id: "target.ghost".to_owned(),
        }
    );
}

#[test]
fn a_host_reported_target_with_no_record_and_no_coverage_is_rejected() {
    let input = alpha_with(|input| {
        input.observed_targets.push("target.ghost".to_owned());
    });
    assert_eq!(
        reject(vec![input], &["action.alpha"]),
        ActionError::UncoveredTarget {
            action_id: "action.alpha".to_owned(),
            target_id: "target.ghost".to_owned(),
        }
    );
}

#[test]
fn a_static_definition_carrying_a_live_instance_is_rejected() {
    let input = alpha_with(|input| {
        input.instance = ActionField::available(instance("action-instance.alpha", GENERATION));
    });
    assert_eq!(
        reject(vec![input], &["action.alpha"]),
        ActionError::InvalidInstanceReference {
            action_id: "action.alpha".to_owned(),
        }
    );
}

#[test]
fn a_cross_generation_action_reference_is_rejected_as_stale() {
    let mut referring = definition("action.alpha", ActionKind::PlayCard);
    referring.instance_generation = GENERATION + 1;
    referring.references = vec![ActionSemanticReference {
        kind: ActionReferenceKind::Action,
        id: "action.other".to_owned(),
        label: text("action.other"),
    }];
    let other = definition("action.other", ActionKind::PlayCard);
    assert_eq!(
        reject(vec![referring, other], &["action.alpha", "action.other"]),
        ActionError::StaleActionReference {
            action_id: "action.alpha".to_owned(),
            referenced: GENERATION,
            current: GENERATION + 1,
        }
    );
}

#[test]
fn a_reference_whose_target_is_more_restricted_is_rejected_without_naming_it() {
    let mut visible = definition("action.alpha", ActionKind::PlayCard);
    visible.references = vec![ActionSemanticReference {
        kind: ActionReferenceKind::Action,
        id: "action.hidden".to_owned(),
        label: text("action.hidden"),
    }];
    let mut hidden = definition("action.hidden", ActionKind::PlayCard);
    hidden.visibility = ActionVisibility::Hidden;
    assert_eq!(
        reject(vec![visible, hidden], &["action.alpha", "action.hidden"]),
        ActionError::HiddenReferenceLeak {
            action_id: "action.alpha".to_owned(),
            reference_kind: ActionReferenceKind::Action,
        }
    );
}

#[test]
fn a_destination_reference_naming_nothing_presented_is_rejected() {
    let mut input = definition("action.alpha", ActionKind::PlayCard);
    input.eligibility = refused(ActionRefusalReason::FullCapacity, "no free capacity");
    input.restrictions = vec![ActionTargetRestriction {
        target: ActionField::available("target.discard".to_owned()),
        ..blocking(
            "restriction.capacity",
            ActionRestrictionKind::RequiresFreeCapacity,
            ActionRefusalReason::FullCapacity,
        )
    }];
    assert_eq!(
        reject(vec![input], &["action.alpha"]),
        ActionError::DanglingTarget {
            action_id: "action.alpha".to_owned(),
            target_id: "target.discard".to_owned(),
        }
    );
}

#[test]
fn a_manifest_that_does_not_inventory_the_action_family_is_rejected() {
    let manifest = manifest_of(&[(ACTION_REFERENCE_ENEMY_KIND, "enemy.slime")]);
    assert_eq!(
        produce(&manifest, snapshot(&manifest, Vec::new())).expect_err("missing family"),
        ActionError::MissingFamily
    );
}

#[test]
fn a_source_snapshot_that_disagrees_with_the_manifest_count_is_rejected() {
    let manifest = manifest(&["action.alpha", "action.beta"]);
    assert_eq!(
        produce(&manifest, snapshot(&manifest, vec![alpha()])).expect_err("count"),
        ActionError::FamilyCountMismatch
    );
}

#[test]
fn a_failing_source_is_sanitized_into_one_explicit_error() {
    let manifest = fixture_manifest();
    for (failure, expected) in [
        (
            FixtureActionFailure::NoActiveSource,
            ActionError::NoActiveSource,
        ),
        (
            FixtureActionFailure::AccessDenied,
            ActionError::SourceAccessDenied,
        ),
        (
            FixtureActionFailure::Malformed,
            ActionError::MalformedSource,
        ),
    ] {
        let produced =
            ActionCatalogProducer::new().produce(&manifest, &FailingActionSource(failure));
        assert_eq!(produced.expect_err("failed source"), expected);
    }
}

#[test]
fn an_action_identity_past_its_byte_bound_is_refused_by_field() {
    let oversized = definition(
        &"a".repeat(ACTION_MAX_IDENTITY_BYTES + 1),
        ActionKind::PlayCard,
    );
    assert_eq!(
        reject(vec![oversized], &["action.alpha"]),
        ActionError::InvalidInput("action_id")
    );
}

#[test]
fn a_label_past_the_text_bound_is_refused_by_field() {
    let oversized = alpha_with(|input| {
        input.label = text(&"l".repeat(ACTION_MAX_TEXT_BYTES + 1));
    });
    assert_eq!(
        reject(vec![oversized], &["action.alpha"]),
        ActionError::InvalidInput("label")
    );
}

#[test]
fn a_preview_past_a_local_collection_bound_is_refused_by_name() {
    let mut oversized = alpha();
    oversized.previews[0].changes = (0..=ACTION_MAX_CHANGES)
        .map(|index| change(&format!("change.{index}"), "6", "12", 6))
        .collect();
    assert_eq!(
        reject(vec![oversized], &["action.alpha"]),
        ActionError::InvalidPreview {
            action_id: "action.alpha".to_owned(),
            preview_id: "preview.alpha.slime".to_owned(),
        }
    );
}

#[test]
fn more_definitions_than_the_local_bound_are_refused() {
    let ids: Vec<String> = (0..=ACTION_MAX_DEFINITIONS)
        .map(|index| format!("action.overflow{index}"))
        .collect();
    let borrows: Vec<&str> = ids.iter().map(String::as_str).collect();
    let definitions = ids
        .iter()
        .map(|action_id| definition(action_id, ActionKind::PlayCard))
        .collect();
    assert_eq!(
        reject(definitions, &borrows),
        ActionError::InvalidInput("definitions")
    );
}

#[test]
fn an_aggregate_record_bound_is_enforced() {
    let label = "p".repeat(ACTION_MAX_TEXT_BYTES);
    let mut oversized = alpha();
    oversized.previews = (0..ACTION_MAX_PREVIEWS)
        .map(|index| {
            let mut record = preview(
                &format!("preview.alpha.{index}"),
                ActionPreviewClass::Conditional,
                None,
            );
            record.label = text(&label);
            record
        })
        .collect();
    let error = reject(vec![oversized], &["action.alpha"]);
    assert!(
        matches!(
            error,
            ActionError::DefinitionTooLarge { limit, .. } if limit == ACTION_MAX_DEFINITION_BYTES
        ),
        "an oversized record is refused rather than truncated: {error:?}"
    );
}
