// SPDX-License-Identifier: MIT

use super::*;
use serde_json::{Value, json};
use std::error::Error;
use sts2_protocol::game_facts_reference_v1::{EvidenceStatus, QueryResultEntry, Value as Scalar};

#[allow(dead_code)]
#[path = "../../../../crates/game-mod/tests/support/core_facts_catalog.rs"]
mod independent_catalog;

fn source() -> Value {
    json!({"generation_before":11,"generation_after":11,"game_build":"test-build",
        "locale":"en-US","packages":[{"package_id":"base","package_version":"1","order":0}],
        "available_entity_kinds":["card"],"registry_definition_counts":{"card":1},
        "definitions":[{"entity_kind":"card","namespaced_id":"base:card:strike",
            "semantic_inputs":"{\"cost\":1}","localized_text":"{\"title\":\"Strike\"}",
            "origin":{"package_id":"base","package_version":"1"},"override_chain":[]}],
        "adapter_compatibility":"card-source-v1"})
}

fn fixture() -> Result<Value, Box<dyn Error>> {
    let source = source();
    let manifest = crate::content_manifest_ffi::produce_manifest(&serde_json::to_vec(&source)?)
        .map_err(|()| "fixture source must produce a canonical manifest")?;
    let scope = json!({"instance_id":"instance-1","run_id":"run-1","authority_epoch":3,
        "content_manifest_id":manifest.inventory_revision,"locale":"en-US"});
    let instance = json!({"instance_id":"instance-1","run_id":"run-1","epoch":7,
        "entity_kind":"card","entity_id":"card-1"});
    let snapshot =
        json!({"snapshot_id":"snapshot-1","instance_ref":instance,"state_generation":19});
    let capabilities = LiveFixedCardFacts::capabilities("corr-1")?;
    let mut request = serde_json::to_value(capabilities)?;
    request["kind"] = json!("query_request");
    request["capabilities"] = Value::Null;
    request["query"] = json!({"rules_reference_version":2,"rule_ids":["resource.fixed_card_cost"],
        "binding":{"mode":"live","content_manifest_id":manifest.inventory_revision,
            "locale":"en-US","visibility_scope":"player_visible","instance_ref":instance,
            "snapshot_ref":snapshot},"scope":scope,
        "parent_observation":{"instance_ref":instance,"snapshot_ref":snapshot,"state_generation":19}});
    Ok(
        json!({"operation":"query","correlation_id":"corr-1","request":request,"source":source,
        "observation":{"mode_id":"standard","manifest":{
            "catalog_generation":manifest.catalog_generation,
            "adapter_compatibility":manifest.adapter_compatibility,
            "content_set_revision":manifest.content_set_revision,
            "localized_text_revision":manifest.localized_text_revision,
            "inventory_revision":manifest.inventory_revision},"scope":scope,"instance_ref":instance,
            "snapshot_ref":snapshot,"energy":{"availability":"available","value":3},
            "card_cost":{"availability":"available","value":1}}}),
    )
}

fn call(bytes: &[u8], capacity: u32) -> (i32, Vec<u8>, u32) {
    let mut output = vec![0_u8; capacity as usize];
    let mut length = 99;
    // SAFETY: Live separate input/output buffers and writable length match the declared sizes.
    let status = unsafe {
        sts2_game_mod_live_facts_respond(
            bytes.as_ptr(),
            bytes.len() as u32,
            output.as_mut_ptr(),
            capacity,
            &mut length,
        )
    };
    output.truncate(length as usize);
    (status, output, length)
}

fn response(input: &Value) -> Result<Message, Box<dyn Error>> {
    let (status, bytes, _) = call(&serde_json::to_vec(input)?, MAX_OUTPUT);
    assert_eq!(status, 200);
    Ok(GameFactsReferenceV1Codec::decode(&bytes)?)
}

