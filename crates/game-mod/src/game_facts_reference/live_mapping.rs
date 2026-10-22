// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};
use sts2_game_core::{RuleId, rule_inventory};
use sts2_protocol::game_facts_reference_v1::{
    Applicability, Availability, BindingMode, EntityKind, ErrorCode, EvidenceStatus,
    InventoryBinding, InventoryManifest, Observation, Query, QueryResult, QueryResultEntry,
    RuleInput, SourceKind, SourceRef, Unit, UnsupportedReason, Value,
};

use super::{
    CoreFactsInventory, FactsEvidenceStatus, FactsInputAvailability, FactsInputSource,
    FactsRuleEntry, FactsRuleInput, FactsSourceKind, GAME_FACTS_REFERENCE_PRODUCER_VERSION,
    LiveFactsField, LiveFactsFieldStatus as Status, OwnedLiveFactsCard, OwnedLiveFactsSnapshot,
    is_opaque_facts_identity,
};
use crate::ContentManifest;

pub(super) fn map_query(
    query: &Query,
    source: &OwnedLiveFactsSnapshot,
    manifest: &ContentManifest,
) -> Result<QueryResult, ErrorCode> {
    if query.binding.mode != BindingMode::Live {
        return Err(ErrorCode::MissingCapability);
    }
    if query.scope.as_ref() != Some(&source.scope)
        || query.binding.visibility_scope.as_deref() != Some("owner")
    {
        return Err(ErrorCode::DeniedScope);
    }
    if query.binding.content_manifest_id != manifest.inventory_revision
        || query.binding.locale != manifest.locale
        || source.catalog_generation != manifest.catalog_generation
        || super::validation::validate_manifest_binding(manifest).is_err()
        || !is_opaque_facts_identity(&source.mode_id)
    {
        return Err(ErrorCode::InvalidBinding);
    }
    let instance = query
        .binding
        .instance_ref
        .as_ref()
        .ok_or(ErrorCode::InvalidBinding)?;
    let snapshot = query
        .binding
        .snapshot_ref
        .as_ref()
        .ok_or(ErrorCode::InvalidBinding)?;
    if instance.epoch != source.epoch
        || snapshot.snapshot_id != source.snapshot_id
        || snapshot.state_generation != source.state_generation
    {
        return Err(ErrorCode::StaleSnapshot);
    }
    if instance.entity_kind != EntityKind::Card {
        return Err(ErrorCode::MissingCapability);
    }
    if query
        .rule_ids
        .iter()
        .any(|id| !rule_inventory().iter().any(|rule| rule.id.as_str() == id))
    {
        return Err(ErrorCode::UnknownId);
    }
    if source.cards.len() > 512 || !valid_field(&source.player_energy) {
        return Err(ErrorCode::MissingCapability);
    }
    if source.player_energy.status == Status::Stale {
        return Err(ErrorCode::StaleSnapshot);
    }
    let mut seen = BTreeSet::new();
    for card in &source.cards {
        if card.resolved_cost.status == Status::Stale
            || card.fixed_energy_cost.status == Status::Stale
        {
            return Err(ErrorCode::StaleSnapshot);
        }
        if !owner_identity(&card.instance_id)
            || !seen.insert(&card.instance_id)
            || !owner_identity(&card.definition_id)
            || card.definition_manifest != manifest.inventory_revision
            || !valid_field(&card.resolved_cost)
            || !valid_field(&card.fixed_energy_cost)
        {
            return Err(ErrorCode::InvalidBinding);
        }
    }
    let card = source
        .cards
        .iter()
        .find(|card| card.instance_id == instance.entity_id)
        .ok_or(ErrorCode::UnknownId)?;
    if !manifest.definitions.iter().any(|definition| {
        definition.entity_kind == "card"
            && definition.namespaced_id == card.definition_id
            && definition.handled
    }) {
        return Err(ErrorCode::InvalidBinding);
    }
    let references = [
        reference(source, card, "energy"),
        reference(source, card, "card_cost"),
    ];
    let inventory = CoreFactsInventory::new_for_manifest(
        manifest,
        source.mode_id.clone(),
        vec![FactsRuleEntry {
            rule_id: RuleId::FixedCardCost.as_str().to_owned(),
            evidence: FactsEvidenceStatus::SourceDerived,
            inputs: ["energy", "card_cost"]
                .into_iter()
                .zip(&references)
                .map(|(name, source)| FactsRuleInput {
                    name: name.to_owned(),
                    unit: "energy".to_owned(),
                    availability: FactsInputAvailability::Required,
                    source: Some(FactsInputSource {
                        kind: FactsSourceKind::GameMod,
                        reference: source.clone(),
                    }),
                })
                .collect(),
        }],
        Vec::new(),
    )
    .map_err(|_| ErrorCode::MissingCapability)?;
    let results = query
        .rule_ids
        .iter()
        .map(|id| {
            if id != RuleId::FixedCardCost.as_str() {
                return Ok(QueryResultEntry::Unsupported {
                    rule_id: id.clone(),
                    reason_code: UnsupportedReason::MissingCapability,
                });
            }
            let observations = [
                integer_observation(&source.player_energy)?,
                cost_observation(card)?,
            ];
            Ok(QueryResultEntry::Found {
                rule_id: id.clone(),
                evidence_status: EvidenceStatus::SourceDerived,
                inputs: ["energy", "card_cost"]
                    .into_iter()
                    .zip(observations)
                    .zip(&references)
                    .map(|((name, observation), source)| RuleInput {
                        name: name.to_owned(),
                        unit: Unit::Energy,
                        applicability: Applicability::Required,
                        observation: Some(observation),
                        source_ref: SourceRef {
                            kind: SourceKind::GameMod,
                            r#ref: Some(source.clone()),
                        },
                    })
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, ErrorCode>>()?;
    Ok(QueryResult {
        producer_version: GAME_FACTS_REFERENCE_PRODUCER_VERSION.to_owned(),
        rules_reference_version: query.rules_reference_version,
        inventory_binding: InventoryBinding {
            build_id: manifest.game_build.clone(),
            mode_id: inventory.as_inventory().build().mode_id.clone(),
            manifest: InventoryManifest {
                catalog_generation: manifest.catalog_generation,
                adapter_compatibility: manifest.adapter_compatibility.clone(),
                content_set_revision: manifest.content_set_revision.clone(),
                localized_text_revision: manifest.localized_text_revision.clone(),
                inventory_revision: manifest.inventory_revision.clone(),
            },
        },
        results,
        unsupported_combinations: Vec::new(),
    })
}

fn valid_field<T>(field: &LiveFactsField<T>) -> bool {
    (field.status == Status::Available) == field.value.is_some()
}

fn owner_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
}

fn integer_observation(field: &LiveFactsField<i32>) -> Result<Observation, ErrorCode> {
    if let (Status::Available, Some(value)) = (field.status, field.value) {
        if value < 0 {
            return Err(ErrorCode::MissingCapability);
        }
        return Ok(Observation {
            availability: Availability::Available,
            value: Some(Value::Integer {
                value: i64::from(value),
            }),
            reason: None,
        });
    }
    unavailable(field.status)
}

fn cost_observation(card: &OwnedLiveFactsCard) -> Result<Observation, ErrorCode> {
    match (card.fixed_energy_cost.status, card.fixed_energy_cost.value) {
        (Status::Available, Some(true)) => integer_observation(&card.resolved_cost),
        (Status::Available, Some(false)) => Ok(Observation {
            availability: Availability::Unsupported,
            value: None,
            reason: Some("variable_energy_cost".to_owned()),
        }),
        (status, _) => unavailable(status),
    }
}

fn unavailable(status: Status) -> Result<Observation, ErrorCode> {
    let (availability, reason) = match status {
        Status::NotObserved => (Availability::NotObservable, "field_not_observed"),
        Status::Unsupported => (Availability::Unsupported, "field_unsupported"),
        Status::Unknown => (Availability::Unavailable, "field_unknown"),
        Status::Failed => (Availability::Unavailable, "field_read_failed"),
        Status::Stale => return Err(ErrorCode::StaleSnapshot),
        Status::Available => return Err(ErrorCode::MissingCapability),
    };
    Ok(Observation {
        availability,
        value: None,
        reason: Some(reason.to_owned()),
    })
}

fn reference(source: &OwnedLiveFactsSnapshot, card: &OwnedLiveFactsCard, name: &str) -> String {
    let mut hash = Sha256::new();
    for part in [&source.snapshot_id, &card.instance_id, name] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    let digest: String = hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("owned-card-facts-v1.{digest}")
}
