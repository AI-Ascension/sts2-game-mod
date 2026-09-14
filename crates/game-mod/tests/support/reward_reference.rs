// SPDX-License-Identifier: MIT

use sts2_game_mod::{
    ContentManifest, ContentUnlockState, REWARD_REFERENCE_ENTITY_KIND,
    REWARD_REFERENCE_PRODUCER_VERSION, RewardActionKind, RewardCatalog, RewardCatalogProducer,
    RewardCatalogSnapshot, RewardCatalogSource, RewardDefinitionReference, RewardEvidence,
    RewardField, RewardGenerationRule, RewardItemInput, RewardItemReference, RewardKind,
    RewardLegalAction, RewardModifier, RewardModifierKind, RewardNumericValue,
    RewardOfferDefinitionInput, RewardParameter, RewardProbability, RewardQuantity,
    RewardRarityWeight, RewardReplacementPolicy, RewardRequirement, RewardRequirementKind,
    RewardSelectionInput, RewardSemanticReference, RewardSemanticReferenceKind, RewardSourceError,
    RewardStatePolicy, RewardText, RewardUnavailableReason, RewardVisibility,
    RewardVisibilityScope,
};

#[path = "reward_manifest.rs"]
mod manifest_support;
pub use manifest_support::*;

pub fn text(value: &str) -> RewardText {
    RewardText::available(value).expect("text")
}

pub fn reference(kind: RewardSemanticReferenceKind, id: &str) -> RewardSemanticReference {
    RewardSemanticReference {
        kind,
        id: id.to_owned(),
        label: text(id),
    }
}

pub fn quantity(unit: Option<&str>, amount: i64) -> RewardQuantity {
    quantity_pair(unit, amount, amount)
}

pub fn quantity_pair(unit: Option<&str>, base: i64, visible: i64) -> RewardQuantity {
    RewardQuantity {
        unit: match unit {
            Some(unit) => RewardField::Available(unit.to_owned()),
            None => RewardField::Unavailable(RewardUnavailableReason::NotApplicable),
        },
        base_amount: RewardNumericValue::Fixed(base),
        visible_amount: RewardNumericValue::Fixed(visible),
        modified: RewardField::Available(base != visible),
    }
}

pub fn exact_probability(numerator: u32, denominator: u32) -> RewardProbability {
    RewardProbability::Exact {
        numerator,
        denominator,
        evidence: RewardEvidence::SourceDerived,
    }
}

pub fn action(action_kind: RewardActionKind) -> RewardLegalAction {
    RewardLegalAction {
        action_kind,
        label: text("action"),
        condition: RewardField::Unavailable(RewardUnavailableReason::NotApplicable),
        visibility: RewardVisibility::Visible,
    }
}

pub fn selection(
    group_id: &str,
    choose_min: u32,
    choose_max: u32,
    allow_skip: bool,
) -> RewardSelectionInput {
    RewardSelectionInput {
        group_id: group_id.to_owned(),
        label: text(group_id),
        choose_min: RewardField::Available(choose_min),
        choose_max: RewardField::Available(choose_max),
        optional_skip: RewardField::Available(allow_skip),
        legal_actions: vec![
            action(RewardActionKind::Choose),
            action(RewardActionKind::Skip),
        ],
        references: Vec::new(),
        visibility: RewardVisibility::Visible,
    }
}

pub fn parameter(parameter_id: &str, value: i64) -> RewardParameter {
    RewardParameter {
        parameter_id: parameter_id.to_owned(),
        label: text(parameter_id),
        unit: Some("count".to_owned()),
        value: RewardNumericValue::Fixed(value),
    }
}

pub fn requirement(requirement_id: &str, kind: RewardRequirementKind) -> RewardRequirement {
    RewardRequirement {
        requirement_id: requirement_id.to_owned(),
        kind,
        label: text(requirement_id),
        parameters: vec![parameter("min", 1)],
        references: Vec::new(),
        visibility: RewardVisibility::Visible,
    }
}

pub fn modifier(modifier_id: &str) -> RewardModifier {
    RewardModifier {
        modifier_id: modifier_id.to_owned(),
        kind: RewardModifierKind::IncreaseQuantity,
        label: text(modifier_id),
        amount: quantity(None, 1),
        condition: RewardField::Unavailable(RewardUnavailableReason::NotApplicable),
        rule_reference: RewardField::Available("rule:modifier".to_owned()),
        references: Vec::new(),
        evidence: RewardEvidence::SourceDerived,
        visibility: RewardVisibility::Visible,
    }
}

