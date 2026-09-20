// SPDX-License-Identifier: MIT

//! Shared legal-action fixture: one available action with a dead and an invalid target, one action
//! refused for an unaffordable cost, one refused for full capacity, one that still requires a
//! selection, one disabled by its mode, one visible only in the owner scope, and one hidden.

use super::*;

/// Manifest for the shared fixture.
pub fn fixture_manifest() -> ContentManifest {
    manifest(&FIXTURE_IDS)
}

/// One cost contributor with both amounts stated.
pub fn cost(
    cost_id: &str,
    kind: ActionCostKind,
    required: i64,
    available: i64,
) -> ActionCostContributor {
    ActionCostContributor {
        cost_id: cost_id.to_owned(),
        label: text(cost_id),
        kind,
        unit: text("points"),
        resource: none(),
        required: ActionField::available(required),
        available: ActionField::available(available),
        affordable: available >= required,
        references: Vec::new(),
    }
}

/// One restriction the action currently admits.
pub fn satisfied(restriction_id: &str, kind: ActionRestrictionKind) -> ActionTargetRestriction {
    ActionTargetRestriction {
        restriction_id: restriction_id.to_owned(),
        label: text(restriction_id),
        kind,
        target: none(),
        satisfied: true,
        reason: none(),
        references: Vec::new(),
    }
}

/// One restriction the action currently refuses for an explicit reason.
pub fn blocking(
    restriction_id: &str,
    kind: ActionRestrictionKind,
    reason: ActionRefusalReason,
) -> ActionTargetRestriction {
    ActionTargetRestriction {
        restriction_id: restriction_id.to_owned(),
        label: text(restriction_id),
        kind,
        target: none(),
        satisfied: false,
        reason: ActionField::available(reason),
        references: Vec::new(),
    }
}

/// One observed target the host currently accepts.
pub fn target(
    target_id: &str,
    kind: ActionTargetKind,
    definition: ActionSemanticReference,
) -> ActionTargetInput {
    ActionTargetInput {
        target_id: target_id.to_owned(),
        label: text(target_id),
        kind,
        definition,
        eligibility: eligible(),
        detail: ActionField::available(target_id.to_owned()),
        visibility: ActionVisibility::Visible,
        evidence: ActionEvidence::SourceDerived,
        references: Vec::new(),
    }
}

/// One observed target the host currently refuses for an explicit reason.
pub fn refused_target(
    target_id: &str,
    kind: ActionTargetKind,
    definition: ActionSemanticReference,
    reason: ActionRefusalReason,
    reason_text: &str,
) -> ActionTargetInput {
    ActionTargetInput {
        eligibility: refused(reason, reason_text),
        ..target(target_id, kind, definition)
    }
}

/// One target-specific consequence with both sides stated.
pub fn change(change_id: &str, before: &str, after: &str, delta: i64) -> ActionPreviewChange {
    ActionPreviewChange {
        change_id: change_id.to_owned(),
        label: text(change_id),
        kind: ActionEffectKind::DamageDealt,
        unit: text("health"),
        target: none(),
        before: ActionField::available(before.to_owned()),
        after: ActionField::available(after.to_owned()),
        delta: ActionField::available(delta),
        references: Vec::new(),
    }
}

pub fn assumption(assumption_id: &str) -> ActionPreviewAssumption {
    ActionPreviewAssumption {
        assumption_id: assumption_id.to_owned(),
        label: text(assumption_id),
        references: Vec::new(),
    }
}

pub fn omission(omission_id: &str, kind: ActionOmissionKind) -> ActionPreviewOmission {
    ActionPreviewOmission {
        omission_id: omission_id.to_owned(),
        kind,
        label: text(omission_id),
        references: Vec::new(),
    }
}

pub fn selection_requirement(
    requirement_id: &str,
    selection: ActionSemanticReference,
    required: u32,
    maximum: Option<u32>,
) -> ActionPreviewSelection {
    ActionPreviewSelection {
        requirement_id: requirement_id.to_owned(),
        label: text(requirement_id),
        selection,
        required,
        maximum: match maximum {
            Some(maximum) => ActionField::available(maximum),
            None => none(),
        },
        references: Vec::new(),
    }
}

