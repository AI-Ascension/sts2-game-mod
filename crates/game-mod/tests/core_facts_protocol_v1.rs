// SPDX-License-Identifier: MIT

#[path = "support/core_facts_catalog.rs"]
mod catalog;
#[allow(dead_code)]
#[path = "support/game_facts_protocol_v1.rs"]
mod support;

use sts2_game_mod::{
    CoreFactsInventory, FactsEvidenceStatus, FactsInventory, FactsRepresentation, FactsRuleEntry,
    FactsSourceKind, FactsUnsupportedCombination, GameFactsReferenceV1Adapter,
};
use sts2_protocol::game_facts_reference_v1::{
    Applicability, BindingMode, ErrorCode, EvidenceStatus, GameFactsReferenceV1Codec, Message,
    QueryResultEntry, SourceKind, UnsupportedReason, validate_response_against_request,
};

fn admit(rules: Vec<FactsRuleEntry>) -> Result<CoreFactsInventory, String> {
    CoreFactsInventory::new_for_manifest(&support::manifest(), "standard", rules, Vec::new())
        .map_err(|error| error.to_string())
}

fn respond(
    inventory: &CoreFactsInventory,
    ids: &[&str],
    mode: BindingMode,
) -> Result<Message, String> {
    let manifest = support::manifest();
    let request = support::query(ids, &manifest.inventory_revision, mode);
    let request = GameFactsReferenceV1Codec::decode(
        &GameFactsReferenceV1Codec::encode(&request).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let response =
        GameFactsReferenceV1Adapter::respond_admitted(&request, Some(inventory), Some(&manifest))
            .map_err(|error| error.to_string())?;
    let response = GameFactsReferenceV1Codec::decode(
        &GameFactsReferenceV1Codec::encode(&response).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    validate_response_against_request(&request, &response).map_err(|error| error.to_string())?;
    Ok(response)
}

#[test]
fn all_sixteen_signatures_round_trip_with_exact_metadata_and_generic_bytes() -> Result<(), String> {
    let manifest = support::manifest();
    let mut rules = catalog::rules();
    for rule in &mut rules {
        for (index, input) in rule.inputs.iter_mut().enumerate() {
            if let Some(source) = input.source.as_mut() {
                source.kind = if index % 2 == 0 {
                    FactsSourceKind::GameMod
                } else {
                    FactsSourceKind::ContentManifest
                };
            }
        }
    }
    let admitted = admit(rules.clone())?;
    let generic = FactsInventory::new_for_manifest(
        &manifest,
        "standard",
        FactsRepresentation {
            version: "rules-reference-v2".to_owned(),
            encoding: "game-facts-reference-v1".to_owned(),
        },
        rules.clone(),
        Vec::new(),
    )
    .map_err(|error| error.to_string())?;
    let ids: Vec<&str> = rules
        .iter()
        .rev()
        .map(|rule| rule.rule_id.as_str())
        .collect();
    let request = support::query(&ids, &manifest.inventory_revision, BindingMode::Static);
    let response = respond(&admitted, &ids, BindingMode::Static)?;
    let generic_response =
        GameFactsReferenceV1Adapter::respond(&request, Some(&generic), Some(&manifest))
            .map_err(|error| error.to_string())?;
    assert_eq!(
        GameFactsReferenceV1Codec::encode(&response).map_err(|error| error.to_string())?,
        GameFactsReferenceV1Codec::encode(&generic_response).map_err(|error| error.to_string())?
    );
    let result = response.result.ok_or("missing result")?;
    assert_eq!(result.results.len(), 16);
    for (entry, expected) in result.results.iter().zip(rules.iter().rev()) {
        let QueryResultEntry::Found {
            rule_id,
            evidence_status,
            inputs,
        } = entry
        else {
            return Err("admitted synthetic source-derived signature was not mapped".to_owned());
        };
        assert_eq!(rule_id, &expected.rule_id);
        assert_eq!(*evidence_status, EvidenceStatus::SourceDerived);
        assert_eq!(inputs.len(), expected.inputs.len());
        for (input, expected) in inputs.iter().zip(&expected.inputs) {
            assert_eq!(input.name, expected.name);
            assert_eq!(
                serde_json::to_value(input.unit).map_err(|error| error.to_string())?,
                expected.unit
            );
            assert_eq!(input.applicability, Applicability::Required);
            assert!(input.observation.is_none());
            let source = expected.source.as_ref().ok_or("fixture omitted source")?;
            assert_eq!(
                input.source_ref.r#ref.as_deref(),
                Some(source.reference.as_str())
            );
            assert_eq!(
                input.source_ref.kind,
                match source.kind {
                    FactsSourceKind::GameMod => SourceKind::GameMod,
                    FactsSourceKind::ContentManifest => SourceKind::ContentManifest,
                }
            );
        }
    }
    Ok(())
}

#[test]
fn admission_neither_supplies_provenance_nor_promotes_evidence() -> Result<(), String> {
    for status in FactsEvidenceStatus::ALL {
        let mut rule = catalog::rules().remove(4); // Core labels its synthetic heal fixture Confirmed.
        rule.evidence = status;
        let id = rule.rule_id.clone();
        let admitted = admit(vec![rule])?;
        assert_eq!(admitted.as_inventory().rules()[0].evidence, status);
        let response = respond(&admitted, &[&id], BindingMode::Static)?;
        if status == FactsEvidenceStatus::Confirmed {
            assert_eq!(
                response.error.map(|error| error.code),
                Some(ErrorCode::MissingCapability)
            );
        } else {
            let result = response.result.ok_or("missing result")?;
            if status == FactsEvidenceStatus::SourceDerived {
                assert!(matches!(
                    result.results[0],
                    QueryResultEntry::Found {
                        evidence_status: EvidenceStatus::SourceDerived,
                        ..
                    }
                ));
            } else {
                assert!(matches!(
                    result.results[0],
                    QueryResultEntry::Unsupported {
                        reason_code: UnsupportedReason::MissingCapability,
                        ..
                    }
                ));
            }
        }
    }
    let mut rule = catalog::rules().remove(0);
    rule.inputs[0].source = None;
    let id = rule.rule_id.clone();
    let admitted = admit(vec![rule])?;
    assert!(
        admitted.as_inventory().rules()[0].inputs[0]
            .source
            .is_none()
    );
    let response = respond(&admitted, &[&id], BindingMode::Static)?;
    assert!(matches!(
        response.result.ok_or("missing result")?.results[0],
        QueryResultEntry::Unsupported {
            reason_code: UnsupportedReason::MissingCapability,
            ..
        }
    ));
    Ok(())
}

#[test]
fn known_but_unavailable_subset_live_and_absent_inputs_keep_refusals() -> Result<(), String> {
    let admitted = admit(vec![catalog::rules().remove(0)])?;
    for (ids, mode) in [
        (vec!["resource.fixed_card_cost"], BindingMode::Static),
        (vec!["unknown.rule"], BindingMode::Static),
        (vec!["damage.nominal_card"], BindingMode::Live),
    ] {
        let response = respond(&admitted, &ids, mode)?;
        assert_eq!(
            response.error.map(|error| error.code),
            Some(ErrorCode::MissingCapability)
        );
    }
    let manifest = support::manifest();
    let request = support::query(
        &["damage.nominal_card"],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    for (inventory, manifest) in [(None, Some(&manifest)), (Some(&admitted), None)] {
        let response = GameFactsReferenceV1Adapter::respond_admitted(&request, inventory, manifest)
            .map_err(|error| error.to_string())?;
        assert_eq!(
            response.error.as_ref().map(|error| error.code),
            Some(ErrorCode::MissingCapability)
        );
        validate_response_against_request(&request, &response)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[test]
fn manifest_changes_after_admission_are_not_authenticated_or_silently_rebound() -> Result<(), String>
{
    let admitted = admit(vec![catalog::rules().remove(0)])?;
    for field in 0..7 {
        let mut changed = support::manifest();
        match field {
            0 => changed.game_build = "changed-build".to_owned(),
            1 => changed.adapter_compatibility = "changed-adapter".to_owned(),
            2 => changed.catalog_generation += 1,
            3 => changed.content_set_revision = "changed-content".to_owned(),
            4 => changed.localized_text_revision = "changed-text".to_owned(),
            5 => changed.inventory_revision = "changed-inventory".to_owned(),
            _ => changed.locale = "fr".to_owned(),
        }
        let request = support::query(
            &["damage.nominal_card"],
            &changed.inventory_revision,
            BindingMode::Static,
        );
        let response = GameFactsReferenceV1Adapter::respond_admitted(
            &request,
            Some(&admitted),
            Some(&changed),
        )
        .map_err(|error| error.to_string())?;
        assert_eq!(
            response.error.as_ref().map(|error| error.code),
            Some(ErrorCode::InvalidBinding)
        );
        validate_response_against_request(&request, &response)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[test]
fn declared_unsupported_interactions_remain_ordered_and_cannot_be_hidden() -> Result<(), String> {
    let manifest = support::manifest();
    let rules: Vec<_> = catalog::rules().into_iter().take(2).collect();
    let ids: Vec<_> = rules.iter().map(|rule| rule.rule_id.clone()).collect();
    let unsupported = vec![FactsUnsupportedCombination {
        rule_ids: ids.clone(),
        reason: "Synthetic combination not established".to_owned(),
    }];
    let admitted =
        CoreFactsInventory::new_for_manifest(&manifest, "standard", rules, unsupported.clone())
            .map_err(|error| error.to_string())?;
    assert_eq!(
        admitted.as_inventory().unsupported_combinations(),
        unsupported
    );
    let partial = respond(&admitted, &[&ids[0]], BindingMode::Static)?;
    assert!(matches!(
        partial.result.ok_or("missing result")?.results[0],
        QueryResultEntry::Unsupported {
            reason_code: UnsupportedReason::MissingCapability,
            ..
        }
    ));
    let complete = respond(&admitted, &[&ids[1], &ids[0]], BindingMode::Static)?;
    let combinations = complete
        .result
        .ok_or("missing result")?
        .unsupported_combinations;
    assert_eq!(combinations.len(), 1);
    assert_eq!(combinations[0].rule_ids, ids);
    assert_eq!(combinations[0].reason, unsupported[0].reason);
    Ok(())
}
