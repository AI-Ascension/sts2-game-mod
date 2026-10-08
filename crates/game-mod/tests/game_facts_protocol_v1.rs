// SPDX-License-Identifier: MIT

#[path = "support/game_facts_protocol_v1.rs"]
mod support;
#[path = "support/game_facts_unsupported_combinations.rs"]
mod unsupported_combinations;

use sts2_game_mod::{
    FactsBuildBinding, FactsEvidenceStatus, FactsInventory, FactsSourceKind, GameFactsError,
    GameFactsReferenceV1Adapter,
};
use sts2_protocol::game_facts_reference_v1::{
    BindingMode, ErrorCode, EvidenceStatus, GameFactsReferenceV1Codec, MessageKind,
    QueryResultEntry, SourceKind, UnsupportedReason, validate_response_against_request,
};

fn unbound_inventory() -> Result<FactsInventory, GameFactsError> {
    let manifest = support::manifest();
    FactsInventory::new(
        FactsBuildBinding {
            build_id: manifest.game_build.clone(),
            mode_id: "standard".to_owned(),
            manifest: manifest.cursor_binding(),
        },
        support::representation(),
        vec![support::rule(
            "resource.block_gain",
            FactsEvidenceStatus::SourceDerived,
            vec![support::input("amount", "block", Some("source.block"))],
        )],
        Vec::new(),
    )
}

#[test]
fn capabilities_report_only_the_fixed_static_profile_bounds() -> Result<(), String> {
    let response = GameFactsReferenceV1Adapter::capabilities("facts-caps-1")
        .map_err(|error| error.to_string())?;
    GameFactsReferenceV1Codec::validate(&response).map_err(|error| error.to_string())?;
    let capabilities = response
        .capabilities
        .as_ref()
        .ok_or_else(|| "capabilities response omitted its payload".to_owned())?;

    assert_eq!(response.kind, MessageKind::CapabilitiesResponse);
    assert_eq!(capabilities.max_rule_ids, 16);
    assert_eq!(capabilities.max_inputs_per_rule, 64);
    assert_eq!(capabilities.max_unsupported_combinations, 256);
    assert_eq!(capabilities.max_message_bytes, 262_144);
    assert!(capabilities.snapshot_policy.is_none());
    Ok(())
}

