// SPDX-License-Identifier: MIT

//! The manifest and value builders every completed-run fixture record shares.

use sts2_game_mod::{
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifest,
    ContentManifestProducer, ContentOriginInput, ContentPackageInput, ContentSourceError,
    RunResultDuration, RunResultDurationKind, RunResultFieldValue, RunResultQuantity,
    RunResultScore, RunResultScoreComponent, RunResultUnit, RunStatistic, RunStatisticKind,
    ScoreAuthority, ScoreMode,
};

/// Manifest families the shared fixture resolves its ending entries against.
pub const FIXTURE_ENTRIES: &[(&str, &str)] = &[
    ("card", "card.bash"),
    ("card", "card.defend"),
    ("card", "card.strike"),
    ("potion", "potion.fire"),
    ("relic", "relic.burning_blood"),
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestSource {
    pub snapshot: ContentCatalogSnapshot,
}

impl ContentCatalogSource for ManifestSource {
    fn read_catalog(&self) -> Result<ContentCatalogSnapshot, ContentSourceError> {
        Ok(self.snapshot.clone())
    }
}

fn content_definition(entity_kind: &str, id: &str) -> ContentDefinitionInput {
    ContentDefinitionInput {
        entity_kind: entity_kind.to_owned(),
        namespaced_id: id.to_owned(),
        semantic_inputs: format!("{entity_kind}={id}"),
        localized_text: Some(id.to_owned()),
        origin: ContentOriginInput {
            package_id: Some("base:synthetic".to_owned()),
            package_version: Some("1".to_owned()),
        },
        override_chain: Vec::new(),
    }
}

pub fn manifest_source(entries: &[(&str, &str)]) -> ManifestSource {
    let definitions = entries
        .iter()
        .map(|(entity_kind, id)| content_definition(entity_kind, id))
        .collect();
    let mut available_entity_kinds: Vec<String> =
        entries.iter().map(|(kind, _)| (*kind).to_owned()).collect();
    available_entity_kinds.sort();
    available_entity_kinds.dedup();
    ManifestSource {
        snapshot: ContentCatalogSnapshot {
            generation_before: 7,
            generation_after: 7,
            game_build: "sts2-build:synthetic".to_owned(),
            locale: "en-US".to_owned(),
            packages: vec![ContentPackageInput {
                package_id: "base:synthetic".to_owned(),
                package_version: Some("1".to_owned()),
                order: 0,
            }],
            registry_definition_counts: available_entity_kinds
                .iter()
                .map(|kind| {
                    (
                        kind.clone(),
                        entries
                            .iter()
                            .filter(|(entry_kind, _)| entry_kind == kind)
                            .count(),
                    )
                })
                .collect(),
            available_entity_kinds,
            definitions,
        },
    }
}

pub fn manifest(entries: &[(&str, &str)]) -> ContentManifest {
    let mut available_entity_kinds: Vec<String> =
        entries.iter().map(|(kind, _)| (*kind).to_owned()).collect();
    available_entity_kinds.sort();
    available_entity_kinds.dedup();
    ContentManifestProducer::new("adapter-v1", available_entity_kinds)
        .expect("producer")
        .produce(&manifest_source(entries))
        .expect("manifest")
}

/// The manifest every fixture record resolves its ending entries against.
pub fn fixture_manifest() -> ContentManifest {
    manifest(FIXTURE_ENTRIES)
}

pub fn unit(name: &str) -> RunResultUnit {
    RunResultUnit {
        unit: name.to_owned(),
    }
}

pub fn quantity(name: &str, amount: i64) -> RunResultQuantity {
    RunResultQuantity {
        amount,
        unit: unit(name),
    }
}

pub fn duration(seconds: u64) -> RunResultFieldValue<RunResultDuration> {
    RunResultFieldValue::present(RunResultDuration {
        seconds,
        kind: RunResultDurationKind::WallClock,
    })
}

pub fn component(
    id: &str,
    label: &str,
    amount: i64,
    authority: ScoreAuthority,
) -> RunResultScoreComponent {
    RunResultScoreComponent {
        component_id: id.to_owned(),
        label: RunResultFieldValue::present(label.to_owned()),
        contribution: quantity("points", amount),
        authority,
    }
}

/// A componentized host score whose components reconcile with the displayed total.
pub fn reconciled_score(components: Vec<RunResultScoreComponent>) -> RunResultScore {
    let total = components
        .iter()
        .map(|component| component.contribution.amount)
        .sum();
    RunResultScore {
        authority: ScoreAuthority::HostGameScore,
        mode: ScoreMode::Componentized,
        total: RunResultFieldValue::present(quantity("points", total)),
        components: RunResultFieldValue::present(components),
    }
}

/// A score mode that reports a displayed total without components.
pub fn total_only_score(total: i64) -> RunResultScore {
    RunResultScore {
        authority: ScoreAuthority::HostGameScore,
        mode: ScoreMode::TotalOnly,
        total: RunResultFieldValue::present(quantity("points", total)),
        components: RunResultFieldValue::absent(),
    }
}

/// A mode that reports no score at all, stated rather than collapsed to zero.
pub fn no_score() -> RunResultScore {
    RunResultScore {
        authority: ScoreAuthority::NoScore,
        mode: ScoreMode::Unsupported,
        total: RunResultFieldValue::absent(),
        components: RunResultFieldValue::absent(),
    }
}

pub fn statistics() -> RunResultFieldValue<Vec<RunStatistic>> {
    RunResultFieldValue::present(vec![RunStatistic {
        statistic_id: "stat.turns".to_owned(),
        kind: RunStatisticKind::CombatTurns,
        label: RunResultFieldValue::present("Turns".to_owned()),
        value: RunResultFieldValue::present(quantity("turns", 24)),
    }])
}