#[test]
fn actual_abi_manifest_and_codec_produce_only_ordered_source_derived_raw_inputs()
-> Result<(), Box<dyn Error>> {
    let input = fixture()?;
    let output = response(&input)?;
    let result = output.result.ok_or("missing result")?;
    let QueryResultEntry::Found {
        evidence_status,
        inputs,
        ..
    } = &result.results[0]
    else {
        return Err("fixed cost must be found".into());
    };
    assert_eq!(*evidence_status, EvidenceStatus::SourceDerived);
    assert_eq!(
        inputs
            .iter()
            .map(|value| value.name.as_str())
            .collect::<Vec<_>>(),
        vec!["energy", "card_cost"]
    );
    assert_eq!(
        inputs[0]
            .observation
            .as_ref()
            .and_then(|value| value.value.as_ref()),
        Some(&Scalar::Integer { value: 3 })
    );
    assert_eq!(
        inputs[1]
            .observation
            .as_ref()
            .and_then(|value| value.value.as_ref()),
        Some(&Scalar::Integer { value: 1 })
    );
    assert_eq!(
        result.inventory_binding.manifest.inventory_revision,
        input["request"]["query"]["binding"]["content_manifest_id"]
    );
    Ok(())
}

#[test]
fn unavailable_values_and_x_cost_remain_null_never_zero() -> Result<(), Box<dyn Error>> {
    for availability in [
        "unavailable",
        "not_observable",
        "unsupported",
        "missing",
        "redacted",
    ] {
        let mut input = fixture()?;
        input["observation"]["card_cost"] = json!({"availability":availability,"value":null});
        let output = serde_json::to_value(response(&input)?)?;
        assert_eq!(
            output["result"]["results"][0]["inputs"][1]["observation"]["value"],
            Value::Null
        );
        assert_eq!(
            output["result"]["results"][0]["inputs"][1]["observation"]["availability"],
            availability
        );
    }
    for invalid in [json!(-1), json!(2_147_483_648_i64), Value::Null] {
        let mut input = fixture()?;
        input["observation"]["card_cost"]["value"] = invalid;
        assert_eq!(
            serde_json::to_value(response(&input)?)?["error"]["code"],
            "invalid_bounds"
        );
    }
    Ok(())
}

#[test]
fn stale_binding_locale_generation_and_catalog_changes_refuse() -> Result<(), Box<dyn Error>> {
    for pointer in [
        "/observation/scope/authority_epoch",
        "/observation/instance_ref/epoch",
        "/observation/snapshot_ref/state_generation",
    ] {
        let mut input = fixture()?;
        *input
            .pointer_mut(pointer)
            .ok_or("missing fixture pointer")? = json!(22);
        let (status, bytes, _) = call(&serde_json::to_vec(&input)?, MAX_OUTPUT);
        assert_eq!(status, 409, "{pointer}");
        assert_eq!(
            serde_json::to_value(GameFactsReferenceV1Codec::decode(&bytes)?)?["error"]["code"],
            "stale_snapshot",
            "{pointer}"
        );
    }
    for (pointer, value) in [
        ("/source/locale", json!("fr-FR")),
        (
            "/source/definitions/0/semantic_inputs",
            json!("{\"cost\":2}"),
        ),
        (
            "/source/definitions/0/localized_text",
            json!("{\"title\":\"Different\"}"),
        ),
    ] {
        let mut input = fixture()?;
        *input
            .pointer_mut(pointer)
            .ok_or("missing fixture pointer")? = value;
        assert_eq!(
            serde_json::to_value(response(&input)?)?["error"]["code"],
            "invalid_binding",
            "{pointer}"
        );
    }
    let mut input = fixture()?;
    input["source"]["generation_after"] = json!(12);
    assert_eq!(call(&serde_json::to_vec(&input)?, MAX_OUTPUT).0, 503);
    Ok(())
}

