// SPDX-License-Identifier: MIT

#[allow(dead_code)]
#[path = "support/game_facts_protocol_v1.rs"]
mod support;

use std::error::Error;
use sts2_game_core::{RuleId, rule_inventory};
use sts2_game_mod::{
    LiveFactsField, LiveFactsFieldStatus as Status, OwnedLiveFactsCard, OwnedLiveFactsSnapshot,
    OwnedLiveGameFactsProducer as Producer,
};
use sts2_protocol::game_facts_reference_v1::{
    BindingMode, EntityKind, ErrorCode, EvidenceStatus, GameFactsReferenceV1Codec as Codec,
    Message, QueryResultEntry, Value,
};

fn fixture() -> Result<(Message, OwnedLiveFactsSnapshot), Box<dyn Error>> {
    let mut request = support::query(
        &[RuleId::FixedCardCost.as_str()],
        "inventory-fixture",
        BindingMode::Live,
    );
    let query = request.query.as_mut().ok_or("missing query")?;
    let instance = query
        .binding
        .instance_ref
        .as_mut()
        .ok_or("missing instance")?;
    instance.entity_kind = EntityKind::Card;
    instance.entity_id = "card-occurrence".to_owned();
    let snapshot = query
        .binding
        .snapshot_ref
        .as_mut()
        .ok_or("missing snapshot")?;
    snapshot.instance_ref = instance.clone();
    let parent = query.parent_observation.as_mut().ok_or("missing parent")?;
    parent.instance_ref = instance.clone();
    parent.snapshot_ref = snapshot.clone();
    let source = OwnedLiveFactsSnapshot {
        scope: query.scope.clone().ok_or("missing scope")?,
        mode_id: "standard".to_owned(),
        catalog_generation: 7,
        snapshot_id: snapshot.snapshot_id.clone(),
        epoch: 1,
        state_generation: 1,
        player_energy: available(300),
        cards: vec![OwnedLiveFactsCard {
            instance_id: "card-occurrence".to_owned(),
            definition_id: "card.fixture".to_owned(),
            definition_manifest: "inventory-fixture".to_owned(),
            resolved_cost: available(270),
            fixed_energy_cost: available(true),
        }],
    };
    Ok((request, source))
}

fn available<T>(value: T) -> LiveFactsField<T> {
    LiveFactsField {
        status: Status::Available,
        value: Some(value),
    }
}

fn response(request: &Message, source: &OwnedLiveFactsSnapshot) -> Result<Message, Box<dyn Error>> {
    Ok(Codec::decode(&Producer::respond(
        &Codec::encode(request)?,
        &request.correlation_id,
        source,
        &support::manifest(),
    )?)?)
}

#[test]
fn copies_unclamped_inputs_in_actual_core_order_without_promoting_evidence()
-> Result<(), Box<dyn Error>> {
    let (request, mut source) = fixture()?;
    for (energy, cost) in [(300, 270), (0, 0), (i32::MAX, i32::MAX)] {
        source.player_energy = available(energy);
        source.cards[0].resolved_cost = available(cost);
        let result = response(&request, &source)?
            .result
            .ok_or("missing result")?;
        let QueryResultEntry::Found {
            evidence_status,
            inputs,
            ..
        } = &result.results[0]
        else {
            return Err("expected owned input observations".into());
        };
        assert_eq!(*evidence_status, EvidenceStatus::SourceDerived);
        let declared = rule_inventory()
            .iter()
            .find(|rule| rule.id == RuleId::FixedCardCost)
            .ok_or("Core fixed cost missing")?;
        assert_eq!(
            inputs
                .iter()
                .map(|input| input.name.as_str())
                .collect::<Vec<_>>(),
            declared
                .inputs
                .iter()
                .map(|input| input.name)
                .collect::<Vec<_>>()
        );
        for (input, value) in inputs.iter().zip([energy, cost]) {
            assert_eq!(
                input.observation.as_ref().and_then(|v| v.value.as_ref()),
                Some(&Value::Integer {
                    value: i64::from(value)
                })
            );
            assert!(
                input
                    .source_ref
                    .r#ref
                    .as_deref()
                    .is_some_and(|value| value.starts_with("owned-card-facts-v1."))
            );
        }
        assert_eq!(result.inventory_binding.build_id, "build-fixture");
        assert_eq!(result.inventory_binding.mode_id, "standard");
    }
    Ok(())
}

