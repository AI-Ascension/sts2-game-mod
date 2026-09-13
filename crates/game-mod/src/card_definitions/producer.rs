// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::{ContentCursorBinding, ContentDefinitionReference, ContentManifest};

use super::error::{CardDefinitionError, CardDefinitionInputError, CardDefinitionSourceError};
use super::validation::validate_input;
use super::variants::{
    CardDefinition, CardDefinitionCatalog, CardDefinitionInput, CardDefinitionProvenance,
    CardVariant,
};

/// One coherent source snapshot of every card definition in a content manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardDefinitionSnapshot {
    /// Manifest witness copied with the source values.
    pub manifest: ContentCursorBinding,
    /// Locale used by all localized variant text.
    pub locale: String,
    /// Typed records for every manifest card definition.
    pub definitions: Vec<CardDefinitionInput>,
}

/// Owner-local source boundary for typed card definitions.
///
/// Implementations must copy values from supported owner definitions. They must not construct
/// playable cards, upgrade live instances, consume randomness, mutate profiles, or expose host
/// objects and exceptions.
pub trait CardDefinitionSource {
    /// Copies one coherent card snapshot for the supplied content manifest.
    fn read_definitions(
        &self,
        manifest: &ContentManifest,
    ) -> Result<CardDefinitionSnapshot, CardDefinitionSourceError>;
}

/// Bounded producer for one immutable card-definition catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardDefinitionProducer;

impl CardDefinitionProducer {
    /// Creates the owner-local producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces a manifest-bound card catalog without mutating game state.
    pub fn produce<S: CardDefinitionSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<CardDefinitionCatalog, CardDefinitionError> {
        let snapshot = source
            .read_definitions(manifest)
            .map_err(CardDefinitionError::Source)?;
        let binding = manifest.cursor_binding();
        if snapshot.manifest != binding {
            return Err(CardDefinitionError::ManifestMismatch);
        }
        if snapshot.locale != manifest.locale {
            return Err(CardDefinitionError::LocaleMismatch);
        }
        if snapshot.definitions.len() > super::CARD_DEFINITION_MAX_DEFINITIONS {
            return Err(CardDefinitionError::InvalidInput(
                CardDefinitionInputError::CollectionTooLarge("definitions"),
            ));
        }

        let card_family = manifest
            .families
            .iter()
            .find(|family| family.entity_kind == "card")
            .ok_or(CardDefinitionError::NoCardFamily)?;
        if !card_family.handled {
            return Err(CardDefinitionError::UnsupportedCardFamily);
        }

        let manifest_cards = manifest
            .definitions
            .iter()
            .filter(|definition| definition.entity_kind == "card")
            .map(|definition| (definition.namespaced_id.as_str(), definition))
            .collect::<BTreeMap<_, _>>();
        let mut inputs = BTreeMap::new();
        for input in snapshot.definitions {
            validate_input(&input)?;
            if !manifest_cards.contains_key(input.namespaced_id.as_str()) {
                return Err(CardDefinitionError::UnknownDefinition {
                    namespaced_id: input.namespaced_id,
                });
            }
            if inputs.insert(input.namespaced_id.clone(), input).is_some() {
                return Err(CardDefinitionError::DuplicateDefinition);
            }
        }

        let mut definitions = BTreeMap::new();
        for (namespaced_id, manifest_definition) in manifest_cards {
            let Some(input) = inputs.remove(namespaced_id) else {
                return Err(CardDefinitionError::MissingDefinition {
                    namespaced_id: namespaced_id.to_owned(),
                });
            };
            let reference = ContentDefinitionReference {
                manifest: binding.clone(),
                entity_kind: "card".to_owned(),
                namespaced_id: namespaced_id.to_owned(),
            };
            let definition = CardDefinition {
                reference,
                provenance: CardDefinitionProvenance {
                    origin: manifest_definition.origin.clone(),
                    override_chain: manifest_definition.override_chain.clone(),
                    semantic_revision: manifest_definition.semantic_revision.clone(),
                    localized_text_revision: manifest_definition.localized_text_revision.clone(),
                },
                card_type: input.card_type,
                rarity: input.rarity,
                character_or_pool: input.character_or_pool,
                variants: input.variants.into_iter().map(CardVariant::from).collect(),
                upgrade_paths: input.upgrade_paths,
                acquisition: input.acquisition,
                unlock: input.unlock,
            };
            definitions.insert(namespaced_id.to_owned(), definition);
        }
        if !inputs.is_empty() {
            return Err(CardDefinitionError::UnknownDefinition {
                namespaced_id: inputs.keys().next().cloned().unwrap_or_default(),
            });
        }

        Ok(CardDefinitionCatalog::from_parts(
            binding,
            snapshot.locale,
            definitions,
        ))
    }
}

impl Default for CardDefinitionProducer {
    fn default() -> Self {
        Self::new()
    }
}

impl From<super::variants::CardVariantInput> for CardVariant {
    fn from(input: super::variants::CardVariantInput) -> Self {
        Self {
            variant_id: input.variant_id,
            kind: input.kind,
            upgrade_level: input.upgrade_level,
            title: input.title,
            description: input.description,
            cost: input.cost,
            targeting: input.targeting,
            keywords: input.keywords,
            effects: input.effects,
            structural_modifiers: input.structural_modifiers,
        }
    }
}
