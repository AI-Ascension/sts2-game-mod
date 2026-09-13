// SPDX-License-Identifier: MIT

use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate_fixture(fixture: &LocalKindFixture) -> Result<(), LocalReadError> {
    if fixture.schema.kind.is_empty() {
        return Err(LocalReadError::InvalidSchema);
    }
    for (name, definition) in &fixture.schema.fields {
        if name != &definition.name {
            return Err(LocalReadError::InvalidSchema);
        }
        if let Some(group) = &definition.detail_group
            && !fixture
                .schema
                .groups
                .get(group)
                .is_some_and(|fields| fields.contains(name))
        {
            return Err(LocalReadError::InvalidSchema);
        }
    }
    for fields in fixture.schema.groups.values() {
        if fields.is_empty()
            || fields
                .iter()
                .any(|field| !fixture.schema.fields.contains_key(field))
        {
            return Err(LocalReadError::InvalidSchema);
        }
    }
    let mut ids = BTreeSet::new();
    for entity in &fixture.entities {
        if entity.entity_id.is_empty() || !ids.insert(entity.entity_id.clone()) {
            return Err(LocalReadError::InvalidSchema);
        }
        if entity
            .values
            .keys()
            .any(|field| !fixture.schema.fields.contains_key(field))
            || entity
                .detail_bytes
                .keys()
                .any(|group| !fixture.schema.groups.contains_key(group))
        {
            return Err(LocalReadError::InvalidSchema);
        }
        for (field, value) in &entity.values {
            if let LocalFixtureValue::Value(value) = value {
                let Some(definition) = fixture.schema.fields.get(field) else {
                    return Err(LocalReadError::InvalidSchema);
                };
                if !value_matches_kind(value, definition.value_kind) {
                    return Err(LocalReadError::InvalidSchema);
                }
            }
        }
    }
    Ok(())
}

fn value_matches_kind(value: &LocalFieldValue, kind: LocalFieldValueKind) -> bool {
    matches!(
        (value, kind),
        (LocalFieldValue::Integer(_), LocalFieldValueKind::Integer)
            | (LocalFieldValue::Boolean(_), LocalFieldValueKind::Boolean)
            | (LocalFieldValue::Text(_), LocalFieldValueKind::Text)
            | (LocalFieldValue::TextList(_), LocalFieldValueKind::TextList)
    )
}

pub(super) fn build_summary(
    schema: &LocalKindSchema,
    entity: &LocalEntityFixture,
    fields: &[String],
    origin: &LocalFieldOrigin,
) -> LocalEntitySummary {
    let mut detail_groups = BTreeSet::new();
    let values = fields
        .iter()
        .filter_map(|field| schema.fields.get(field))
        .map(|definition| {
            if let Some(group) = &definition.detail_group
                && !definition.basic
            {
                detail_groups.insert(group.clone());
            }
            let result = if definition.protected {
                unavailable(
                    LocalFieldStatus::Denied,
                    LocalReasonCode::ScopeDenied,
                    origin,
                )
            } else {
                result_for_fixture(
                    definition,
                    entity.values.get(&definition.name),
                    origin,
                    definition.basic,
                )
            };
            (definition.name.clone(), result)
        })
        .collect::<BTreeMap<_, _>>();
    let detail_links = detail_groups
        .into_iter()
        .map(|field_group| LocalDetailLink {
            entity_kind: origin.source_kind.clone(),
            entity_id: entity.entity_id.clone(),
            field_group,
            origin: origin.clone(),
        })
        .collect();
    LocalEntitySummary {
        entity_kind: origin.source_kind.clone(),
        entity_id: entity.entity_id.clone(),
        fields: values,
        detail_links,
    }
}