pub fn rarity_weight(rarity: &str, weight: i64) -> RewardRarityWeight {
    RewardRarityWeight {
        rarity: rarity.to_owned(),
        weight: quantity(None, weight),
        probability: exact_probability(1, 1),
        evidence: RewardEvidence::SourceDerived,
    }
}

pub fn rule(rule_id: &str, item_ids: &[&str]) -> RewardGenerationRule {
    RewardGenerationRule {
        rule_id: rule_id.to_owned(),
        label: text(rule_id),
        pool: RewardField::Available(
            item_ids
                .iter()
                .map(|id| reference(RewardSemanticReferenceKind::Item, id))
                .collect(),
        ),
        rarity_weights: vec![rarity_weight("rarity:common", 1)],
        rarity_status: sts2_game_mod::RewardFieldStatus::Available,
        eligibility: vec![requirement("req:act", RewardRequirementKind::Progression)],
        eligibility_status: sts2_game_mod::RewardFieldStatus::Available,
        modifiers: vec![modifier("mod:one")],
        modifiers_status: sts2_game_mod::RewardFieldStatus::Available,
        probability: exact_probability(1, 1),
        references: vec![reference(RewardSemanticReferenceKind::Rule, rule_id)],
        evidence: RewardEvidence::SourceDerived,
        visibility: RewardVisibility::Visible,
    }
}

pub fn state_policy() -> RewardStatePolicy {
    RewardStatePolicy {
        claim_limit: RewardField::Available(1),
        capacity: RewardField::Available(3),
        replacement: RewardField::Available(RewardReplacementPolicy::NotRequired),
        multi_stage: RewardField::Unavailable(RewardUnavailableReason::NotApplicable),
        visibility: RewardVisibility::Visible,
    }
}

pub fn kind_for(reference_kind: &RewardSemanticReferenceKind) -> RewardKind {
    match reference_kind {
        RewardSemanticReferenceKind::Card => RewardKind::Card,
        RewardSemanticReferenceKind::Relic => RewardKind::Relic,
        RewardSemanticReferenceKind::Potion => RewardKind::Potion,
        RewardSemanticReferenceKind::Currency => RewardKind::Currency,
        _ => RewardKind::SpecialGrant,
    }
}

pub fn item_with_quantity(
    item_id: &str,
    reference_kind: RewardSemanticReferenceKind,
    reference_id: &str,
    quantity: RewardQuantity,
    visibility: RewardVisibility,
) -> RewardItemInput {
    RewardItemInput {
        item_id: item_id.to_owned(),
        label: text(item_id),
        reference: reference(reference_kind, reference_id),
        quantity,
        instance: RewardField::Unavailable(RewardUnavailableReason::NotObserved),
        evidence: RewardEvidence::SourceDerived,
        visibility,
    }
}

pub fn item(
    item_id: &str,
    reference_kind: RewardSemanticReferenceKind,
    reference_id: &str,
    amount: i64,
    visibility: RewardVisibility,
) -> RewardItemInput {
    let unit = if matches!(reference_kind, RewardSemanticReferenceKind::Currency) {
        Some("currency:gold")
    } else {
        None
    };
    item_with_quantity(
        item_id,
        reference_kind,
        reference_id,
        quantity(unit, amount),
        visibility,
    )
}

pub fn reward_definition_input(
    reward_id: &str,
    kind: RewardKind,
    items: Vec<RewardItemInput>,
) -> RewardOfferDefinitionInput {
    let item_ids: Vec<&str> = items.iter().map(|item| item.item_id.as_str()).collect();
    let generation = vec![rule("rule:one", &item_ids)];
    RewardOfferDefinitionInput {
        reward_id: reward_id.to_owned(),
        label: text(reward_id),
        kind,
        unlock_state: ContentUnlockState::Unlocked,
        visibility: RewardVisibility::Visible,
        selection: selection("group:pick", 1, 1, true),
        items,
        generation,
        state_policy: state_policy(),
        references: Vec::new(),
    }
}

pub fn simple_reward(
    reward_id: &str,
    item_id: &str,
    reference_kind: RewardSemanticReferenceKind,
    reference_id: &str,
) -> RewardOfferDefinitionInput {
    let kind = kind_for(&reference_kind);
    reward_definition_input(
        reward_id,
        kind,
        vec![item(
            item_id,
            reference_kind,
            reference_id,
            1,
            RewardVisibility::Visible,
        )],
    )
}

