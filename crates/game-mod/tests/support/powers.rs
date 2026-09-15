// SPDX-License-Identifier: MIT

use crate::content_fixture::ManifestSource;
use sts2_game_mod::{
    ContentDefinitionInput, ContentManifest, ContentManifestProducer, ContentOriginInput,
    ContentUnlockState, POWER_STATUS_PRODUCER_VERSION, PowerStatusAmount,
    PowerStatusAmountDefinition, PowerStatusCap, PowerStatusCatalog, PowerStatusCatalogSnapshot,
    PowerStatusCatalogSource, PowerStatusCategory, PowerStatusCounterDefinition,
    PowerStatusCounterValue, PowerStatusDecayDefinition, PowerStatusDecayRule,
    PowerStatusDefinitionInput, PowerStatusDurationDefinition, PowerStatusDurationRule,
    PowerStatusFamilyCoverage, PowerStatusFamilyState, PowerStatusField, PowerStatusInstanceInput,
    PowerStatusKind, PowerStatusLiveBinding, PowerStatusOrigin, PowerStatusOwner,
    PowerStatusOwnerId, PowerStatusOwnerKind, PowerStatusPendingExpiry, PowerStatusReferenceKind,
    PowerStatusSemanticReference, PowerStatusSourceKind, PowerStatusSourceReference,
    PowerStatusStackingDefinition, PowerStatusStackingPolicy, PowerStatusUnit,
    PowerStatusVisibility,
};

#[derive(Clone, Debug)]
pub struct CatalogSource {
    pub snapshot: Result<PowerStatusCatalogSnapshot, sts2_game_mod::PowerStatusSourceError>,
}

impl PowerStatusCatalogSource for CatalogSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<PowerStatusCatalogSnapshot, sts2_game_mod::PowerStatusSourceError> {
        self.snapshot.clone()
    }
}

pub fn unit(value: &str) -> PowerStatusUnit {
    PowerStatusUnit::new(value).expect("unit")
}

pub fn category(value: &str) -> PowerStatusCategory {
    PowerStatusCategory::new(value).expect("category")
}

pub fn owner_id(value: &str) -> PowerStatusOwnerId {
    PowerStatusOwnerId::new(value).expect("owner id")
}

pub fn manifest() -> ContentManifest {
    let mut snapshot = crate::content_fixture::catalog_snapshot();
    snapshot
        .available_entity_kinds
        .push("power_status".to_owned());
    snapshot
        .registry_definition_counts
        .insert("power_status".to_owned(), 6);
    for id in [
        "mod:synthetic:strength",
        "mod:synthetic:vulnerable",
        "mod:synthetic:poison",
        "mod:synthetic:multi",
        "mod:synthetic:marker",
        "mod:synthetic:hidden",
    ] {
        snapshot.definitions.push(ContentDefinitionInput {
            entity_kind: "power_status".to_owned(),
            namespaced_id: id.to_owned(),
            semantic_inputs: format!("kind=power_status;id={id}"),
            localized_text: Some(id.to_owned()),
            origin: ContentOriginInput {
                package_id: Some("mod:synthetic".to_owned()),
                package_version: None,
            },
            override_chain: Vec::new(),
        });
    }
    ContentManifestProducer::new("adapter-v1", ["card".to_owned()])
        .expect("manifest producer")
        .produce(&ManifestSource { snapshot })
        .expect("manifest")
}

fn origin() -> PowerStatusOrigin {
    PowerStatusOrigin {
        kind: "fixture".to_owned(),
        package_id: Some("mod:synthetic".to_owned()),
        package_version: None,
    }
}

fn references() -> Vec<PowerStatusSemanticReference> {
    vec![
        PowerStatusSemanticReference {
            kind: PowerStatusReferenceKind::Effect,
            id: "effect:fixture".to_owned(),
            label: "Fixture effect".to_owned(),
        },
        PowerStatusSemanticReference {
            kind: PowerStatusReferenceKind::Keyword,
            id: "keyword:duration".to_owned(),
            label: "Duration".to_owned(),
        },
        PowerStatusSemanticReference {
            kind: PowerStatusReferenceKind::Rule,
            id: "rule:fixture".to_owned(),
            label: "Fixture rule".to_owned(),
        },
    ]
}

