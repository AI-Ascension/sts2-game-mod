// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/action_reference.rs"]
mod support;

use sts2_game_mod::{
    ActionCatalog, ActionDispatchAuthority, ActionDispatchPrerequisite, ActionError, ActionField,
    ActionOmissionKind, ActionPreviewClass, ActionPreviewQuery, ActionPreviewResult,
    ActionUnavailableReason, ActionVisibilityScope,
};
use support::{GENERATION, fixture, frame, none};

/// Builds one bounded preview query for a fixture action under one scope.
fn query(
    catalog: &ActionCatalog,
    action_id: &str,
    target_id: Option<&str>,
    scope: ActionVisibilityScope,
) -> ActionPreviewQuery {
    ActionPreviewQuery {
        frame: frame(catalog, action_id, GENERATION),
        fence: none(),
        instance: none(),
        target_id: match target_id {
            Some(target_id) => ActionField::available(target_id.to_owned()),
            None => none(),
        },
        scope,
    }
}

/// Previews one fixture action from the public scope.
fn preview(
    catalog: &ActionCatalog,
    action_id: &str,
    target_id: Option<&str>,
) -> ActionPreviewResult {
    catalog
        .reader(ActionVisibilityScope::Public)
        .preview(&query(
            catalog,
            action_id,
            target_id,
            ActionVisibilityScope::Public,
        ))
        .expect("preview")
}

#[test]
fn an_exact_preview_states_one_target_specific_consequence_at_the_same_start() {
    let (_, catalog) = fixture();
    let result = preview(&catalog, "action.alpha", Some("target.slime"));
    assert_eq!(
        result
            .target
            .value()
            .map(|target| target.target_id.as_str()),
        Some("target.slime")
    );
    assert_eq!(result.class, ActionPreviewClass::DeterministicExact);
    assert!(result.states_consequence());
    assert_eq!(result.consequence_count, 1);
    assert_eq!(result.omission_count, 0);
    assert!(!result.outcome_withheld);
    assert!(!result.is_incomplete());
    let declared = result.preview.value().expect("declared preview");
    assert_eq!(declared.class, ActionPreviewClass::DeterministicExact);
    assert_eq!(declared.changes.len(), 1);
    let change = &declared.changes[0];
    assert_eq!(change.change_id, "change.damage");
    assert_eq!(change.before.value().map(String::as_str), Some("6"));
    assert_eq!(change.after.value().map(String::as_str), Some("12"));
    assert_eq!(change.delta.value(), Some(&6));
}

#[test]
fn a_refused_action_still_states_the_consequence_and_its_uncertainty() {
    let (_, catalog) = fixture();
    assert!(
        !catalog
            .definition("action.beta")
            .expect("definition")
            .is_available()
    );
    let result = preview(&catalog, "action.beta", None);
    assert_eq!(result.class, ActionPreviewClass::Partial);
    assert!(result.states_consequence());
    assert_eq!(result.consequence_count, 1);
    assert_eq!(result.omission_count, 1);
    assert!(result.outcome_withheld);
    assert!(result.is_incomplete());
    let declared = result.preview.value().expect("declared preview");
    assert_eq!(declared.omissions.len(), 1);
    assert_eq!(
        declared.omissions[0].kind,
        ActionOmissionKind::RandomOutcome
    );
}

#[test]
fn a_selection_requirement_is_stated_as_a_consequence_not_as_prose() {
    let (_, catalog) = fixture();
    let result = preview(&catalog, "action.delta", None);
    assert_eq!(result.class, ActionPreviewClass::DeterministicExact);
    assert_eq!(result.consequence_count, 1);
    let declared = result.preview.value().expect("declared preview");
    assert_eq!(declared.selections.len(), 1);
    assert_eq!(declared.selections[0].requirement_id, "requirement.card");
    assert_eq!(declared.selections[0].selection.id, "selection.alpha");
    assert_eq!(declared.selections[0].required, 1);
    assert_eq!(declared.selections[0].maximum.value(), Some(&1));
}

#[test]
fn a_subject_the_catalog_cannot_resolve_is_refused_rather_than_invented() {
    let (_, catalog) = fixture();
    for target_id in ["target.mystery", "target.ghost"] {
        let error = catalog
            .reader(ActionVisibilityScope::Public)
            .preview(&query(
                &catalog,
                "action.alpha",
                Some(target_id),
                ActionVisibilityScope::Public,
            ))
            .expect_err("unresolvable subject");
        assert_eq!(
            error,
            ActionError::NoSupportedPreview {
                action_id: "action.alpha".to_owned(),
            }
        );
    }
}

#[test]
fn an_undeclared_preview_is_reported_as_unavailable_rather_than_as_empty() {
    let (_, catalog) = fixture();
    for action_id in ["action.epsilon", "action.gamma"] {
        let result = preview(&catalog, action_id, None);
        assert_eq!(result.class, ActionPreviewClass::Unavailable);
        assert!(!result.states_consequence());
        assert_eq!(result.consequence_count, 0);
        assert_eq!(result.omission_count, 0);
        assert!(result.outcome_withheld);
        assert!(result.is_incomplete());
    }
    let absent = preview(&catalog, "action.epsilon", None);
    assert_eq!(
        absent.preview.reason(),
        Some(ActionUnavailableReason::Undescribed)
    );
    assert!(absent.preview.value().is_none());
}

#[test]
fn an_owner_only_subject_reports_the_same_unavailable_class_without_leaking() {
    let (_, catalog) = fixture();
    let result = catalog
        .reader(ActionVisibilityScope::Owner)
        .preview(&query(
            &catalog,
            "action.zeta",
            None,
            ActionVisibilityScope::Owner,
        ))
        .expect("preview");
    assert_eq!(result.class, ActionPreviewClass::Unavailable);
    assert_eq!(result.consequence_count, 0);
    assert_eq!(
        result.preview.reason(),
        Some(ActionUnavailableReason::Undescribed)
    );
    let public = catalog
        .reader(ActionVisibilityScope::Public)
        .preview(&query(
            &catalog,
            "action.zeta",
            None,
            ActionVisibilityScope::Public,
        ))
        .expect_err("owner only");
    assert_eq!(public, ActionError::ExcludedByScope);
}

#[test]
fn every_preview_result_still_requires_fresh_validation_and_grants_nothing() {
    let (_, catalog) = fixture();
    let expected = vec![
        ActionDispatchPrerequisite::FreshLegalCatalog,
        ActionDispatchPrerequisite::FreshEpoch,
        ActionDispatchPrerequisite::FreshTargetValidation,
    ];
    for action_id in [
        "action.alpha",
        "action.beta",
        "action.delta",
        "action.epsilon",
    ] {
        let result = preview(&catalog, action_id, None);
        assert_eq!(result.authority, ActionDispatchAuthority::NotGranted);
        assert_eq!(result.required_prerequisites, expected);
        assert!(result.requires_fresh_validation());
    }
}
