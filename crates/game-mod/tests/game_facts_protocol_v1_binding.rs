// SPDX-License-Identifier: MIT

#[path = "support/game_facts_protocol_v1.rs"]
mod support;

use sts2_game_mod::{
    ContentManifest, FactsEvidenceStatus, FactsInventory, FactsSourceKind,
    GameFactsReferenceV1Adapter,
};
use sts2_protocol::game_facts_reference_v1::{
    BindingMode, ErrorCode, GameFactsReferenceV1Codec, GameFactsReferenceV1Rejection, MessageKind,
    QueryResultEntry, validate_response_against_request,
};

fn inventory(manifest: &ContentManifest) -> Result<FactsInventory, String> {
    support::inventory(
        manifest,
        vec![support::rule(
            "resource.block_gain",
            FactsEvidenceStatus::SourceDerived,
            vec![support::input("amount", "block", Some("source.block"))],
        )],
        Vec::new(),
    )
    .map_err(|error| error.to_string())
}

fn assert_invalid_binding(
    inventory: &FactsInventory,
    manifest: &ContentManifest,
    requested_revision: &str,
) -> Result<(), String> {
    let request = support::query(
        &["resource.block_gain"],
        requested_revision,
        BindingMode::Static,
    );
    let response = GameFactsReferenceV1Adapter::respond(&request, Some(inventory), Some(manifest))
        .map_err(|error| error.to_string())?;

    assert_eq!(response.kind, MessageKind::ErrorResponse);
    assert_eq!(response.query, request.query);
    assert_eq!(response.correlation_id, request.correlation_id);
    let error = response
        .error
        .as_ref()
        .ok_or_else(|| "binding refusal omitted its typed error".to_owned())?;
    assert_eq!(error.code, ErrorCode::InvalidBinding);
    assert_eq!(error.field, None);
    assert_eq!(
        error.reason.as_deref(),
        Some("The supplied content binding does not match this request.")
    );
    assert!(!error.retryable);
    validate_response_against_request(&request, &response).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn independent_binding_field_changes_return_invalid_binding() -> Result<(), String> {
    let original = support::manifest();
    let original_inventory = inventory(&original)?;

    assert_invalid_binding(&original_inventory, &original, "another-inventory")?;

    // This field is also a member of ContentCursorBinding, so change it only once here.
    let mut changed_revision = original.clone();
    changed_revision.inventory_revision = "inventory-other".to_owned();
    assert_invalid_binding(
        &original_inventory,
        &changed_revision,
        &changed_revision.inventory_revision,
    )?;

    let mut changed_build = original.clone();
    changed_build.game_build = "build-other".to_owned();
    assert_invalid_binding(
        &original_inventory,
        &changed_build,
        &original.inventory_revision,
    )?;

    let mut changed_generation = original.clone();
    changed_generation.catalog_generation += 1;
    assert_invalid_binding(
        &original_inventory,
        &changed_generation,
        &original.inventory_revision,
    )?;

    let mut changed_adapter = original.clone();
    changed_adapter.adapter_compatibility = "adapter-other".to_owned();
    assert_invalid_binding(
        &original_inventory,
        &changed_adapter,
        &original.inventory_revision,
    )?;

    let mut changed_content = original.clone();
    changed_content.content_set_revision = "content-other".to_owned();
    assert_invalid_binding(
        &original_inventory,
        &changed_content,
        &original.inventory_revision,
    )?;

    let mut changed_text = original.clone();
    changed_text.localized_text_revision = "text-other".to_owned();
    assert_invalid_binding(
        &original_inventory,
        &changed_text,
        &original.inventory_revision,
    )?;
    Ok(())
}

#[test]
fn request_locale_mismatch_returns_invalid_binding_with_echo_and_conformance() -> Result<(), String>
{
    let manifest = support::manifest();
    let inventory = inventory(&manifest)?;
    let mut request = support::query(
        &["resource.block_gain"],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    request
        .query
        .as_mut()
        .ok_or_else(|| "locale mismatch query was omitted".to_owned())?
        .binding
        .locale = "fr".to_owned();

    let response =
        GameFactsReferenceV1Adapter::respond(&request, Some(&inventory), Some(&manifest))
            .map_err(|error| error.to_string())?;
    GameFactsReferenceV1Codec::validate(&response).map_err(|error| error.to_string())?;
    validate_response_against_request(&request, &response).map_err(|error| error.to_string())?;

    assert_eq!(response.kind, MessageKind::ErrorResponse);
    assert_eq!(response.query, request.query);
    assert_eq!(response.correlation_id, request.correlation_id);
    let error = response
        .error
        .as_ref()
        .ok_or_else(|| "locale mismatch omitted its typed error".to_owned())?;
    assert_eq!(error.code, ErrorCode::InvalidBinding);
    assert_eq!(error.field, None);
    assert_eq!(
        error.reason.as_deref(),
        Some("The supplied content binding does not match this request.")
    );
    assert!(!error.retryable);
    Ok(())
}

#[test]
fn matching_non_default_locale_is_accepted() -> Result<(), String> {
    let mut manifest = support::manifest();
    manifest.locale = "fr".to_owned();
    let inventory = inventory(&manifest)?;
    let mut request = support::query(
        &["resource.block_gain"],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    request
        .query
        .as_mut()
        .ok_or_else(|| "non-default locale query was omitted".to_owned())?
        .binding
        .locale = manifest.locale.clone();

    let response =
        GameFactsReferenceV1Adapter::respond(&request, Some(&inventory), Some(&manifest))
            .map_err(|error| error.to_string())?;
    GameFactsReferenceV1Codec::validate(&response).map_err(|error| error.to_string())?;
    validate_response_against_request(&request, &response).map_err(|error| error.to_string())?;

    assert_eq!(response.kind, MessageKind::QueryResponse);
    assert_eq!(response.query, request.query);
    assert_eq!(response.correlation_id, request.correlation_id);
    let result = response
        .result
        .as_ref()
        .ok_or_else(|| "matching non-default locale omitted its result".to_owned())?;
    assert!(matches!(result.results[0], QueryResultEntry::Found { .. }));
    Ok(())
}

#[test]
fn captured_inventory_locale_mismatch_returns_invalid_binding() -> Result<(), String> {
    let captured_manifest = support::manifest();
    let inventory = inventory(&captured_manifest)?;
    // Change only locale after capture, leaving the inventory revision and cursor unchanged.
    let mut supplied_manifest = captured_manifest.clone();
    supplied_manifest.locale = "fr".to_owned();

    let mut request = support::query(
        &["resource.block_gain"],
        &supplied_manifest.inventory_revision,
        BindingMode::Static,
    );
    request
        .query
        .as_mut()
        .ok_or_else(|| "captured-locale query was omitted".to_owned())?
        .binding
        .locale = supplied_manifest.locale.clone();

    let response =
        GameFactsReferenceV1Adapter::respond(&request, Some(&inventory), Some(&supplied_manifest))
            .map_err(|error| error.to_string())?;
    GameFactsReferenceV1Codec::validate(&response).map_err(|error| error.to_string())?;
    validate_response_against_request(&request, &response).map_err(|error| error.to_string())?;

    assert_eq!(response.kind, MessageKind::ErrorResponse);
    assert_eq!(response.query, request.query);
    assert_eq!(response.correlation_id, request.correlation_id);
    let error = response
        .error
        .as_ref()
        .ok_or_else(|| "captured-locale mismatch omitted its typed error".to_owned())?;
    assert_eq!(error.code, ErrorCode::InvalidBinding);
    assert_eq!(error.field, None);
    assert_eq!(
        error.reason.as_deref(),
        Some("The supplied content binding does not match this request.")
    );
    assert!(!error.retryable);
    Ok(())
}

#[test]
fn consistent_values_echo_complete_manifest_binding_and_query() -> Result<(), String> {
    let manifest = support::manifest();
    let inventory = inventory(&manifest)?;
    let request = support::query(
        &["resource.block_gain"],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    let response =
        GameFactsReferenceV1Adapter::respond(&request, Some(&inventory), Some(&manifest))
            .map_err(|error| error.to_string())?;
    GameFactsReferenceV1Codec::validate(&response).map_err(|error| error.to_string())?;
    validate_response_against_request(&request, &response).map_err(|error| error.to_string())?;

    assert_eq!(response.query, request.query);
    assert_eq!(response.correlation_id, request.correlation_id);
    let result = response
        .result
        .as_ref()
        .ok_or_else(|| "bound response omitted its result".to_owned())?;
    assert_eq!(result.inventory_binding.build_id, manifest.game_build);
    assert_eq!(result.inventory_binding.mode_id, "standard");
    let binding = &result.inventory_binding.manifest;
    assert_eq!(binding.catalog_generation, manifest.catalog_generation);
    assert_eq!(
        binding.adapter_compatibility,
        manifest.adapter_compatibility
    );
    assert_eq!(binding.content_set_revision, manifest.content_set_revision);
    assert_eq!(
        binding.localized_text_revision,
        manifest.localized_text_revision
    );
    assert_eq!(binding.inventory_revision, manifest.inventory_revision);
    assert!(matches!(result.results[0], QueryResultEntry::Found { .. }));
    Ok(())
}

#[test]
fn valid_non_query_message_kind_is_refused_as_an_invalid_request() -> Result<(), String> {
    let request = GameFactsReferenceV1Adapter::capabilities("facts-caps-1")
        .map_err(|error| error.to_string())?;
    assert_eq!(request.kind, MessageKind::CapabilitiesResponse);
    assert_eq!(
        GameFactsReferenceV1Adapter::respond(&request, None, None),
        Err(GameFactsReferenceV1Rejection::InvalidRequest)
    );
    Ok(())
}

#[test]
fn source_reference_tokens_enforce_the_closed_shape_rules() -> Result<(), String> {
    let manifest = support::manifest();
    let overlong = format!("source{}", "x".repeat(123));
    let invalid_tokens = [
        "C:/private/source",
        "source..item",
        "fileRecord",
        "source\u{7f}control",
        "owner.exception.details",
        overlong.as_str(),
    ];
    for token in invalid_tokens {
        let inventory = support::inventory(
            &manifest,
            vec![support::rule(
                "resource.block_gain",
                FactsEvidenceStatus::SourceDerived,
                vec![support::input_with_kind(
                    "amount",
                    "block",
                    FactsSourceKind::GameMod,
                    Some(token),
                )],
            )],
            Vec::new(),
        );
        assert!(inventory.is_err());
    }
    Ok(())
}
