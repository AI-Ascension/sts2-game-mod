// SPDX-License-Identifier: MIT

#[path = "support/game_facts_core_signatures.rs"]
mod signatures;
#[path = "support/game_facts_protocol_v1.rs"]
mod support;

use sts2_game_mod::{
    CoreFactsInventory, FactsEvidenceStatus, FactsRuleEntry, FactsUnsupportedCombination,
    GameFactsReferenceV1Adapter,
};
use sts2_protocol::game_facts_reference_v1::{
    Applicability, BindingMode, ErrorCode, EvidenceStatus, GameFactsReferenceV1Codec, MessageKind,
    QueryResultEntry, UnsupportedReason, validate_response_against_request,
};

#[test]
fn the_complete_core_subset_preserves_ordered_wire_metadata_and_the_existing_profile()
-> Result<(), String> {
    let manifest = support::manifest();
    let rules = signatures::rules();
    let admitted =
        CoreFactsInventory::new_for_manifest(&manifest, "standard", rules.clone(), Vec::new())
            .map_err(|error| error.to_string())?;
    let ids: Vec<&str> = rules
        .iter()
        .rev()
        .map(|rule| rule.rule_id.as_str())
        .collect();
    let request = support::query(&ids, &manifest.inventory_revision, BindingMode::Static);
    let response =
        GameFactsReferenceV1Adapter::respond_core(&request, Some(&admitted), Some(&manifest))
            .map_err(|error| error.to_string())?;
    GameFactsReferenceV1Codec::validate(&response).map_err(|error| error.to_string())?;
    validate_response_against_request(&request, &response).map_err(|error| error.to_string())?;
    assert_eq!(response.protocol_version, "game-facts-reference-v1");
    assert_eq!(
        response.schema_digest,
        "3065a2ff96e6e5af628b3d5cb43b8f2db4232ae4c4dc71324b973419908a2985"
    );
    assert_eq!(response.kind, MessageKind::QueryResponse);
    assert_eq!(response.query, request.query);
    assert_eq!(response.correlation_id, request.correlation_id);
    let result = response.result.as_ref().ok_or("missing complete result")?;
    assert_eq!(result.rules_reference_version, 2);
    assert_eq!(result.results.len(), 16);
    for (entry, expected) in result.results.iter().zip(rules.iter().rev()) {
        let QueryResultEntry::Found {
            rule_id,
            evidence_status,
            inputs,
        } = entry
        else {
            return Err("a reviewed static signature was not mapped".to_owned());
        };
        assert_eq!(rule_id, &expected.rule_id);
        assert_eq!(*evidence_status, EvidenceStatus::SourceDerived);
        for (input, expected_input) in inputs.iter().zip(&expected.inputs) {
            assert_eq!(input.name, expected_input.name);
            assert_eq!(
                serde_json::to_value(input.unit).map_err(|error| error.to_string())?,
                expected_input.unit
            );
            assert_eq!(input.applicability, Applicability::Required);
            assert!(input.observation.is_none());
            assert_eq!(
                input.source_ref.r#ref.as_deref(),
                expected_input
                    .source
                    .as_ref()
                    .map(|source| source.reference.as_str())
            );
        }
        assert_eq!(inputs.len(), expected.inputs.len());
    }
    let legacy =
        GameFactsReferenceV1Adapter::respond(&request, Some(admitted.inventory()), Some(&manifest))
            .map_err(|error| error.to_string())?;
    assert_eq!(
        serde_json::to_vec(&response).map_err(|error| error.to_string())?,
        serde_json::to_vec(&legacy).map_err(|error| error.to_string())?
    );
    Ok(())
}