pub(super) fn result_for_fixture(
    definition: &LocalFieldDefinition,
    fixture: Option<&LocalFixtureValue>,
    origin: &LocalFieldOrigin,
    basic: bool,
) -> LocalFieldResult {
    if !definition.supported {
        return unavailable(
            LocalFieldStatus::Unsupported,
            LocalReasonCode::UnsupportedField,
            origin,
        );
    }
    if !basic {
        return unavailable(
            LocalFieldStatus::NotObserved,
            LocalReasonCode::SurfaceNotObserved,
            origin,
        );
    }
    match fixture {
        Some(LocalFixtureValue::Value(value)) => LocalFieldResult::Available {
            value: value.clone(),
            origin: origin.clone(),
        },
        Some(LocalFixtureValue::NotApplicable) => unavailable(
            LocalFieldStatus::NotApplicable,
            LocalReasonCode::PhaseNotApplicable,
            origin,
        ),
        Some(LocalFixtureValue::NotObserved) | None => unavailable(
            LocalFieldStatus::NotObserved,
            LocalReasonCode::SurfaceNotObserved,
            origin,
        ),
        Some(LocalFixtureValue::Unknown) => unavailable(
            LocalFieldStatus::Unknown,
            LocalReasonCode::ValueUnknown,
            origin,
        ),
        Some(LocalFixtureValue::Failed) => unavailable(
            LocalFieldStatus::Failed,
            LocalReasonCode::ExtractionFailed,
            origin,
        ),
    }
}

pub(super) fn unavailable(
    status: LocalFieldStatus,
    reason: LocalReasonCode,
    origin: &LocalFieldOrigin,
) -> LocalFieldResult {
    LocalFieldResult::Unavailable {
        status,
        reason,
        origin: origin.clone(),
    }
}

/// Measures the synthetic `<field>=<payload>\n` detail representation in UTF-8 bytes.
pub(super) fn measure_detail_bytes(
    schema: &LocalKindSchema,
    entity: &LocalEntityFixture,
    fields: &BTreeSet<String>,
) -> Result<usize, LocalReadError> {
    fields.iter().try_fold(0usize, |total, field| {
        let definition = schema
            .fields
            .get(field)
            .ok_or(LocalReadError::InvalidSchema)?;
        let payload = if definition.protected {
            "denied".len()
        } else if !definition.supported {
            "unsupported".len()
        } else {
            fixture_payload_bytes(entity.values.get(field))
        };
        total
            .checked_add(definition.name.len())
            .and_then(|size| size.checked_add(2))
            .and_then(|size| size.checked_add(payload))
            .ok_or(LocalReadError::DetailSizeUnavailable)
    })
}

fn fixture_payload_bytes(fixture: Option<&LocalFixtureValue>) -> usize {
    match fixture {
        Some(LocalFixtureValue::Value(value)) => value_bytes(value),
        Some(LocalFixtureValue::NotApplicable) => "not_applicable".len(),
        Some(LocalFixtureValue::NotObserved) | None => "not_observed".len(),
        Some(LocalFixtureValue::Unknown) => "unknown".len(),
        Some(LocalFixtureValue::Failed) => "failed".len(),
    }
}

fn value_bytes(value: &LocalFieldValue) -> usize {
    match value {
        LocalFieldValue::Integer(value) => value.to_string().len(),
        LocalFieldValue::Boolean(value) => value.to_string().len(),
        LocalFieldValue::Text(value) => value.len(),
        LocalFieldValue::TextList(values) => 2usize
            .saturating_add(values.iter().map(String::len).sum::<usize>())
            .saturating_add(values.len().saturating_sub(1)),
    }
}

pub(super) fn field_status(
    definition: &LocalFieldDefinition,
    fixture: Option<&LocalFixtureValue>,
    basic: bool,
) -> LocalFieldStatus {
    if definition.protected {
        return LocalFieldStatus::Denied;
    }
    result_for_fixture(definition, fixture, &dummy_origin(), basic).status()
}

pub(super) fn aggregate_status(
    statuses: impl Iterator<Item = LocalFieldStatus>,
) -> LocalFieldStatus {
    let mut aggregate = LocalFieldStatus::Available;
    let mut observed = false;
    for status in statuses {
        observed = true;
        if status != LocalFieldStatus::Available && status_rank(status) > status_rank(aggregate) {
            aggregate = status;
        }
    }
    if observed {
        aggregate
    } else {
        LocalFieldStatus::NotObserved
    }
}

fn status_rank(status: LocalFieldStatus) -> u8 {
    match status {
        LocalFieldStatus::Available => 0,
        LocalFieldStatus::NotApplicable => 1,
        LocalFieldStatus::NotObserved => 2,
        LocalFieldStatus::Unknown => 3,
        LocalFieldStatus::Unsupported => 4,
        LocalFieldStatus::Denied => 5,
        LocalFieldStatus::Stale => 6,
        LocalFieldStatus::Failed => 7,
    }
}

fn dummy_origin() -> LocalFieldOrigin {
    LocalFieldOrigin::new("support-report", LocalReadReference::new("", "", 0))
}