#[test]
fn abi_refuses_unknown_fields_duplicates_invalid_kind_and_bounds() -> Result<(), Box<dyn Error>> {
    let input = fixture()?;
    let bytes = serde_json::to_vec(&input)?;
    assert_eq!(call(&bytes, 1), (500, vec![], 0));
    assert_eq!(call(b"{}", MAX_OUTPUT), (400, vec![], 0));
    let mut changed = input.clone();
    changed["extra"] = json!(true);
    assert_eq!(call(&serde_json::to_vec(&changed)?, MAX_OUTPUT).0, 400);
    changed = input.clone();
    changed["request"]["kind"] = json!("query_response");
    assert_eq!(call(&serde_json::to_vec(&changed)?, MAX_OUTPUT).0, 400);
    let duplicate = String::from_utf8(bytes)?.replace(
        "\"rules_reference_version\":2",
        "\"rules_reference_version\":2,\"rules_reference_version\":2",
    );
    assert_eq!(call(duplicate.as_bytes(), MAX_OUTPUT).0, 400);
    let mut length = 99;
    let mut output = [0_u8; 1];
    // SAFETY: Deliberately invalid/null input must be refused before dereference, output is live.
    let status = unsafe {
        sts2_game_mod_live_facts_respond(
            std::ptr::null(),
            u32::MAX,
            output.as_mut_ptr(),
            1,
            &mut length,
        )
    };
    assert_eq!((status, length), (400, 0));
    Ok(())
}

#[test]
fn other_admitted_rules_are_explicitly_unsupported_and_unknown_ids_refuse()
-> Result<(), Box<dyn Error>> {
    let mut input = fixture()?;
    let ids = independent_catalog::SIGNATURES
        .iter()
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    input["request"]["query"]["rule_ids"] = json!(ids);
    let output = response(&input)?;
    let result = output.result.ok_or("missing result")?;
    assert_eq!(result.results.len(), 16);
    for (id, entry) in ids.iter().zip(result.results) {
        if *id == "resource.fixed_card_cost" {
            assert!(matches!(
                entry,
                QueryResultEntry::Found {
                    evidence_status: EvidenceStatus::SourceDerived,
                    ..
                }
            ));
        } else {
            assert!(matches!(entry, QueryResultEntry::Unsupported { .. }));
        }
    }
    input["request"]["query"]["rule_ids"] = json!(["resource.unknown"]);
    assert_eq!(
        serde_json::to_value(response(&input)?)?["error"]["code"],
        "unknown_id"
    );
    Ok(())
}

#[test]
fn capabilities_declare_single_generation_and_absent_source_never_produces_values()
-> Result<(), Box<dyn Error>> {
    let capabilities = response(
        &json!({"operation":"capabilities","correlation_id":"corr-1",
        "request":null,"source":null,"observation":null}),
    )?;
    let policy = capabilities
        .capabilities
        .and_then(|value| value.snapshot_policy)
        .ok_or("missing live policy")?;
    assert_eq!(policy.lifetime_generations, 1);
    assert_eq!(policy.max_retained_snapshots, 1);
    assert_eq!(policy.invalidated_by.len(), 6);
    for field in ["source", "observation"] {
        let mut input = fixture()?;
        input[field] = Value::Null;
        assert_eq!(
            serde_json::to_value(response(&input)?)?["error"]["code"],
            "missing_capability"
        );
    }
    Ok(())
}

#[test]
fn fact_input_bound_has_headroom_for_the_maximum_source_and_query() {
    assert!(
        MAX_INPUT as usize > 16 * 1024 * 1024 + MAX_OUTPUT as usize,
        "composite bridge must fit source plus request and observation"
    );
}

#[test]
fn revoked_capture_has_canonical_stale_refusal_with_original_request() -> Result<(), Box<dyn Error>>
{
    let mut input = fixture()?;
    input["operation"] = json!("reject_stale");
    let populated = serde_json::to_vec(&input)?;
    assert_eq!(call(&populated, MAX_OUTPUT).0, 400);
    input["source"] = Value::Null;
    input["observation"] = Value::Null;
    let (status, bytes, _) = call(&serde_json::to_vec(&input)?, MAX_OUTPUT);
    assert_eq!(status, 409);
    let decoded = GameFactsReferenceV1Codec::decode(&bytes)?;
    assert_eq!(
        serde_json::to_value(&decoded)?["error"]["code"],
        "stale_snapshot"
    );
    let request = GameFactsReferenceV1Codec::decode(&serde_json::to_vec(&input["request"])?)?;
    sts2_protocol::game_facts_reference_v1::validate_response_against_request(&request, &decoded)?;
    input["request"]["query"]["parent_observation"]["state_generation"] = json!(20);
    assert_eq!(call(&serde_json::to_vec(&input)?, MAX_OUTPUT).0, 400);
    Ok(())
}