#[test]
fn missing_and_variable_costs_remain_explicit_and_never_become_zero() -> Result<(), Box<dyn Error>>
{
    let (request, mut source) = fixture()?;
    for status in [
        Status::NotObserved,
        Status::Unsupported,
        Status::Unknown,
        Status::Failed,
    ] {
        source.player_energy = LiveFactsField {
            status,
            value: None,
        };
        source.cards[0].resolved_cost = LiveFactsField {
            status,
            value: None,
        };
        let result = response(&request, &source)?
            .result
            .ok_or("missing result")?;
        let QueryResultEntry::Found { inputs, .. } = &result.results[0] else {
            return Err("missing inputs".into());
        };
        assert!(inputs.iter().all(|input| {
            input
                .observation
                .as_ref()
                .is_some_and(|o| o.value.is_none() && o.reason.is_some())
        }));
    }
    source.cards[0].resolved_cost = available(17);
    source.cards[0].fixed_energy_cost = available(false);
    let result = response(&request, &source)?
        .result
        .ok_or("missing result")?;
    let QueryResultEntry::Found { inputs, .. } = &result.results[0] else {
        return Err("missing inputs".into());
    };
    assert_eq!(
        inputs[1]
            .observation
            .as_ref()
            .and_then(|o| o.reason.as_deref()),
        Some("variable_energy_cost")
    );
    assert!(
        inputs[1]
            .observation
            .as_ref()
            .is_some_and(|o| o.value.is_none())
    );
    Ok(())
}

#[test]
fn rejects_source_and_request_fences_before_publishing() -> Result<(), Box<dyn Error>> {
    let (request, source) = fixture()?;
    let mut cases = Vec::new();
    let mut changed = source.clone();
    changed.scope.authority_epoch += 1;
    cases.push((changed, ErrorCode::DeniedScope));
    let mut changed = source.clone();
    changed.scope.run_id = "native-run-is-not-harness-run".to_owned();
    cases.push((changed, ErrorCode::DeniedScope));
    let mut changed = source.clone();
    changed.epoch += 1;
    cases.push((changed, ErrorCode::StaleSnapshot));
    let mut changed = source.clone();
    changed.state_generation += 1;
    cases.push((changed, ErrorCode::StaleSnapshot));
    let mut changed = source.clone();
    changed.catalog_generation += 1;
    cases.push((changed, ErrorCode::InvalidBinding));
    let mut changed = source.clone();
    changed.cards[0].definition_manifest = "changed".to_owned();
    cases.push((changed, ErrorCode::InvalidBinding));
    let mut changed = source.clone();
    changed.cards[0].definition_id = "invented".to_owned();
    cases.push((changed, ErrorCode::InvalidBinding));
    let mut changed = source.clone();
    changed.cards.push(changed.cards[0].clone());
    cases.push((changed, ErrorCode::InvalidBinding));
    let mut changed = source.clone();
    changed.player_energy = available(-1);
    cases.push((changed, ErrorCode::MissingCapability));
    let mut changed = source.clone();
    changed.player_energy = LiveFactsField {
        status: Status::Stale,
        value: None,
    };
    cases.push((changed, ErrorCode::StaleSnapshot));
    for (changed, code) in cases {
        assert_eq!(
            response(&request, &changed)?.error.map(|e| e.code),
            Some(code)
        );
    }
    let bytes = Codec::encode(&request)?;
    assert!(Producer::admit(&bytes, "wrong-correlation").is_err());
    assert!(
        Producer::admit(
            br#"{"kind":"query_request","kind":"query_request"}"#,
            &request.correlation_id
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn known_unsupported_families_are_ordered_and_unknown_ids_are_refused() -> Result<(), Box<dyn Error>>
{
    let (mut request, source) = fixture()?;
    request.query.as_mut().ok_or("missing query")?.rule_ids = rule_inventory()
        .iter()
        .map(|rule| rule.id.as_str().to_owned())
        .collect();
    let result = response(&request, &source)?
        .result
        .ok_or("missing result")?;
    assert_eq!(result.results.len(), rule_inventory().len());
    for (entry, declared) in result.results.iter().zip(rule_inventory()) {
        match entry {
            QueryResultEntry::Found { rule_id, .. } => {
                assert_eq!(rule_id, RuleId::FixedCardCost.as_str())
            }
            QueryResultEntry::Unsupported { rule_id, .. } => {
                assert_eq!(rule_id, declared.id.as_str())
            }
        }
    }
    request.query.as_mut().ok_or("missing query")?.rule_ids = vec!["invented.rule".to_owned()];
    assert_eq!(
        response(&request, &source)?.error.map(|e| e.code),
        Some(ErrorCode::UnknownId)
    );
    let capabilities = Codec::decode(&Producer::capabilities("facts-capabilities")?)?;
    assert_eq!(
        capabilities
            .capabilities
            .and_then(|c| c.snapshot_policy)
            .map(|p| p.max_retained_snapshots),
        Some(1)
    );
    Ok(())
}
