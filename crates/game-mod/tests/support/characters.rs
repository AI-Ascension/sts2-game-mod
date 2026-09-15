// SPDX-License-Identifier: MIT

use sts2_game_mod::{
    CharacterCatalog, CharacterCatalogSnapshot, CharacterCatalogSource, CharacterContentReference,
    CharacterDefinitionInput, CharacterFamilyCoverage, CharacterFamilyState, CharacterField,
    CharacterFormula, CharacterLoadoutAvailability, CharacterLoadoutInput,
    CharacterLoadoutRequirement, CharacterMechanicReference, CharacterNumericValue,
    CharacterOrigin, CharacterPoolReference, CharacterResource, CharacterStartingConfiguration,
    CharacterText, CharacterUnlock, ContentCatalogSnapshot, ContentCatalogSource,
    ContentDefinitionInput, ContentManifest, ContentManifestProducer, ContentOriginInput,
    ContentPackageInput, ContentUnlockState,
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
pub struct CatalogSource {
    pub snapshot: Result<CharacterCatalogSnapshot, sts2_game_mod::CharacterSourceError>,
}

impl CharacterCatalogSource for CatalogSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<CharacterCatalogSnapshot, sts2_game_mod::CharacterSourceError> {
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

fn definition(
    entity_kind: &str,
    namespaced_id: &str,
    package_id: &str,
    package_version: Option<&str>,
) -> ContentDefinitionInput {
    ContentDefinitionInput {
        entity_kind: entity_kind.to_owned(),
        namespaced_id: namespaced_id.to_owned(),
        semantic_inputs: format!("kind={entity_kind};id={namespaced_id}"),
        localized_text: Some(namespaced_id.to_owned()),
        origin: ContentOriginInput {
            package_id: Some(package_id.to_owned()),
            package_version: package_version.map(str::to_owned),
        },
        override_chain: Vec::new(),
    }
}

pub fn manifest() -> ContentManifest {
    let snapshot = ContentCatalogSnapshot {
        generation_before: 11,
        generation_after: 11,
        game_build: "sts2-build:synthetic-character".to_owned(),
        locale: "en-US".to_owned(),
        packages: vec![
            package("base:game", Some("synthetic-1"), 0),
            package("mod:synthetic", None, 1),
        ],
        available_entity_kinds: vec![
            "card".to_owned(),
            "character".to_owned(),
            "relic".to_owned(),
        ],
        registry_definition_counts: [
            ("card".to_owned(), 2),
            ("character".to_owned(), 3),
            ("relic".to_owned(), 1),
        ]
        .into(),
        definitions: vec![
            definition(
                "card",
                "base:card:starter",
                "base:game",
                Some("synthetic-1"),
            ),
            definition(
                "card",
                "base:card:starter-plus",
                "base:game",
                Some("synthetic-1"),
            ),
            definition(
                "relic",
                "base:relic:starter",
                "base:game",
                Some("synthetic-1"),
            ),
            definition(
                "character",
                "base:character:ember",
                "base:game",
                Some("synthetic-1"),
            ),
            definition("character", "mod:character:locked", "mod:synthetic", None),
            definition("character", "mod:character:unknown", "mod:synthetic", None),
        ],
    };
    ContentManifestProducer::new("adapter-v1", ["character".to_owned()])
        .expect("manifest producer")
        .produce(&ManifestSource { snapshot })
        .expect("manifest")
}

fn origin(package_id: &str, package_version: Option<&str>) -> CharacterOrigin {
    CharacterOrigin {
        kind: "synthetic-registry".to_owned(),
        package_id: Some(package_id.to_owned()),
        package_version: package_version.map(str::to_owned),
    }
}

fn requirement(id: &str, description: CharacterText) -> CharacterLoadoutRequirement {
    CharacterLoadoutRequirement {
        progression_id: id.to_owned(),
        description,
    }
}

fn starting(
    hp: CharacterNumericValue,
    max_hp: CharacterNumericValue,
    gold: CharacterNumericValue,
) -> CharacterStartingConfiguration {
    CharacterStartingConfiguration {
        starting_hp: hp,
        max_hp,
        gold,
        potion_capacity: CharacterNumericValue::Fixed(3),
        resources: CharacterField::Available(vec![CharacterResource {
            resource_id: "resource:ember".to_owned(),
            unit: Some("count".to_owned()),
            amount: CharacterNumericValue::Fixed(2),
            capacity: Some(CharacterNumericValue::Fixed(5)),
        }]),
    }
}

fn loadout(
    id: &str,
    mode: Option<&str>,
    difficulty: Option<&str>,
    origin: CharacterOrigin,
    starting: CharacterStartingConfiguration,
) -> CharacterLoadoutInput {
    CharacterLoadoutInput {
        loadout_id: id.to_owned(),
        mode: mode.map(str::to_owned),
        difficulty: difficulty.map(str::to_owned),
        origin,
        availability: CharacterLoadoutAvailability::Available,
        starting,
        starting_deck: CharacterField::Available(vec![
            CharacterContentReference {
                entity_kind: "card".to_owned(),
                namespaced_id: "base:card:starter".to_owned(),
                quantity: 4,
            },
            CharacterContentReference {
                entity_kind: "card".to_owned(),
                namespaced_id: "base:card:starter-plus".to_owned(),
                quantity: 1,
            },
        ]),
        starting_relics: CharacterField::Available(vec![CharacterContentReference {
            entity_kind: "relic".to_owned(),
            namespaced_id: "base:relic:starter".to_owned(),
            quantity: 1,
        }]),
        pools: CharacterField::Available(vec![CharacterPoolReference {
            entity_kind: "card".to_owned(),
            pool_id: "pool:ember".to_owned(),
            label: CharacterText::Available("Ember cards".to_owned()),
        }]),
        mechanics: CharacterField::Available(vec![CharacterMechanicReference {
            mechanic_id: "mechanic:ember".to_owned(),
            label: CharacterText::Available("Ember resource".to_owned()),
            dependencies: vec!["resource:ember".to_owned()],
        }]),
        prerequisites: CharacterField::Available(Vec::new()),
    }
}

pub fn definitions() -> Vec<CharacterDefinitionInput> {
    let mut ember = CharacterDefinitionInput {
        character_id: "base:character:ember".to_owned(),
        name: CharacterText::Available("Émbér".to_owned()),
        description: CharacterText::Available("A synthetic character.".to_owned()),
        origin: origin("base:game", Some("synthetic-1")),
        loadouts: vec![
            loadout(
                "standard",
                Some("campaign"),
                Some("normal"),
                origin("base:game", Some("synthetic-1")),
                starting(
                    CharacterNumericValue::Fixed(70),
                    CharacterNumericValue::Fixed(75),
                    CharacterNumericValue::Fixed(99),
                ),
            ),
            loadout(
                "alternate",
                Some("practice"),
                Some("hard"),
                origin("base:game", Some("synthetic-1")),
                starting(
                    CharacterNumericValue::Formula(CharacterFormula {
                        rule_reference: "formula:practice_hp".to_owned(),
                        unresolved_inputs: vec!["difficulty".to_owned()],
                    }),
                    CharacterNumericValue::Fixed(80),
                    CharacterNumericValue::Fixed(0),
                ),
            ),
        ],
        unlock: CharacterField::Available(CharacterUnlock {
            state: ContentUnlockState::Unlocked,
            requirements: CharacterField::Available(Vec::new()),
        }),
    };
    ember.loadouts[1].prerequisites = CharacterField::Available(vec![requirement(
        "mode:practice",
        CharacterText::Available("Practice mode".to_owned()),
    )]);

    let locked = CharacterDefinitionInput {
        character_id: "mod:character:locked".to_owned(),
        name: CharacterText::Available("Locked".to_owned()),
        description: CharacterText::Available("Requires progression.".to_owned()),
        origin: origin("mod:synthetic", None),
        loadouts: vec![{
            let mut value = loadout(
                "standard",
                Some("campaign"),
                Some("normal"),
                origin("mod:synthetic", None),
                starting(
                    CharacterNumericValue::Unavailable(
                        sts2_game_mod::CharacterUnavailableReason::NotObserved,
                    ),
                    CharacterNumericValue::Unavailable(
                        sts2_game_mod::CharacterUnavailableReason::NotObserved,
                    ),
                    CharacterNumericValue::Fixed(50),
                ),
            );
            value.availability = CharacterLoadoutAvailability::Unavailable(
                sts2_game_mod::CharacterUnavailableReason::Unsupported,
            );
            value.starting.resources = CharacterField::Available(Vec::new());
            value.starting_deck = CharacterField::Available(Vec::new());
            value.starting_relics = CharacterField::Available(Vec::new());
            value.pools = CharacterField::Available(Vec::new());
            value.mechanics = CharacterField::Available(Vec::new());
            value.prerequisites = CharacterField::Available(vec![requirement(
                "progression:unlock-locked",
                CharacterText::Available("Complete the fixture milestone.".to_owned()),
            )]);
            value
        }],
        unlock: CharacterField::Available(CharacterUnlock {
            state: ContentUnlockState::Locked,
            requirements: CharacterField::Available(vec![requirement(
                "progression:unlock-locked",
                CharacterText::Available("Complete the fixture milestone.".to_owned()),
            )]),
        }),
    };

    let mut unknown_loadout = loadout(
        "unknown",
        None,
        None,
        origin("mod:synthetic", None),
        starting(
            CharacterNumericValue::Unavailable(sts2_game_mod::CharacterUnavailableReason::Unknown),
            CharacterNumericValue::Unavailable(sts2_game_mod::CharacterUnavailableReason::Unknown),
            CharacterNumericValue::Unavailable(sts2_game_mod::CharacterUnavailableReason::Unknown),
        ),
    );
    unknown_loadout.starting.resources =
        CharacterField::Unavailable(sts2_game_mod::CharacterUnavailableReason::Unknown);
    unknown_loadout.starting_deck =
        CharacterField::Unavailable(sts2_game_mod::CharacterUnavailableReason::NotObserved);
    unknown_loadout.starting_relics =
        CharacterField::Unavailable(sts2_game_mod::CharacterUnavailableReason::NotObserved);
    unknown_loadout.pools =
        CharacterField::Unavailable(sts2_game_mod::CharacterUnavailableReason::Unsupported);
    unknown_loadout.mechanics =
        CharacterField::Unavailable(sts2_game_mod::CharacterUnavailableReason::Unsupported);
    unknown_loadout.prerequisites =
        CharacterField::Unavailable(sts2_game_mod::CharacterUnavailableReason::NotObserved);
    let unknown = CharacterDefinitionInput {
        character_id: "mod:character:unknown".to_owned(),
        name: CharacterText::Unavailable(sts2_game_mod::CharacterUnavailableReason::Unknown),
        description: CharacterText::Unavailable(sts2_game_mod::CharacterUnavailableReason::Unknown),
        origin: origin("mod:synthetic", None),
        loadouts: vec![unknown_loadout],
        unlock: CharacterField::Unavailable(sts2_game_mod::CharacterUnavailableReason::Unknown),
    };
    vec![ember, locked, unknown]
}

pub fn snapshot(manifest: &ContentManifest) -> CharacterCatalogSnapshot {
    CharacterCatalogSnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        producer_version: sts2_game_mod::CHARACTER_PRODUCER_VERSION.to_owned(),
        family: CharacterFamilyCoverage {
            entity_kind: "character".to_owned(),
            state: CharacterFamilyState::Handled,
            definition_count: 3,
        },
        definitions: definitions(),
    }
}

pub fn catalog() -> CharacterCatalog {
    let manifest = manifest();
    sts2_game_mod::CharacterCatalogProducer::new()
        .produce(
            &manifest,
            &CatalogSource {
                snapshot: Ok(snapshot(&manifest)),
            },
        )
        .expect("character catalog")
}