#[test]
fn core_membership_never_grants_missing_source_live_or_confirmed_capability() -> Result<(), String>
{
    let manifest = support::manifest();
    // Core labels this arithmetic fixture Confirmed but its source is still Synthetic.
    let original = signatures::rules().remove(4);
    assert_eq!(original.rule_id, "heal.saturating_recovery");
    for evidence in FactsEvidenceStatus::ALL {
        let mut rule = original.clone();
        rule.evidence = evidence;
        let admitted =
            CoreFactsInventory::new_for_manifest(&manifest, "standard", vec![rule], Vec::new())
                .map_err(|error| error.to_string())?;
        for mode in [BindingMode::Static, BindingMode::Live] {
            let request = support::query(&[&original.rule_id], &manifest.inventory_revision, mode);
            let response = GameFactsReferenceV1Adapter::respond_core(
                &request,
                Some(&admitted),
                Some(&manifest),
            )
            .map_err(|error| error.to_string())?;
            if mode == BindingMode::Live || evidence == FactsEvidenceStatus::Confirmed {
                assert_eq!(
                    response.error.as_ref().map(|error| error.code),
                    Some(ErrorCode::MissingCapability)
                );
                assert!(response.result.is_none());
            } else {
                let result = response.result.as_ref().ok_or("missing static result")?;
                assert_eq!(
                    matches!(result.results[0], QueryResultEntry::Found { .. }),
                    evidence == FactsEvidenceStatus::SourceDerived
                );
            }
        }
    }
    let mut missing_source = original;
    missing_source.inputs[0].source = None;
    let admitted = CoreFactsInventory::new_for_manifest(
        &manifest,
        "standard",
        vec![missing_source],
        Vec::new(),
    )
    .map_err(|error| error.to_string())?;
    let request = support::query(
        &["heal.saturating_recovery"],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    let response =
        GameFactsReferenceV1Adapter::respond_core(&request, Some(&admitted), Some(&manifest))
            .map_err(|error| error.to_string())?;
    let result = response.result.ok_or("missing unsupported result")?;
    assert!(matches!(
        result.results[0],
        QueryResultEntry::Unsupported {
            reason_code: UnsupportedReason::MissingCapability,
            ..
        }
    ));
    Ok(())
}

#[test]
fn complete_membership_does_not_invent_missing_subset_rules_or_hide_interactions()
-> Result<(), String> {
    let manifest = support::manifest();
    let rules: Vec<FactsRuleEntry> = signatures::rules().into_iter().take(2).collect();
    let ids: Vec<String> = rules.iter().map(|rule| rule.rule_id.clone()).collect();
    let admitted = CoreFactsInventory::new_for_manifest(
        &manifest,
        "standard",
        rules,
        vec![FactsUnsupportedCombination {
            rule_ids: ids.clone(),
            reason: "Synthetic ordering excludes this interaction.".to_owned(),
        }],
    )
    .map_err(|error| error.to_string())?;
    for requested in [
        vec!["resource.fixed_card_cost"],
        vec!["resource.not_in_core"],
    ] {
        let request = support::query(
            &requested,
            &manifest.inventory_revision,
            BindingMode::Static,
        );
        let response =
            GameFactsReferenceV1Adapter::respond_core(&request, Some(&admitted), Some(&manifest))
                .map_err(|error| error.to_string())?;
        assert_eq!(
            response.error.as_ref().map(|error| error.code),
            Some(ErrorCode::MissingCapability)
        );
    }
    let partial = support::query(
        &[&ids[0]],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    let response =
        GameFactsReferenceV1Adapter::respond_core(&partial, Some(&admitted), Some(&manifest))
            .map_err(|error| error.to_string())?;
    let result = response.result.ok_or("missing partial result")?;
    assert!(matches!(
        result.results[0],
        QueryResultEntry::Unsupported {
            reason_code: UnsupportedReason::MissingCapability,
            ..
        }
    ));
    let both = support::query(
        &[&ids[1], &ids[0]],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    let response =
        GameFactsReferenceV1Adapter::respond_core(&both, Some(&admitted), Some(&manifest))
            .map_err(|error| error.to_string())?;
    assert_eq!(
        response
            .result
            .ok_or("missing interaction result")?
            .unsupported_combinations[0]
            .rule_ids,
        ids
    );

    let mut stale = manifest.clone();
    stale.catalog_generation += 1;
    let response = GameFactsReferenceV1Adapter::respond_core(&both, Some(&admitted), Some(&stale))
        .map_err(|error| error.to_string())?;
    assert_eq!(
        response.error.as_ref().map(|error| error.code),
        Some(ErrorCode::InvalidBinding)
    );
    Ok(())
}

#[test]
fn the_legacy_generic_mapper_still_accepts_its_existing_opaque_rule_vocabulary()
-> Result<(), String> {
    let manifest = support::manifest();
    let legacy = support::inventory(
        &manifest,
        vec![support::rule(
            "resource.block_gain",
            FactsEvidenceStatus::SourceDerived,
            vec![support::input("amount", "block", Some("source.block"))],
        )],
        Vec::new(),
    )
    .map_err(|error| error.to_string())?;
    let request = support::query(
        &["resource.block_gain"],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    let response = GameFactsReferenceV1Adapter::respond(&request, Some(&legacy), Some(&manifest))
        .map_err(|error| error.to_string())?;
    assert!(matches!(
        response.result.ok_or("missing legacy result")?.results[0],
        QueryResultEntry::Found { .. }
    ));
    assert!(
        CoreFactsInventory::new_for_manifest(
            &manifest,
            "standard",
            legacy.rules().to_vec(),
            Vec::new()
        )
        .is_err()
    );
    Ok(())
}
