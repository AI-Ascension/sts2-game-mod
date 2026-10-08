// SPDX-License-Identifier: MIT

#[path = "support/game_facts_protocol_v1.rs"]
mod support;
#[path = "support/game_facts_unsupported_combinations.rs"]
mod unsupported_combinations;

use sts2_game_mod::{
    FactsEvidenceStatus, FactsInputAvailability, FactsSourceKind, GameFactsReferenceV1Adapter,
};
use sts2_protocol::game_facts_reference_v1::{
    Applicability, BindingMode, EvidenceStatus, GameFactsReferenceV1Codec,
    GameFactsReferenceV1Rejection, QueryResultEntry, Unit, UnsupportedReason,
};

#[test]
fn missing_provenance_and_unrepresentable_units_are_unsupported_per_rule() -> Result<(), String> {
    let manifest = support::manifest();
    let inventory = support::inventory(
        &manifest,
        vec![
            support::rule(
                "resource.no_source",
                FactsEvidenceStatus::SourceDerived,
                vec![support::input("amount", "block", None)],
            ),
            support::rule(
                "resource.unknown_unit",
                FactsEvidenceStatus::SourceDerived,
                vec![support::input("amount", "health", Some("source.health"))],
            ),
        ],
        Vec::new(),
    )
    .map_err(|error| error.to_string())?;
    let request = support::query(
        &["resource.no_source", "resource.unknown_unit"],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    let response =
        GameFactsReferenceV1Adapter::respond(&request, Some(&inventory), Some(&manifest))
            .map_err(|error| error.to_string())?;
    let results = &response
        .result
        .as_ref()
        .ok_or_else(|| "query response omitted result".to_owned())?
        .results;
    assert!(matches!(
        results[0],
        QueryResultEntry::Unsupported {
            reason_code: UnsupportedReason::MissingCapability,
            ..
        }
    ));
    assert!(matches!(
        results[1],
        QueryResultEntry::Unsupported {
            reason_code: UnsupportedReason::UnsupportedField,
            ..
        }
    ));
    Ok(())
}

#[test]
fn evidence_labels_are_preserved_and_unconfirmed_entries_stay_unsupported() -> Result<(), String> {
    let manifest = support::manifest();
    let inventory = support::inventory(
        &manifest,
        vec![
            support::rule(
                "resource.source_derived",
                FactsEvidenceStatus::SourceDerived,
                vec![support::input("amount", "block", Some("source.block"))],
            ),
            support::rule(
                "resource.proposed",
                FactsEvidenceStatus::Proposed,
                vec![support::input("amount", "block", Some("source.proposed"))],
            ),
            support::rule(
                "resource.inferred",
                FactsEvidenceStatus::Inferred,
                vec![support::input("amount", "block", Some("source.inferred"))],
            ),
            support::rule(
                "resource.unverified",
                FactsEvidenceStatus::Unverified,
                vec![support::input("amount", "block", Some("source.unverified"))],
            ),
        ],
        Vec::new(),
    )
    .map_err(|error| error.to_string())?;
    let request = support::query(
        &[
            "resource.source_derived",
            "resource.proposed",
            "resource.inferred",
            "resource.unverified",
        ],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    let response =
        GameFactsReferenceV1Adapter::respond(&request, Some(&inventory), Some(&manifest))
            .map_err(|error| error.to_string())?;
    let results = &response
        .result
        .as_ref()
        .ok_or_else(|| "query response omitted result".to_owned())?
        .results;

    assert!(matches!(
        &results[0],
        QueryResultEntry::Found {
            evidence_status: EvidenceStatus::SourceDerived,
            ..
        }
    ));
    for result in &results[1..] {
        assert!(matches!(
            result,
            QueryResultEntry::Unsupported {
                reason_code: UnsupportedReason::MissingCapability,
                ..
            }
        ));
    }
    Ok(())
}

#[test]
fn conditional_and_unknown_applicability_are_mapped_without_observations() -> Result<(), String> {
    let manifest = support::manifest();
    let inventory = support::inventory(
        &manifest,
        vec![
            support::rule(
                "resource.conditional",
                FactsEvidenceStatus::SourceDerived,
                vec![support::input_with_availability(
                    "amount",
                    "damage",
                    FactsSourceKind::GameMod,
                    FactsInputAvailability::Conditional,
                    Some("source.conditional"),
                )],
            ),
            support::rule(
                "resource.unknown_applicability",
                FactsEvidenceStatus::SourceDerived,
                vec![support::input_with_availability(
                    "amount",
                    "damage",
                    FactsSourceKind::GameMod,
                    FactsInputAvailability::Unknown,
                    Some("source.unknown"),
                )],
            ),
        ],
        Vec::new(),
    )
    .map_err(|error| error.to_string())?;
    let request = support::query(
        &["resource.conditional", "resource.unknown_applicability"],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    let response =
        GameFactsReferenceV1Adapter::respond(&request, Some(&inventory), Some(&manifest))
            .map_err(|error| error.to_string())?;
    let results = &response
        .result
        .as_ref()
        .ok_or_else(|| "query response omitted result".to_owned())?
        .results;
    let expected_applicability = [Applicability::Conditional, Applicability::Unknown];
    for (result, expected) in results.iter().zip(expected_applicability) {
        let QueryResultEntry::Found { inputs, .. } = result else {
            return Err("source-derived applicability was not mapped".to_owned());
        };
        assert_eq!(inputs[0].applicability, expected);
        assert_eq!(inputs[0].observation, None);
    }
    Ok(())
}

#[test]
fn only_exact_closed_unit_names_map_to_protocol_units() -> Result<(), String> {
    let manifest = support::manifest();
    let accepted = [
        ("damage", Unit::Damage),
        ("hit_points", Unit::HitPoints),
        ("block", Unit::Block),
        ("energy", Unit::Energy),
        ("count", Unit::Count),
        ("turns", Unit::Turns),
        ("rounds", Unit::Rounds),
        ("gold", Unit::Gold),
        ("multiplier", Unit::Multiplier),
        ("ratio", Unit::Ratio),
        ("entity", Unit::Entity),
        ("boolean", Unit::Boolean),
        ("dimensionless", Unit::Dimensionless),
    ];
    let inputs = accepted
        .iter()
        .enumerate()
        .map(|(index, (unit, _))| {
            support::input(&format!("input{index}"), unit, Some("source.units"))
        })
        .collect();
    let inventory = support::inventory(
        &manifest,
        vec![support::rule(
            "resource.units",
            FactsEvidenceStatus::SourceDerived,
            inputs,
        )],
        Vec::new(),
    )
    .map_err(|error| error.to_string())?;
    let request = support::query(
        &["resource.units"],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    let response =
        GameFactsReferenceV1Adapter::respond(&request, Some(&inventory), Some(&manifest))
            .map_err(|error| error.to_string())?;
    let QueryResultEntry::Found { inputs, .. } = &response
        .result
        .as_ref()
        .ok_or_else(|| "query response omitted result".to_owned())?
        .results[0]
    else {
        return Err("closed protocol units were not mapped".to_owned());
    };
    for (input, (_, expected)) in inputs.iter().zip(accepted) {
        assert_eq!(input.unit, expected);
    }

    let invalid_inventory = support::inventory(
        &manifest,
        vec![
            support::rule(
                "resource.alias_unit",
                FactsEvidenceStatus::SourceDerived,
                vec![support::input("amount", "health", Some("source.health"))],
            ),
            support::rule(
                "resource.case_unit",
                FactsEvidenceStatus::SourceDerived,
                vec![support::input("amount", "Damage", Some("source.damage"))],
            ),
        ],
        Vec::new(),
    )
    .map_err(|error| error.to_string())?;
    let invalid_request = support::query(
        &["resource.alias_unit", "resource.case_unit"],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    let invalid_response = GameFactsReferenceV1Adapter::respond(
        &invalid_request,
        Some(&invalid_inventory),
        Some(&manifest),
    )
    .map_err(|error| error.to_string())?;
    for result in &invalid_response
        .result
        .as_ref()
        .ok_or_else(|| "invalid-unit response omitted result".to_owned())?
        .results
    {
        assert!(matches!(
            result,
            QueryResultEntry::Unsupported {
                reason_code: UnsupportedReason::UnsupportedField,
                ..
            }
        ));
    }
    Ok(())
}

#[test]
fn duplicate_unsupported_combinations_are_not_silently_disclosed_as_valid() -> Result<(), String> {
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
        vec![
            unsupported_combinations::unsupported(
                &["resource.alpha", "resource.beta"],
                "First declaration.",
            ),
            unsupported_combinations::unsupported(
                &["resource.beta", "resource.alpha"],
                "Duplicate declaration.",
            ),
        ],
    )
    .map_err(|error| error.to_string())?;
    let request = support::query(
        &["resource.alpha", "resource.beta"],
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    let response =
        GameFactsReferenceV1Adapter::respond(&request, Some(&inventory), Some(&manifest))
            .map_err(|error| error.to_string())?;
    let result = response
        .result
        .as_ref()
        .ok_or_else(|| "duplicate-combination response omitted result".to_owned())?;
    assert!(result.unsupported_combinations.is_empty());
    assert_eq!(result.results.len(), 2);
    for entry in &result.results {
        assert!(matches!(
            entry,
            QueryResultEntry::Unsupported {
                reason_code: UnsupportedReason::UnsupportedField,
                ..
            }
        ));
    }
    Ok(())
}

#[test]
fn oversized_semantically_valid_response_is_refused_by_the_wire_byte_cap() -> Result<(), String> {
    let manifest = support::manifest();
    let mut rules = Vec::new();
    let mut rule_ids = Vec::new();
    for rule_index in 0..16 {
        let rule_id = format!("resource.rule{rule_index}");
        rule_ids.push(rule_id.clone());
        let inputs = (0..64)
            .map(|input_index| {
                let prefix = format!("i{input_index:03}");
                let name = format!("{prefix}{}", "n".repeat(128 - prefix.len()));
                let reference = "s".repeat(128);
                support::input(&name, "damage", Some(&reference))
            })
            .collect();
        rules.push(support::rule(
            &rule_id,
            FactsEvidenceStatus::SourceDerived,
            inputs,
        ));
    }
    let inventory =
        support::inventory(&manifest, rules, Vec::new()).map_err(|error| error.to_string())?;
    let requested_ids = rule_ids.iter().map(String::as_str).collect::<Vec<_>>();
    let request = support::query(
        &requested_ids,
        &manifest.inventory_revision,
        BindingMode::Static,
    );
    GameFactsReferenceV1Codec::validate(&request).map_err(|error| error.to_string())?;

    assert_eq!(
        GameFactsReferenceV1Adapter::respond(&request, Some(&inventory), Some(&manifest)),
        Err(GameFactsReferenceV1Rejection::ResultLimitExceeded)
    );
    Ok(())
}
