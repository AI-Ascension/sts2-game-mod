// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use sts2_game_mod::{
    LocalAvailabilityStore, LocalEntityFixture, LocalFieldDefinition, LocalFieldResult,
    LocalFieldValue, LocalFieldValueKind, LocalFixtureValue, LocalKindFixture, LocalKindSchema,
    LocalReadError, LocalReadReference,
};

pub fn reference() -> LocalReadReference {
    LocalReadReference::new("content-1", "snapshot-1", 7)
}

pub fn entity<K, D>(
    entity_id: &str,
    values: impl IntoIterator<Item = (K, LocalFixtureValue)>,
    detail_bytes: impl IntoIterator<Item = (D, usize)>,
) -> LocalEntityFixture
where
    K: Into<String>,
    D: Into<String>,
{
    LocalEntityFixture {
        entity_id: entity_id.to_owned(),
        values: values
            .into_iter()
            .map(|(field, value)| (field.into(), value))
            .collect(),
        detail_bytes: detail_bytes
            .into_iter()
            .map(|(group, bytes)| (group.into(), bytes))
            .collect(),
    }
}

pub fn schema() -> LocalKindSchema {
    let mut detail = LocalFieldDefinition::new("detail_text", LocalFieldValueKind::Text);
    detail.basic = false;
    detail.detail_group = Some("details".to_owned());

    let mut health = LocalFieldDefinition::new("health", LocalFieldValueKind::Integer);
    health.required = true;

    let mut unsupported = LocalFieldDefinition::new("unsupported", LocalFieldValueKind::Boolean);
    unsupported.supported = false;

    let mut denied = LocalFieldDefinition::new("denied", LocalFieldValueKind::Text);
    denied.protected = true;
    denied.required = true;

    LocalKindSchema::new("synthetic_entity")
        .with_field(health)
        .with_field(LocalFieldDefinition::new(
            "tags",
            LocalFieldValueKind::TextList,
        ))
        .with_field(detail)
        .with_field(LocalFieldDefinition::new(
            "not_applicable",
            LocalFieldValueKind::Text,
        ))
        .with_field(LocalFieldDefinition::new(
            "hidden",
            LocalFieldValueKind::Text,
        ))
        .with_field(unsupported)
        .with_field(denied)
        .with_field(LocalFieldDefinition::new(
            "failed",
            LocalFieldValueKind::Text,
        ))
        .with_field(LocalFieldDefinition::new(
            "unknown",
            LocalFieldValueKind::Text,
        ))
        .with_group("details", ["detail_text"])
}

pub fn store(total_known: bool) -> Result<LocalAvailabilityStore, LocalReadError> {
    let fixture = LocalKindFixture::new(
        schema(),
        [
            entity(
                "entity-a",
                [
                    (
                        "health",
                        LocalFixtureValue::Value(LocalFieldValue::Integer(0)),
                    ),
                    (
                        "tags",
                        LocalFixtureValue::Value(LocalFieldValue::TextList(Vec::new())),
                    ),
                    (
                        "detail_text",
                        LocalFixtureValue::Value(LocalFieldValue::Text("recovered".to_owned())),
                    ),
                    ("not_applicable", LocalFixtureValue::NotApplicable),
                    ("hidden", LocalFixtureValue::NotObserved),
                    ("failed", LocalFixtureValue::Failed),
                    ("unknown", LocalFixtureValue::Unknown),
                ],
                [("details", 24)],
            ),
            entity(
                "entity-b",
                [
                    (
                        "health",
                        LocalFixtureValue::Value(LocalFieldValue::Integer(3)),
                    ),
                    (
                        "tags",
                        LocalFixtureValue::Value(LocalFieldValue::TextList(vec![
                            "visible".to_owned(),
                        ])),
                    ),
                    (
                        "detail_text",
                        LocalFixtureValue::Value(LocalFieldValue::Text("too large".to_owned())),
                    ),
                ],
                [("details", 128)],
            ),
        ],
        total_known,
    );
    let mut store = LocalAvailabilityStore::new(reference(), 2, 64)?;
    store.add_kind(fixture)?;
    Ok(store)
}

pub fn store_with_detail_bytes(
    detail_bytes: BTreeMap<String, usize>,
) -> Result<LocalAvailabilityStore, LocalReadError> {
    store_with_detail_value(detail_bytes, "bounded")
}

pub fn store_with_detail_value(
    detail_bytes: BTreeMap<String, usize>,
    detail_text: impl Into<String>,
) -> Result<LocalAvailabilityStore, LocalReadError> {
    let fixture = LocalKindFixture::new(
        schema(),
        [entity(
            "entity-a",
            [(
                "detail_text",
                LocalFixtureValue::Value(LocalFieldValue::Text(detail_text.into())),
            )],
            detail_bytes,
        )],
        true,
    );
    let mut store = LocalAvailabilityStore::new(reference(), 2, 64)?;
    store.add_kind(fixture)?;
    Ok(store)
}

pub fn field<'a>(
    fields: &'a BTreeMap<String, LocalFieldResult>,
    name: &str,
) -> Result<&'a LocalFieldResult, LocalReadError> {
    fields.get(name).ok_or(LocalReadError::UnknownField)
}
