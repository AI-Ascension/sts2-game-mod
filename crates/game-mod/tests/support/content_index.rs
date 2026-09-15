// SPDX-License-Identifier: MIT

use sts2_game_mod::{
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentIndex,
    ContentIndexDefinitionInput, ContentIndexProducer, ContentIndexSnapshot, ContentIndexSource,
    ContentIndexSourceError, ContentKindAdapter, ContentKindAdapterRegistry, ContentManifest,
    ContentManifestProducer, ContentOriginInput, ContentPackageInput, ContentQueryLocale,
    ContentRarity, ContentReferenceVisibilityPolicy, ContentUnlockState,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestSource {
    pub snapshot: ContentCatalogSnapshot,
}

impl ContentCatalogSource for ManifestSource {
    fn read_catalog(&self) -> Result<ContentCatalogSnapshot, sts2_game_mod::ContentSourceError> {
        Ok(self.snapshot.clone())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexSource {
    pub snapshot: Result<ContentIndexSnapshot, ContentIndexSourceError>,
}

impl ContentIndexSource for IndexSource {
    fn read_index(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<ContentIndexSnapshot, ContentIndexSourceError> {
        self.snapshot.clone()
    }
}

fn package(package_id: &str, version: Option<&str>, order: u32) -> ContentPackageInput {
    ContentPackageInput {
        package_id: package_id.to_owned(),
        package_version: version.map(str::to_owned),
        order,
    }
}

fn manifest_definition(
    kind: &str,
    id: &str,
    package_id: Option<&str>,
    package_version: Option<&str>,
    text: &str,
) -> ContentDefinitionInput {
    ContentDefinitionInput {
        entity_kind: kind.to_owned(),
        namespaced_id: id.to_owned(),
        semantic_inputs: format!("kind={kind};id={id}"),
        localized_text: Some(text.to_owned()),
        origin: ContentOriginInput {
            package_id: package_id.map(str::to_owned),
            package_version: package_version.map(str::to_owned),
        },
        override_chain: Vec::new(),
    }
}

pub fn catalog_snapshot() -> ContentCatalogSnapshot {
    ContentCatalogSnapshot {
        generation_before: 7,
        generation_after: 7,
        game_build: "sts2-build:0.107.1".to_owned(),
        locale: "en-US".to_owned(),
        packages: vec![
            package("base:game", Some("0.107.1"), 0),
            package("mod:synthetic", None, 1),
        ],
        available_entity_kinds: vec!["card".to_owned(), "relic".to_owned()],
        registry_definition_counts: [("card".to_owned(), 3), ("relic".to_owned(), 1)].into(),
        definitions: vec![
            manifest_definition(
                "card",
                "base:ironclad:strike",
                Some("base:game"),
                Some("0.107.1"),
                "Strike",
            ),
            manifest_definition(
                "card",
                "mod:synthetic:strike",
                Some("mod:synthetic"),
                None,
                "Strike",
            ),
            manifest_definition(
                "card",
                "mod:synthetic:eclair",
                Some("mod:synthetic"),
                None,
                "Éclair",
            ),
            manifest_definition(
                "relic",
                "mod:synthetic:badge",
                Some("mod:synthetic"),
                None,
                "Badge",
            ),
        ],
    }
}

pub fn manifest() -> ContentManifest {
    let source = ManifestSource {
        snapshot: catalog_snapshot(),
    };
    ContentManifestProducer::new("adapter-v1", ["card".to_owned()])
        .expect("valid manifest producer")
        .produce(&source)
        .expect("valid manifest")
}

pub fn card_input(
    id: &str,
    name: Option<&str>,
    aliases: &[&str],
    description: Option<&str>,
    character_or_pool: Option<&str>,
    rarity: &str,
    unlock_state: ContentUnlockState,
) -> ContentIndexDefinitionInput {
    let mut input = ContentIndexDefinitionInput::new("card", id);
    input.display_name = name.map(str::to_owned);
    input.aliases = aliases.iter().map(|alias| (*alias).to_owned()).collect();
    input.rendered_description = description.map(str::to_owned);
    input.character_or_pool = character_or_pool.map(str::to_owned);
    input.rarity = Some(ContentRarity::new(rarity).expect("valid rarity"));
    input.unlock_state = unlock_state;
    input
}

pub fn source_snapshot(manifest: &ContentManifest) -> ContentIndexSnapshot {
    ContentIndexSnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        definitions: vec![
            card_input(
                "base:ironclad:strike",
                Some("Strike"),
                &["Hit"],
                Some("Deal six damage."),
                Some("ironclad"),
                "common",
                ContentUnlockState::Unlocked,
            ),
            card_input(
                "mod:synthetic:strike",
                Some("Strike"),
                &["Hit"],
                Some("Deal seven damage."),
                Some("ironclad"),
                "rare",
                ContentUnlockState::Locked,
            ),
            card_input(
                "mod:synthetic:eclair",
                Some("Éclair"),
                &["Sweet"],
                Some("A non-ASCII fixture."),
                Some("silent"),
                "uncommon",
                ContentUnlockState::Unlocked,
            ),
        ],
    }
}

pub fn registry(manifest: &ContentManifest) -> ContentKindAdapterRegistry {
    let card = ContentKindAdapter::supported(
        "card",
        [
            sts2_game_mod::ContentFilterKind::CharacterOrPool,
            sts2_game_mod::ContentFilterKind::Rarity,
            sts2_game_mod::ContentFilterKind::UnlockState,
        ],
        sts2_game_mod::ContentDetailCapabilities::full(),
    )
    .expect("valid card adapter");
    ContentKindAdapterRegistry::new(manifest, [card]).expect("registry includes relic unsupported")
}

pub fn index() -> ContentIndex {
    let manifest = manifest();
    let source = IndexSource {
        snapshot: Ok(source_snapshot(&manifest)),
    };
    ContentIndexProducer::new(
        registry(&manifest),
        ContentReferenceVisibilityPolicy::AllowLockedReferences,
    )
    .produce(&manifest, &source)
    .expect("valid content index")
}

pub fn locale() -> ContentQueryLocale {
    ContentQueryLocale::new("en-US").expect("valid locale")
}