fn duration_counter(unit_name: &str, default: i64) -> PowerStatusDurationDefinition {
    PowerStatusDurationDefinition {
        default: Some(default),
        rule: PowerStatusDurationRule::Counter {
            unit: unit(unit_name),
        },
        reset: sts2_game_mod::PowerStatusReset::Turn,
    }
}

fn permanent_duration() -> PowerStatusDurationDefinition {
    PowerStatusDurationDefinition {
        default: None,
        rule: PowerStatusDurationRule::Permanent,
        reset: sts2_game_mod::PowerStatusReset::Never,
    }
}

fn no_decay() -> PowerStatusDecayDefinition {
    PowerStatusDecayDefinition {
        rule: PowerStatusDecayRule::None,
        reset: sts2_game_mod::PowerStatusReset::Never,
    }
}

fn input(
    id: &str,
    kind: PowerStatusKind,
    amount: PowerStatusAmountDefinition,
    duration: PowerStatusDurationDefinition,
    unlock_state: ContentUnlockState,
    visibility: PowerStatusVisibility,
) -> PowerStatusDefinitionInput {
    PowerStatusDefinitionInput {
        definition_id: id.to_owned(),
        kind,
        category: category("combat"),
        title: id.rsplit(':').next().expect("title").to_owned(),
        description: "Static timing and stacking rules remain separate from live values."
            .to_owned(),
        origin: origin(),
        visibility,
        unlock_state,
        amount,
        stacking: PowerStatusStackingDefinition {
            policy: PowerStatusStackingPolicy::Additive,
            cap: Some(PowerStatusCap {
                minimum: Some(0),
                maximum: Some(99),
                unit: unit("count"),
            }),
        },
        duration,
        decay: no_decay(),
        references: references(),
    }
}

pub fn definitions() -> Vec<PowerStatusDefinitionInput> {
    let mut definitions = vec![
        input(
            "mod:synthetic:strength",
            PowerStatusKind::Power,
            PowerStatusAmountDefinition::Integer {
                unit: unit("count"),
            },
            duration_counter("turn", 2),
            ContentUnlockState::Unlocked,
            PowerStatusVisibility::Visible,
        ),
        input(
            "mod:synthetic:vulnerable",
            PowerStatusKind::Debuff,
            PowerStatusAmountDefinition::Amountless,
            duration_counter("turn", 1),
            ContentUnlockState::Unlocked,
            PowerStatusVisibility::Visible,
        ),
        input(
            "mod:synthetic:poison",
            PowerStatusKind::Status,
            PowerStatusAmountDefinition::Decimal {
                unit: unit("damage"),
                scale: 1,
            },
            duration_counter("round", 3),
            ContentUnlockState::Unlocked,
            PowerStatusVisibility::Visible,
        ),
        input(
            "mod:synthetic:multi",
            PowerStatusKind::Status,
            PowerStatusAmountDefinition::Counters(vec![
                PowerStatusCounterDefinition {
                    id: "stacks".to_owned(),
                    label: "Stacks".to_owned(),
                    unit: unit("count"),
                    reset: sts2_game_mod::PowerStatusReset::Combat,
                    visibility: PowerStatusVisibility::Visible,
                    cap: Some(9),
                },
                PowerStatusCounterDefinition {
                    id: "turns".to_owned(),
                    label: "Turns".to_owned(),
                    unit: unit("turn"),
                    reset: sts2_game_mod::PowerStatusReset::Turn,
                    visibility: PowerStatusVisibility::Visible,
                    cap: None,
                },
            ]),
            duration_counter("turn", 4),
            ContentUnlockState::Unlocked,
            PowerStatusVisibility::Visible,
        ),
        input(
            "mod:synthetic:marker",
            PowerStatusKind::Stance,
            PowerStatusAmountDefinition::Boolean {
                unit: unit("active"),
            },
            permanent_duration(),
            ContentUnlockState::Unlocked,
            PowerStatusVisibility::Visible,
        ),
        input(
            "mod:synthetic:hidden",
            PowerStatusKind::Custom("secret".to_owned()),
            PowerStatusAmountDefinition::Text {
                unit: unit("label"),
            },
            permanent_duration(),
            ContentUnlockState::Locked,
            PowerStatusVisibility::OwnerOnly,
        ),
    ];
    definitions[4].stacking.policy = PowerStatusStackingPolicy::NonStacking;
    definitions[2].decay = PowerStatusDecayDefinition {
        rule: PowerStatusDecayRule::By {
            amount: 1,
            unit: unit("damage"),
        },
        reset: sts2_game_mod::PowerStatusReset::Round,
    };
    definitions[2]
        .stacking
        .cap
        .as_mut()
        .expect("poison cap")
        .unit = unit("damage");
    definitions
}