/// Base definition with no cost, restriction, target, coverage, or preview.
pub fn definition(action_id: &str, kind: ActionKind) -> ActionDefinitionInput {
    ActionDefinitionInput {
        action_id: action_id.to_owned(),
        parent: ActionParentOperation::CombatTurn,
        kind,
        label: text(action_id),
        visibility: ActionVisibility::Visible,
        evidence: ActionEvidence::SourceDerived,
        instance_generation: GENERATION,
        eligibility: eligible(),
        costs: Vec::new(),
        restrictions: Vec::new(),
        observed_targets: Vec::new(),
        targets: Vec::new(),
        coverage: Vec::new(),
        preview_classes: Vec::new(),
        previews: Vec::new(),
        instance: none(),
        references: Vec::new(),
    }
}

/// One declared preview with only its identity, class, and target set.
pub fn preview(
    preview_id: &str,
    class: ActionPreviewClass,
    target_id: Option<&str>,
) -> ActionPreviewInput {
    ActionPreviewInput {
        preview_id: preview_id.to_owned(),
        label: text(preview_id),
        class,
        target: match target_id {
            Some(target_id) => ActionField::available(target_id.to_owned()),
            None => none(),
        },
        affected: Vec::new(),
        changes: Vec::new(),
        statuses: Vec::new(),
        movements: Vec::new(),
        selections: Vec::new(),
        assumptions: Vec::new(),
        omissions: Vec::new(),
        provenance: rules(),
        evidence: ActionEvidence::SourceDerived,
        references: Vec::new(),
    }
}

/// Available card play with two living targets, a dead target, an invalid target, a covered target,
/// one exact target-specific preview, and one conditional default preview.
pub fn alpha() -> ActionDefinitionInput {
    let mut energy = cost("cost.energy", ActionCostKind::Energy, 1, 1);
    energy.resource =
        ActionField::available(reference(ActionReferenceKind::Resource, "resource.energy"));
    let slime = reference(ActionReferenceKind::Enemy, "enemy.slime");
    let mut exact = preview(
        "preview.alpha.slime",
        ActionPreviewClass::DeterministicExact,
        Some("target.slime"),
    );
    let mut damage = change("change.damage", "6", "12", 6);
    damage.target = ActionField::available(slime.clone());
    damage.references = vec![reference(
        ActionReferenceKind::Effect,
        "action-effect.damage",
    )];
    exact.affected = vec!["target.slime".to_owned()];
    exact.changes = vec![damage];
    let mut conditional = preview(
        "preview.alpha.default",
        ActionPreviewClass::Conditional,
        None,
    );
    conditional.affected = vec!["target.slime".to_owned()];
    conditional.changes = vec![change("change.default", "6", "9", 3)];
    conditional.assumptions = vec![assumption("assumption.living")];
    ActionDefinitionInput {
        observed_targets: vec![
            "target.slime".to_owned(),
            "target.cultist".to_owned(),
            "target.corpse".to_owned(),
            "target.tome".to_owned(),
            "target.mystery".to_owned(),
        ],
        targets: vec![
            target("target.slime", ActionTargetKind::Enemy, slime),
            target(
                "target.cultist",
                ActionTargetKind::Enemy,
                reference(ActionReferenceKind::Enemy, "enemy.cultist"),
            ),
            refused_target(
                "target.corpse",
                ActionTargetKind::Enemy,
                reference(ActionReferenceKind::Enemy, "enemy.corpse"),
                ActionRefusalReason::DeadTarget,
                "that enemy is no longer alive",
            ),
            refused_target(
                "target.tome",
                ActionTargetKind::Card,
                reference(ActionReferenceKind::Card, "card.strike"),
                ActionRefusalReason::InvalidTarget,
                "a card is not a legal target for this action",
            ),
        ],
        coverage: vec![ActionCoverageRecord {
            target_id: "target.mystery".to_owned(),
            state: ActionCoverageState::Unsupported,
            reason: text("no typed record for this target"),
        }],
        preview_classes: vec![
            ActionPreviewClass::DeterministicExact,
            ActionPreviewClass::Conditional,
            ActionPreviewClass::Partial,
            ActionPreviewClass::Unavailable,
        ],
        previews: vec![exact, conditional],
        costs: vec![energy],
        restrictions: vec![satisfied(
            "restriction.living",
            ActionRestrictionKind::RequiresLivingTarget,
        )],
        ..definition("action.alpha", ActionKind::PlayCard)
    }
}