pub fn card_reward(reward_id: &str) -> RewardOfferDefinitionInput {
    reward_definition_input(
        reward_id,
        RewardKind::Card,
        vec![
            item(
                "item:strike",
                RewardSemanticReferenceKind::Card,
                "card:strike",
                1,
                RewardVisibility::Visible,
            ),
            item(
                "item:defend",
                RewardSemanticReferenceKind::Card,
                "card:defend",
                1,
                RewardVisibility::Visible,
            ),
        ],
    )
}

pub fn gold_reward(reward_id: &str) -> RewardOfferDefinitionInput {
    reward_definition_input(
        reward_id,
        RewardKind::Currency,
        vec![item_with_quantity(
            "item:gold",
            RewardSemanticReferenceKind::Currency,
            "currency:gold",
            quantity_pair(Some("currency:gold"), 25, 40),
            RewardVisibility::Visible,
        )],
    )
}

pub fn full_manifest() -> ContentManifest {
    manifest(&[
        ("reward", "reward:card"),
        ("reward", "reward:relic"),
        ("reward", "reward:gold"),
        ("reward", "reward:potion"),
        ("card", "card:strike"),
        ("card", "card:defend"),
        ("relic", "relic:shrine"),
        ("potion", "potion:heal"),
        ("currency", "currency:gold"),
    ])
}

pub fn rich_catalog(content: &ContentManifest) -> RewardCatalog {
    catalog(
        content,
        vec![
            card_reward("reward:card"),
            simple_reward(
                "reward:relic",
                "item:relic",
                RewardSemanticReferenceKind::Relic,
                "relic:shrine",
            ),
            gold_reward("reward:gold"),
            simple_reward(
                "reward:potion",
                "item:potion",
                RewardSemanticReferenceKind::Potion,
                "potion:heal",
            ),
        ],
    )
}

pub fn snapshot(
    manifest: &ContentManifest,
    definitions: Vec<RewardOfferDefinitionInput>,
) -> RewardCatalogSnapshot {
    RewardCatalogSnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        producer_version: REWARD_REFERENCE_PRODUCER_VERSION.to_owned(),
        family: sts2_game_mod::RewardFamilyCoverage {
            entity_kind: REWARD_REFERENCE_ENTITY_KIND.to_owned(),
            state: sts2_game_mod::RewardFamilyState::Handled,
            definition_count: definitions.len(),
        },
        definitions,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardSource {
    pub snapshot: Result<RewardCatalogSnapshot, RewardSourceError>,
}

impl RewardCatalogSource for RewardSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<RewardCatalogSnapshot, RewardSourceError> {
        self.snapshot.clone()
    }
}

pub fn catalog(
    manifest: &ContentManifest,
    definitions: Vec<RewardOfferDefinitionInput>,
) -> RewardCatalog {
    RewardCatalogProducer::new()
        .produce(
            manifest,
            &RewardSource {
                snapshot: Ok(snapshot(manifest, definitions)),
            },
        )
        .expect("catalog")
}

pub fn reward_definition(catalog: &RewardCatalog, reward_id: &str) -> RewardDefinitionReference {
    RewardDefinitionReference {
        catalog: catalog.binding().clone(),
        reward_id: reward_id.to_owned(),
    }
}

pub fn item_reference(
    catalog: &RewardCatalog,
    reward_id: &str,
    item_id: &str,
) -> RewardItemReference {
    RewardItemReference {
        catalog: catalog.binding().clone(),
        reward_id: reward_id.to_owned(),
        item_id: item_id.to_owned(),
    }
}

pub fn list_query(
    locale: &str,
    scope: RewardVisibilityScope,
    limit: usize,
) -> sts2_game_mod::RewardListQuery {
    sts2_game_mod::RewardListQuery {
        locale: locale.to_owned(),
        scope,
        limit,
        continuation: None,
    }
}

pub fn item_list_query(
    reward: RewardDefinitionReference,
    scope: RewardVisibilityScope,
    limit: usize,
) -> sts2_game_mod::RewardItemListQuery {
    sts2_game_mod::RewardItemListQuery {
        reward,
        scope,
        limit,
        continuation: None,
    }
}
