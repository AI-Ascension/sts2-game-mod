// SPDX-License-Identifier: MIT

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use sts2_game_mod::{
        ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput,
        ContentManifestProducer, ContentOriginInput, ContentPackageInput, ContentSourceError,
    };

    #[derive(Clone)]
    struct Fixture(ContentCatalogSnapshot);

    impl ContentCatalogSource for Fixture {
        fn read_catalog(&self) -> Result<ContentCatalogSnapshot, ContentSourceError> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn semantic_inputs_change_revisions_without_id_or_type_changes() {
        let snapshot = ContentCatalogSnapshot {
            generation_before: 17,
            generation_after: 17,
            game_build: "v0.107.1".to_owned(),
            locale: "en_US".to_owned(),
            packages: vec![ContentPackageInput {
                package_id: "base:game".to_owned(),
                package_version: Some("0.107.1".to_owned()),
                order: 0,
            }],
            available_entity_kinds: vec!["badge".to_owned(), "card".to_owned(), "relic".to_owned()],
            registry_definition_counts: [
                ("badge".to_owned(), 1),
                ("card".to_owned(), 1),
                ("relic".to_owned(), 1),
            ]
            .into(),
            definitions: vec![
                definition(
                    "card",
                    "base:card:strike",
                    "{\"canonical_vars\":{\"damage\":{\"base_value\":6,\"string_value\":\"six\"}}}",
                ),
                definition("relic", "base:relic:ring", "{\"rarity\":\"Common\"}"),
                definition(
                    "badge",
                    "base:badge:starter",
                    "{\"should_receive_combat_hooks\":false}",
                ),
            ],
        };
        let producer =
            ContentManifestProducer::new("sts2-game-mod-modeldb-card-v1", vec!["card".to_owned()])
                .expect("producer");
        let first = producer
            .produce(&Fixture(snapshot.clone()))
            .expect("first manifest");
        let stable = producer
            .produce(&Fixture(snapshot.clone()))
            .expect("stable manifest");
        assert_eq!(first.inventory_revision, stable.inventory_revision);

        let mut changed_card = snapshot.clone();
        changed_card.definitions[0].semantic_inputs =
            "{\"canonical_vars\":{\"damage\":{\"base_value\":7,\"string_value\":\"seven\"}}}"
                .to_owned();
        let changed_card_manifest = producer
            .produce(&Fixture(changed_card))
            .expect("changed card manifest");
        assert_ne!(
            first.inventory_revision,
            changed_card_manifest.inventory_revision
        );

        let mut changed_relic = snapshot.clone();
        changed_relic.definitions[1].semantic_inputs = "{\"rarity\":\"Uncommon\"}".to_owned();
        let changed_relic_manifest = producer
            .produce(&Fixture(changed_relic))
            .expect("changed relic manifest");
        assert_ne!(
            first.inventory_revision,
            changed_relic_manifest.inventory_revision
        );

        let mut changed_badge = snapshot;
        changed_badge.definitions[2].semantic_inputs =
            "{\"should_receive_combat_hooks\":true}".to_owned();
        let changed_badge_manifest = producer
            .produce(&Fixture(changed_badge))
            .expect("changed badge manifest");
        assert_ne!(
            first.inventory_revision,
            changed_badge_manifest.inventory_revision
        );
    }

    fn definition(kind: &str, id: &str, semantic: &str) -> ContentDefinitionInput {
        ContentDefinitionInput {
            entity_kind: kind.to_owned(),
            namespaced_id: id.to_owned(),
            semantic_inputs: semantic.to_owned(),
            localized_text: None,
            origin: ContentOriginInput {
                package_id: None,
                package_version: None,
            },
            override_chain: Vec::new(),
        }
    }
}
