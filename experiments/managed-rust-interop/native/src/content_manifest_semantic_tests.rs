// SPDX-License-Identifier: MIT

#[cfg(test)]
mod tests {
    use std::error::Error;
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
    fn semantic_inputs_change_revisions_without_id_or_type_changes() -> Result<(), Box<dyn Error>> {
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
            available_entity_kinds: vec![
                "ancient".to_owned(),
                "badge".to_owned(),
                "card".to_owned(),
                "relic".to_owned(),
            ],
            registry_definition_counts: [
                ("ancient".to_owned(), 1),
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
                definition(
                    "ancient",
                    "base:ancient:shrine",
                    "{\"epithet\":\"A\",\"healed_amount\":5}",
                ),
            ],
        };
        let producer =
            ContentManifestProducer::new("sts2-game-mod-modeldb-card-v1", vec!["card".to_owned()])?;
        let first = producer.produce(&Fixture(snapshot.clone()))?;
        let stable = producer.produce(&Fixture(snapshot.clone()))?;
        assert_eq!(first.inventory_revision, stable.inventory_revision);

        let mut changed_card = snapshot.clone();
        changed_card.definitions[0].semantic_inputs =
            "{\"canonical_vars\":{\"damage\":{\"base_value\":7,\"string_value\":\"seven\"}}}"
                .to_owned();
        let changed_card_manifest = producer.produce(&Fixture(changed_card))?;
        assert_ne!(
            first.inventory_revision,
            changed_card_manifest.inventory_revision
        );

        let mut changed_relic = snapshot.clone();
        changed_relic.definitions[1].semantic_inputs = "{\"rarity\":\"Uncommon\"}".to_owned();
        let changed_relic_manifest = producer.produce(&Fixture(changed_relic))?;
        assert_ne!(
            first.inventory_revision,
            changed_relic_manifest.inventory_revision
        );

        let mut changed_badge = snapshot.clone();
        changed_badge.definitions[2].semantic_inputs =
            "{\"should_receive_combat_hooks\":true}".to_owned();
        let changed_badge_manifest = producer.produce(&Fixture(changed_badge))?;
        assert_ne!(
            first.inventory_revision,
            changed_badge_manifest.inventory_revision
        );
        let mut changed_ancient = snapshot;
        changed_ancient.definitions[3].semantic_inputs =
            "{\"epithet\":\"B\",\"healed_amount\":6}".to_owned();
        let ancient_manifest = producer.produce(&Fixture(changed_ancient))?;
        assert_ne!(
            first.inventory_revision,
            ancient_manifest.inventory_revision
        );
        Ok(())
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
