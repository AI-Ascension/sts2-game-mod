// SPDX-License-Identifier: MIT

//! Owned-value mapping for the managed owner-thread fixed-cost producer.
//! Authentication and fresh host ownership checks remain at the managed source boundary.

use super::protocol_v1::responses;
use crate::ContentManifest;
use serde::Deserialize;
use sts2_game_core::rule_inventory;
use sts2_protocol::game_facts_reference_v1::*;

/// A raw scalar copied on the owner thread, never a calculated rule result.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveFixedCardValue {
    pub availability: Availability,
    #[serde(deserialize_with = "required_value")]
    pub value: Option<i64>,
}

fn required_value<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<i64>, D::Error> {
    Option::<i64>::deserialize(deserializer)
}

/// Owned capture associated with an authenticated retained lookup binding by the managed owner.
/// This structure alone is not an authentication token and must not be supplied by HTTP callers.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveFixedCardObservation {
    pub mode_id: String,
    pub manifest: InventoryManifest,
    pub scope: AuthenticatedScope,
    pub instance_ref: InstanceRef,
    pub snapshot_ref: SnapshotRef,
    pub energy: LiveFixedCardValue,
    pub card_cost: LiveFixedCardValue,
}

/// Bounded mapper for exactly the two owned inputs of Core's fixed-card-cost record.
#[derive(Clone, Copy, Debug)]
pub struct LiveFixedCardFacts;

impl LiveFixedCardFacts {
    /// Canonically refuses a valid request after the managed owner revokes its retained capture.
    pub fn reject_stale(request: &Message) -> Result<Message, GameFactsReferenceV1Rejection> {
        GameFactsReferenceV1Codec::validate(request)?;
        if request.kind != MessageKind::QueryRequest {
            return Err(GameFactsReferenceV1Rejection::InvalidRequest);
        }
        let query = request
            .query
            .as_ref()
            .ok_or(GameFactsReferenceV1Rejection::Malformed)?;
        let mut response =
            responses::error_response(&request.correlation_id, query, ErrorCode::StaleSnapshot);
        if let Some(error) = response.error.as_mut() {
            error.reason = Some("The selected owned snapshot is no longer current.".to_owned());
        }
        GameFactsReferenceV1Codec::validate(&response)?;
        validate_response_against_request(request, &response)?;
        Ok(response)
    }

    /// Declares a single retained generation with fail-closed lifecycle invalidation.
    pub fn capabilities(correlation: &str) -> Result<Message, GameFactsReferenceV1Rejection> {
        let mut message = responses::capabilities_response(correlation);
        if let Some(capabilities) = message.capabilities.as_mut() {
            capabilities.snapshot_policy = Some(SnapshotPolicy {
                supports_live: true,
                lifetime_generations: 1,
                max_retained_snapshots: 1,
                expiry_behavior: ExpiryBehavior::RejectStaleSnapshot,
                invalidated_by: vec![
                    SnapshotInvalidator::Restore,
                    SnapshotInvalidator::Restart,
                    SnapshotInvalidator::ProfileChange,
                    SnapshotInvalidator::ContentChange,
                    SnapshotInvalidator::RunChange,
                    SnapshotInvalidator::EpochChange,
                ],
            });
        }
        GameFactsReferenceV1Codec::validate(&message)?;
        Ok(message)
    }

    /// Maps a freshly checked capture; no query may refresh or advance its selected snapshot.
    pub fn respond(
        request: &Message,
        manifest: Option<&ContentManifest>,
        observation: Option<&LiveFixedCardObservation>,
    ) -> Result<Message, GameFactsReferenceV1Rejection> {
        GameFactsReferenceV1Codec::validate(request)?;
        if request.kind != MessageKind::QueryRequest {
            return Err(GameFactsReferenceV1Rejection::InvalidRequest);
        }
        let query = request
            .query
            .as_ref()
            .ok_or(GameFactsReferenceV1Rejection::Malformed)?;
        let response = match Self::map(query, manifest, observation) {
            Ok(result) => responses::query_response(&request.correlation_id, query, result),
            Err(code) => {
                let mut response = responses::error_response(&request.correlation_id, query, code);
                if let Some(error) = response.error.as_mut() {
                    error.reason =
                        Some("Owned live facts are unavailable for this binding.".to_owned());
                }
                response
            }
        };
        GameFactsReferenceV1Codec::validate(&response)?;
        validate_response_against_request(request, &response)?;
        Ok(response)
    }

