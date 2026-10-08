// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::super::validation::{valid_source_reference_token, validate_manifest_binding};
use super::super::{
    FactsEvidenceStatus, FactsInputAvailability, FactsInventory, FactsRuleEntry, FactsSourceKind,
};
use crate::ContentManifest;
use sts2_protocol::game_facts_reference_v1::{
    Applicability, BindingMode, ErrorCode, EvidenceStatus, InventoryBinding, InventoryManifest,
    Query, QueryResult, QueryResultEntry, RuleInput, SourceKind, SourceRef, Unit,
    UnsupportedCombination, UnsupportedReason,
};

/// Projects one static query without inferring Core membership or rule values.
pub(super) fn map_query(
    query: &Query,
    inventory: Option<&FactsInventory>,
    manifest: Option<&ContentManifest>,
) -> Result<QueryResult, ErrorCode> {
    if query.binding.mode != BindingMode::Static {
        return Err(ErrorCode::MissingCapability);
    }
    let manifest = manifest.ok_or(ErrorCode::MissingCapability)?;
    let inventory = inventory.ok_or(ErrorCode::MissingCapability)?;
    let Some(captured_revision) = inventory.captured_inventory_revision() else {
        return Err(ErrorCode::MissingCapability);
    };
    if validate_manifest_binding(manifest).is_err()
        || query.binding.content_manifest_id != manifest.inventory_revision
        || query.binding.locale != manifest.locale
        || captured_revision != manifest.inventory_revision
        || inventory.build().build_id != manifest.game_build
        || inventory.build().manifest != manifest.cursor_binding()
    {
        return Err(ErrorCode::InvalidBinding);
    }

    let mut rules = Vec::with_capacity(query.rule_ids.len());
    for rule_id in &query.rule_ids {
        let rule = inventory
            .rule(rule_id)
            .ok_or(ErrorCode::MissingCapability)?;
        if rule.evidence == FactsEvidenceStatus::Confirmed {
            return Err(ErrorCode::MissingCapability);
        }
        rules.push(rule);
    }

    let requested: BTreeSet<&str> = query.rule_ids.iter().map(String::as_str).collect();
    let mut hidden_interactions = BTreeSet::new();
    let mut invalid_interactions = BTreeSet::new();
    let mut seen_interactions = BTreeSet::new();
    let mut combinations = Vec::new();
    for combination in inventory.unsupported_combinations() {
        let selected: Vec<&str> = combination
            .rule_ids
            .iter()
            .map(String::as_str)
            .filter(|rule_id| requested.contains(rule_id))
            .collect();
        if selected.is_empty() {
            continue;
        }
        if selected.len() != combination.rule_ids.len() {
            hidden_interactions.extend(selected.into_iter().map(str::to_owned));
            continue;
        }
        if !valid_combination_reason(&combination.reason) {
            invalid_interactions.extend(combination.rule_ids.iter().cloned());
            continue;
        }
        let mut key = combination.rule_ids.clone();
        key.sort_unstable();
        if !seen_interactions.insert(key.clone()) {
            invalid_interactions.extend(key.iter().cloned());
            combinations.retain(|existing: &UnsupportedCombination| {
                let mut existing_key = existing.rule_ids.clone();
                existing_key.sort_unstable();
                existing_key != key
            });
            continue;
        }
        combinations.push(UnsupportedCombination {
            rule_ids: combination.rule_ids.clone(),
            reason: combination.reason.clone(),
        });
    }

    let mut results = Vec::with_capacity(rules.len());
    for rule in rules {
        let entry = if hidden_interactions.contains(&rule.rule_id) {
            unsupported(&rule.rule_id, UnsupportedReason::MissingCapability)
        } else if invalid_interactions.contains(&rule.rule_id) {
            unsupported(&rule.rule_id, UnsupportedReason::UnsupportedField)
        } else if rule.evidence == FactsEvidenceStatus::SourceDerived {
            match map_source_derived_rule(rule) {
                Ok(inputs) => QueryResultEntry::Found {
                    rule_id: rule.rule_id.clone(),
                    evidence_status: EvidenceStatus::SourceDerived,
                    inputs,
                },
                Err(reason) => unsupported(&rule.rule_id, reason),
            }
        } else {
            unsupported(&rule.rule_id, UnsupportedReason::MissingCapability)
        };
        results.push(entry);
    }

    Ok(QueryResult {
        producer_version: super::super::GAME_FACTS_REFERENCE_PRODUCER_VERSION.to_owned(),
        rules_reference_version: query.rules_reference_version,
        inventory_binding: inventory_binding(inventory, manifest),
        results,
        unsupported_combinations: combinations,
    })
}

fn map_source_derived_rule(rule: &FactsRuleEntry) -> Result<Vec<RuleInput>, UnsupportedReason> {
    rule.inputs
        .iter()
        .map(|input| {
            if !protocol_identifier(&input.name) {
                return Err(UnsupportedReason::UnsupportedField);
            }
            let source = input
                .source
                .as_ref()
                .ok_or(UnsupportedReason::MissingCapability)?;
            if !valid_source_reference_token(&source.reference) {
                return Err(UnsupportedReason::UnsupportedField);
            }
            let unit = map_unit(&input.unit).ok_or(UnsupportedReason::UnsupportedField)?;
            let source_kind = match source.kind {
                FactsSourceKind::GameMod => SourceKind::GameMod,
                FactsSourceKind::ContentManifest => SourceKind::ContentManifest,
            };
            Ok(RuleInput {
                name: input.name.clone(),
                unit,
                applicability: match input.availability {
                    FactsInputAvailability::Required => Applicability::Required,
                    FactsInputAvailability::Conditional => Applicability::Conditional,
                    FactsInputAvailability::Unknown => Applicability::Unknown,
                },
                observation: None,
                source_ref: SourceRef {
                    kind: source_kind,
                    r#ref: Some(source.reference.clone()),
                },
            })
        })
        .collect()
}

fn inventory_binding(inventory: &FactsInventory, manifest: &ContentManifest) -> InventoryBinding {
    InventoryBinding {
        build_id: manifest.game_build.clone(),
        mode_id: inventory.build().mode_id.clone(),
        manifest: InventoryManifest {
            catalog_generation: manifest.catalog_generation,
            adapter_compatibility: manifest.adapter_compatibility.clone(),
            content_set_revision: manifest.content_set_revision.clone(),
            localized_text_revision: manifest.localized_text_revision.clone(),
            inventory_revision: manifest.inventory_revision.clone(),
        },
    }
}

fn unsupported(rule_id: &str, reason_code: UnsupportedReason) -> QueryResultEntry {
    QueryResultEntry::Unsupported {
        rule_id: rule_id.to_owned(),
        reason_code,
    }
}

fn map_unit(value: &str) -> Option<Unit> {
    Some(match value {
        "damage" => Unit::Damage,
        "hit_points" => Unit::HitPoints,
        "block" => Unit::Block,
        "energy" => Unit::Energy,
        "count" => Unit::Count,
        "turns" => Unit::Turns,
        "rounds" => Unit::Rounds,
        "gold" => Unit::Gold,
        "multiplier" => Unit::Multiplier,
        "ratio" => Unit::Ratio,
        "entity" => Unit::Entity,
        "boolean" => Unit::Boolean,
        "dimensionless" => Unit::Dimensionless,
        _ => return None,
    })
}

fn protocol_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
}

fn valid_combination_reason(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.chars().count() <= 256
        && !value.chars().any(char::is_control)
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains(':')
        && !value.contains("..")
        && !value.to_ascii_lowercase().contains("exception")
}