pub fn catalog() -> PowerStatusCatalog {
    let manifest = manifest();
    let definitions = definitions();
    let source = CatalogSource {
        snapshot: Ok(PowerStatusCatalogSnapshot {
            manifest: manifest.cursor_binding(),
            locale: manifest.locale.clone(),
            producer_version: POWER_STATUS_PRODUCER_VERSION.to_owned(),
            family: PowerStatusFamilyCoverage {
                entity_kind: "power_status".to_owned(),
                state: PowerStatusFamilyState::Handled,
                definition_count: definitions.len(),
            },
            definitions,
        }),
    };
    sts2_game_mod::PowerStatusCatalogProducer::new()
        .produce(&manifest, &source)
        .expect("catalog")
}

pub fn binding(catalog: &PowerStatusCatalog, epoch: u64) -> PowerStatusLiveBinding {
    PowerStatusLiveBinding {
        catalog: catalog.binding().clone(),
        game_instance_id: "game:1".to_owned(),
        run_id: "run:1".to_owned(),
        snapshot_id: format!("snapshot:{epoch}"),
        epoch,
    }
}

pub fn owner(kind: PowerStatusOwnerKind, id: &str) -> PowerStatusOwner {
    PowerStatusOwner {
        kind,
        id: owner_id(id),
        label: PowerStatusField::Available(id.to_owned()),
    }
}

pub fn source(kind: PowerStatusSourceKind, id: &str) -> PowerStatusSourceReference {
    PowerStatusSourceReference {
        kind,
        id: owner_id(id),
        label: PowerStatusField::Available(id.to_owned()),
    }
}

pub fn instance(
    id: &str,
    definition_id: &str,
    owner_kind: PowerStatusOwnerKind,
    amount: PowerStatusField<PowerStatusAmount>,
    duration: PowerStatusField<sts2_game_mod::PowerStatusDurationState>,
) -> PowerStatusInstanceInput {
    PowerStatusInstanceInput {
        instance_id: id.to_owned(),
        definition_id: definition_id.to_owned(),
        owner: owner(owner_kind, "entity:1"),
        source: PowerStatusField::Available(source(PowerStatusSourceKind::Card, "card:fixture")),
        amount,
        duration,
        application_order: PowerStatusField::Available(1),
        active: PowerStatusField::Available(true),
        suppressed: PowerStatusField::Available(false),
        pending_expiry: PowerStatusField::NotApplicable,
    }
}

pub fn integer(value: i64, unit_name: &str) -> PowerStatusAmount {
    PowerStatusAmount::Integer {
        value,
        unit: unit(unit_name),
    }
}

pub fn duration(value: i64, unit_name: &str) -> sts2_game_mod::PowerStatusDurationState {
    sts2_game_mod::PowerStatusDurationState::Remaining {
        value,
        unit: unit(unit_name),
    }
}

pub fn counter(id: &str, value: i64, unit_name: &str) -> PowerStatusCounterValue {
    PowerStatusCounterValue {
        id: id.to_owned(),
        value: PowerStatusField::Available(value),
        unit: unit(unit_name),
    }
}

pub fn expiry(value: i64, unit_name: &str) -> PowerStatusPendingExpiry {
    PowerStatusPendingExpiry::Remaining {
        value,
        unit: unit(unit_name),
    }
}