    fn map(
        query: &Query,
        manifest: Option<&ContentManifest>,
        observation: Option<&LiveFixedCardObservation>,
    ) -> Result<QueryResult, ErrorCode> {
        if query.binding.mode != BindingMode::Live {
            return Err(ErrorCode::MissingCapability);
        }
        let manifest = manifest.ok_or(ErrorCode::MissingCapability)?;
        let captured = observation.ok_or(ErrorCode::MissingCapability)?;
        if query.binding.visibility_scope.as_deref() != Some("player_visible") {
            return Err(ErrorCode::DeniedScope);
        }
        if manifest.inventory_revision != query.binding.content_manifest_id
            || manifest.locale != query.binding.locale
            || captured.manifest.catalog_generation != manifest.catalog_generation
            || captured.manifest.adapter_compatibility != manifest.adapter_compatibility
            || captured.manifest.content_set_revision != manifest.content_set_revision
            || captured.manifest.localized_text_revision != manifest.localized_text_revision
            || captured.manifest.inventory_revision != manifest.inventory_revision
        {
            return Err(ErrorCode::InvalidBinding);
        }
        if query.scope.as_ref() != Some(&captured.scope)
            || query.binding.instance_ref.as_ref() != Some(&captured.instance_ref)
            || query.binding.snapshot_ref.as_ref() != Some(&captured.snapshot_ref)
            || captured.instance_ref.entity_kind != EntityKind::Card
        {
            return Err(ErrorCode::StaleSnapshot);
        }
        let mut results = Vec::with_capacity(query.rule_ids.len());
        for id in &query.rule_ids {
            if !rule_inventory().iter().any(|rule| rule.id.as_str() == id) {
                return Err(ErrorCode::UnknownId);
            }
            if id != "resource.fixed_card_cost" {
                results.push(QueryResultEntry::Unsupported {
                    rule_id: id.clone(),
                    reason_code: UnsupportedReason::MissingCapability,
                });
                continue;
            }
            // Admission against the real Core catalog is deliberate: a changed signature cannot
            // silently acquire guessed inputs. Only these two raw, ordered energy values exist.
            let inventory = super::CoreFactsInventory::new_for_manifest(
                manifest,
                captured.mode_id.clone(),
                vec![super::FactsRuleEntry {
                    rule_id: id.clone(),
                    evidence: super::FactsEvidenceStatus::SourceDerived,
                    inputs: ["energy", "card_cost"]
                        .into_iter()
                        .map(|name| super::FactsRuleInput {
                            name: name.to_owned(),
                            unit: "energy".to_owned(),
                            availability: super::FactsInputAvailability::Required,
                            source: Some(super::FactsInputSource {
                                kind: super::FactsSourceKind::GameMod,
                                reference: format!("live-fixed-card-{name}-v1"),
                            }),
                        })
                        .collect(),
                }],
                vec![],
            )
            .map_err(|_| ErrorCode::MissingCapability)?;
            let rule = inventory
                .as_inventory()
                .rules()
                .first()
                .ok_or(ErrorCode::MissingCapability)?;
            let inputs = rule
                .inputs
                .iter()
                .zip([&captured.energy, &captured.card_cost])
                .map(|(input, value)| {
                    Ok(RuleInput {
                        name: input.name.clone(),
                        unit: Unit::Energy,
                        applicability: Applicability::Required,
                        observation: Some(raw_observation(value)?),
                        source_ref: SourceRef {
                            kind: SourceKind::GameMod,
                            r#ref: input.source.as_ref().map(|source| source.reference.clone()),
                        },
                    })
                })
                .collect::<Result<Vec<_>, ErrorCode>>()?;
            results.push(QueryResultEntry::Found {
                rule_id: id.clone(),
                evidence_status: EvidenceStatus::SourceDerived,
                inputs,
            });
        }
        Ok(QueryResult {
            producer_version: super::GAME_FACTS_REFERENCE_PRODUCER_VERSION.to_owned(),
            rules_reference_version: query.rules_reference_version,
            inventory_binding: InventoryBinding {
                build_id: manifest.game_build.clone(),
                mode_id: captured.mode_id.clone(),
                manifest: InventoryManifest {
                    catalog_generation: manifest.catalog_generation,
                    adapter_compatibility: manifest.adapter_compatibility.clone(),
                    content_set_revision: manifest.content_set_revision.clone(),
                    localized_text_revision: manifest.localized_text_revision.clone(),
                    inventory_revision: manifest.inventory_revision.clone(),
                },
            },
            results,
            unsupported_combinations: vec![],
        })
    }
}

fn raw_observation(input: &LiveFixedCardValue) -> Result<Observation, ErrorCode> {
    if input.availability == Availability::Available {
        // Negative/sentinel and oversized values are not fixed nonnegative costs or energy.
        let value = input
            .value
            .filter(|value| (0..=i64::from(i32::MAX)).contains(value))
            .ok_or(ErrorCode::InvalidBounds)?;
        Ok(Observation {
            availability: Availability::Available,
            value: Some(Value::Integer { value }),
            reason: None,
        })
    } else if input.value.is_none() {
        Ok(Observation {
            availability: input.availability,
            value: None,
            reason: Some("owner_field_unavailable".to_owned()),
        })
    } else {
        Err(ErrorCode::InvalidBounds)
    }
}