/// Potion refused because its charge cannot be paid, with a partial preview that names its omission.
pub fn beta() -> ActionDefinitionInput {
    let mut unaffordable = cost("cost.charge", ActionCostKind::PotionCharge, 2, 1);
    unaffordable.resource = ActionField::available(reference(
        ActionReferenceKind::Resource,
        "resource.potion_charge",
    ));
    unaffordable.references = vec![reference(ActionReferenceKind::Potion, "potion.fire")];
    let mut partial = preview("preview.beta.partial", ActionPreviewClass::Partial, None);
    partial.changes = vec![change("change.burn", "0", "5", 5)];
    partial.omissions = vec![omission(
        "omission.random",
        ActionOmissionKind::RandomOutcome,
    )];
    ActionDefinitionInput {
        eligibility: refused(
            ActionRefusalReason::InsufficientResource,
            "not enough potion charges for this action",
        ),
        costs: vec![unaffordable],
        preview_classes: vec![ActionPreviewClass::Partial],
        previews: vec![partial],
        ..definition("action.beta", ActionKind::UsePotion)
    }
}

/// End turn refused because its bounded destination has no free capacity.
pub fn gamma() -> ActionDefinitionInput {
    let mut capacity = blocking(
        "restriction.capacity",
        ActionRestrictionKind::RequiresFreeCapacity,
        ActionRefusalReason::FullCapacity,
    );
    capacity.target = ActionField::available("target.discard".to_owned());
    ActionDefinitionInput {
        eligibility: refused(
            ActionRefusalReason::FullCapacity,
            "the discard pile has no free capacity",
        ),
        observed_targets: vec!["target.discard".to_owned()],
        targets: vec![target(
            "target.discard",
            ActionTargetKind::Destination,
            reference(ActionReferenceKind::Destination, "target.discard"),
        )],
        restrictions: vec![capacity],
        preview_classes: vec![ActionPreviewClass::Unavailable],
        previews: vec![preview(
            "preview.gamma.none",
            ActionPreviewClass::Unavailable,
            None,
        )],
        ..definition("action.gamma", ActionKind::EndTurn)
    }
}

/// Event choice refused until the caller answers the selection it still requires.
pub fn delta() -> ActionDefinitionInput {
    let mut exact = preview(
        "preview.delta.default",
        ActionPreviewClass::DeterministicExact,
        None,
    );
    exact.selections = vec![selection_requirement(
        "requirement.card",
        reference(ActionReferenceKind::Selection, "selection.alpha"),
        1,
        Some(1),
    )];
    ActionDefinitionInput {
        parent: ActionParentOperation::EventChoice,
        eligibility: refused(
            ActionRefusalReason::SelectionRequired,
            "choose a card before answering this event",
        ),
        restrictions: vec![blocking(
            "restriction.selection",
            ActionRestrictionKind::RequiresSelection,
            ActionRefusalReason::SelectionRequired,
        )],
        preview_classes: vec![ActionPreviewClass::DeterministicExact],
        previews: vec![exact],
        ..definition("action.delta", ActionKind::EventChoice)
    }
}

/// Rest option disabled by its mode, with a cost the source never observed.
pub fn epsilon() -> ActionDefinitionInput {
    let mut health = cost("cost.health", ActionCostKind::Health, 5, 5);
    health.available = ActionField::unavailable(not_observed());
    health.affordable = false;
    ActionDefinitionInput {
        parent: ActionParentOperation::RestSite,
        eligibility: refused(
            ActionRefusalReason::DisabledOption,
            "this rest option is disabled in the current mode",
        ),
        costs: vec![health],
        restrictions: vec![blocking(
            "restriction.mode",
            ActionRestrictionKind::Custom("mode.gated".to_owned()),
            ActionRefusalReason::DisabledOption,
        )],
        ..definition("action.epsilon", ActionKind::Rest)
    }
}

/// Relic use visible only in the owner scope.
pub fn zeta() -> ActionDefinitionInput {
    ActionDefinitionInput {
        visibility: ActionVisibility::OwnerOnly,
        ..definition("action.zeta", ActionKind::UseRelic)
    }
}

/// Hidden shop purchase whose label the source must not reveal.
pub fn eta() -> ActionDefinitionInput {
    ActionDefinitionInput {
        parent: ActionParentOperation::ShopVisit,
        label: withheld_text(),
        visibility: ActionVisibility::Hidden,
        ..definition("action.eta", ActionKind::ShopPurchase)
    }
}

pub fn fixture_definitions() -> Vec<ActionDefinitionInput> {
    vec![alpha(), beta(), gamma(), delta(), epsilon(), zeta(), eta()]
}

pub fn fixture() -> (ContentManifest, ActionCatalog) {
    let manifest = fixture_manifest();
    let catalog = produce(&manifest, snapshot(&manifest, fixture_definitions())).expect("catalog");
    (manifest, catalog)
}