#[test]
fn synthetic_source_derived_values_map_in_query_and_input_order() -> Result<(), String> {
    let manifest = support::manifest();
    let inventory = support::inventory(
        &manifest,
        vec![
            support::rule(
                "resource.damage",
                FactsEvidenceStatus::SourceDerived,
                vec![
                    support::input("multiplier_numerator", "multiplier", Some("mod.damage.num")),
                    support::input(
                        "multiplier_denominator",
                        "multiplier",
                        Some("mod.damage.den"),
                    ),
                ],
            ),
            support::rule(
                "resource.block_gain",
                FactsEvidenceStatus::SourceDerived,
                vec![support::input_with_kind(
                    "amount",
                    "block",
                    FactsSourceKind::ContentManifest,
                    Some("content.block.value"),
                )],
            ),
        ],
        Vec::new(),
    )
    .map_err(|error| error.to_string())?;
    let request = support::query(
        &["resource.block_gain", "resource.damage"],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    let response =
        GameFactsReferenceV1Adapter::respond(&request, Some(&inventory), Some(&manifest))
            .map_err(|error| error.to_string())?;
    GameFactsReferenceV1Codec::validate(&response).map_err(|error| error.to_string())?;
    validate_response_against_request(&request, &response).map_err(|error| error.to_string())?;

    assert_eq!(response.kind, MessageKind::QueryResponse);
    assert_eq!(response.query, request.query);
    let result = response
        .result
        .as_ref()
        .ok_or_else(|| "query response omitted result".to_owned())?;
    assert_eq!(result.inventory_binding.build_id, manifest.game_build);
    assert_eq!(result.inventory_binding.mode_id, "standard");
    assert_eq!(
        result.inventory_binding.manifest.inventory_revision,
        manifest.inventory_revision
    );
    assert_eq!(result.results.len(), 2);
    match &result.results[0] {
        QueryResultEntry::Found {
            rule_id,
            evidence_status,
            inputs,
        } => {
            assert_eq!(rule_id, "resource.block_gain");
            assert_eq!(*evidence_status, EvidenceStatus::SourceDerived);
            assert_eq!(inputs[0].name, "amount");
            assert_eq!(inputs[0].observation, None);
            assert_eq!(inputs[0].source_ref.kind, SourceKind::ContentManifest);
            assert_eq!(
                inputs[0].source_ref.r#ref.as_deref(),
                Some("content.block.value")
            );
        }
        QueryResultEntry::Unsupported { .. } => {
            return Err("source-derived block rule was not mapped".to_owned());
        }
    }
    match &result.results[1] {
        QueryResultEntry::Found { inputs, .. } => {
            assert_eq!(inputs[0].name, "multiplier_numerator");
            assert_eq!(inputs[1].name, "multiplier_denominator");
            assert_eq!(inputs[0].unit, inputs[1].unit);
            assert_eq!(inputs[0].source_ref.kind, SourceKind::GameMod);
        }
        QueryResultEntry::Unsupported { .. } => {
            return Err("source-derived damage rule was not mapped".to_owned());
        }
    }
    Ok(())
}

#[test]
fn absent_unbound_and_unknown_inventories_return_missing_capability() -> Result<(), String> {
    let manifest = support::manifest();
    let request = support::query(
        &["resource.block_gain"],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    let absent_response = GameFactsReferenceV1Adapter::respond(&request, None, Some(&manifest))
        .map_err(|error| error.to_string())?;
    let unbound = unbound_inventory().map_err(|error| error.to_string())?;
    let unbound_response =
        GameFactsReferenceV1Adapter::respond(&request, Some(&unbound), Some(&manifest))
            .map_err(|error| error.to_string())?;
    for response in [absent_response, unbound_response] {
        assert_eq!(response.kind, MessageKind::ErrorResponse);
        assert_eq!(response.query, request.query);
        assert_eq!(
            response.error.as_ref().map(|error| error.code),
            Some(ErrorCode::MissingCapability)
        );
        assert!(response.result.is_none());
        validate_response_against_request(&request, &response)
            .map_err(|error| error.to_string())?;
    }

    let known = support::inventory(
        &manifest,
        vec![support::rule(
            "resource.block_gain",
            FactsEvidenceStatus::SourceDerived,
            vec![support::input("amount", "block", Some("source.block"))],
        )],
        Vec::new(),
    )
    .map_err(|error| error.to_string())?;
    let unknown_request = support::query(
        &["resource.not_in_subset"],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    let unknown_response =
        GameFactsReferenceV1Adapter::respond(&unknown_request, Some(&known), Some(&manifest))
            .map_err(|error| error.to_string())?;
    assert_eq!(
        unknown_response.error.as_ref().map(|error| error.code),
        Some(ErrorCode::MissingCapability)
    );
    Ok(())
}

#[test]
fn static_confirmed_and_live_requests_refuse_without_downgrading_evidence() -> Result<(), String> {
    let manifest = support::manifest();
    let inventory = support::inventory(
        &manifest,
        vec![support::rule(
            "resource.block_gain",
            FactsEvidenceStatus::Confirmed,
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
    let response =
        GameFactsReferenceV1Adapter::respond(&request, Some(&inventory), Some(&manifest))
            .map_err(|error| error.to_string())?;
    assert_eq!(
        response.error.as_ref().map(|error| error.code),
        Some(ErrorCode::MissingCapability)
    );

    let live_request = support::query(
        &["resource.block_gain"],
        &manifest.inventory_revision,
        BindingMode::Live,
    );
    let live_response =
        GameFactsReferenceV1Adapter::respond(&live_request, Some(&inventory), Some(&manifest))
            .map_err(|error| error.to_string())?;
    assert_eq!(
        live_response.error.as_ref().map(|error| error.code),
        Some(ErrorCode::MissingCapability)
    );
    Ok(())
}

#[test]
fn partial_queries_do_not_hide_interactions_and_complete_queries_echo_them() -> Result<(), String> {
    let manifest = support::manifest();
    let inventory = support::inventory(
        &manifest,
        vec![
            support::rule(
                "resource.alpha",
                FactsEvidenceStatus::SourceDerived,
                vec![support::input("amount", "count", Some("source.alpha"))],
            ),
            support::rule(
                "resource.beta",
                FactsEvidenceStatus::SourceDerived,
                vec![support::input("amount", "count", Some("source.beta"))],
            ),
        ],
        vec![unsupported_combinations::unsupported(
            &["resource.alpha", "resource.beta"],
            "These rules interact.",
        )],
    )
    .map_err(|error| error.to_string())?;

    let partial_request = support::query(
        &["resource.alpha"],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    let partial =
        GameFactsReferenceV1Adapter::respond(&partial_request, Some(&inventory), Some(&manifest))
            .map_err(|error| error.to_string())?;
    let partial_results = &partial
        .result
        .as_ref()
        .ok_or_else(|| "partial response omitted result".to_owned())?
        .results;
    assert!(matches!(
        partial_results[0],
        QueryResultEntry::Unsupported {
            reason_code: UnsupportedReason::MissingCapability,
            ..
        }
    ));

    let complete_request = support::query(
        &["resource.alpha", "resource.beta"],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    let complete =
        GameFactsReferenceV1Adapter::respond(&complete_request, Some(&inventory), Some(&manifest))
            .map_err(|error| error.to_string())?;
    let result = complete
        .result
        .as_ref()
        .ok_or_else(|| "complete response omitted result".to_owned())?;
    assert_eq!(result.unsupported_combinations.len(), 1);
    assert_eq!(
        result.unsupported_combinations[0].rule_ids,
        vec!["resource.alpha".to_owned(), "resource.beta".to_owned()]
    );
    Ok(())
}

#[test]
fn request_echo_is_exact_for_missing_capability() -> Result<(), String> {
    let manifest = support::manifest();
    let request = support::query(
        &["resource.block_gain"],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    let response = GameFactsReferenceV1Adapter::respond(&request, None, None)
        .map_err(|error| error.to_string())?;
    assert_eq!(response.query, request.query);
    assert_eq!(response.correlation_id, request.correlation_id);
    assert_eq!(response.kind, MessageKind::ErrorResponse);
    Ok(())
}
