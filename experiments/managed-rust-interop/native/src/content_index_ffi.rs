// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Mutex, OnceLock},
};
use sts2_game_mod::{
    ContentDetailCapabilities, ContentIndexDefinitionInput, ContentIndexError,
    ContentIndexProducer, ContentIndexReader, ContentIndexSnapshot, ContentIndexSource,
    ContentIndexSourceError, ContentKindAdapter, ContentKindAdapterRegistry, ContentListQuery,
    ContentManifest, ContentManifestProducer, ContentQueryFilters, ContentQueryLocale,
    ContentQueryScope, ContentRarity, ContentReferenceVisibilityPolicy, ContentSearchQuery,
    ContentUnlockState,
};

const STATUS_OK: i32 = 200;
const STATUS_UNAVAILABLE: i32 = 503;
const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_NATIVE_CURSORS: usize = 128;

#[derive(Clone, Debug, Deserialize)]
struct InputSnapshot {
    generation_before: u64,
    generation_after: u64,
    game_build: String,
    locale: String,
    packages: Vec<InputPackage>,
    available_entity_kinds: Vec<String>,
    registry_definition_counts: BTreeMap<String, usize>,
    definitions: Vec<InputDefinition>,
    adapter_compatibility: String,
}

#[derive(Clone, Debug, Deserialize)]
struct InputPackage {
    package_id: String,
    package_version: Option<String>,
    order: u32,
}

#[derive(Clone, Debug, Deserialize)]
struct InputDefinition {
    entity_kind: String,
    namespaced_id: String,
    semantic_inputs: String,
    localized_text: Option<String>,
    origin: InputOrigin,
    override_chain: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct InputOrigin {
    package_id: Option<String>,
    package_version: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct InputQuery {
    operation: String,
    literal: Option<String>,
    entity_kind: String,
    namespaced_id: Option<String>,
    #[serde(default)]
    namespaced_ids: Vec<String>,
    #[serde(default)]
    definition_refs: Vec<String>,
    #[serde(default)]
    manifest_id: String,
    #[serde(default = "default_scope")]
    scope: String,
    limit: usize,
    cursor: Option<String>,
    binding_key: String,
}

fn default_scope() -> String {
    "public".to_owned()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct Output {
    manifest_id: String,
    items: Vec<OutputItem>,
    total_count: usize,
    final_page: bool,
    next_cursor: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct OutputItem {
    entity_kind: String,
    namespaced_id: String,
    display_name: Option<String>,
    aliases: Vec<String>,
    rendered_description: Option<String>,
    character_or_pool: Option<String>,
    rarity: Option<String>,
    unlock_state: String,
    tags: Option<Vec<String>>,
}

struct Source {
    snapshot: ContentIndexSnapshot,
}

impl ContentIndexSource for Source {
    fn read_index(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<ContentIndexSnapshot, ContentIndexSourceError> {
        Ok(self.snapshot.clone())
    }
}

pub(super) struct CursorState {
    reader: ContentIndexReader,
    continuation: sts2_game_mod::ContentContinuation,
    manifest_id: String,
    binding_key: String,
    tags: BTreeMap<(String, String), Option<Vec<String>>>,
}

type BuiltIndex = (
    ContentManifest,
    sts2_game_mod::ContentIndex,
    BTreeMap<(String, String), Option<Vec<String>>>,
);

fn cursors() -> &'static Mutex<HashMap<String, CursorState>> {
    static CURSORS: OnceLock<Mutex<HashMap<String, CursorState>>> = OnceLock::new();
    CURSORS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn retain_cursor(token: String, state: CursorState) {
    if let Ok(mut values) = cursors().lock() {
        if values.len() >= MAX_NATIVE_CURSORS
            && let Some(oldest) = values.keys().next().cloned()
        {
            values.remove(&oldest);
        }
        values.insert(token, state);
    }
}

fn build_index(input: InputSnapshot) -> Result<BuiltIndex, String> {
    let tags = input
        .definitions
        .iter()
        .map(|definition| {
            Ok((
                (
                    definition.entity_kind.clone(),
                    definition.namespaced_id.clone(),
                ),
                input::optional_string_list(&definition.semantic_inputs, "tags")?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let parsed = input
        .definitions
        .iter()
        .map(to_index_input)
        .collect::<Result<Vec<_>, _>>()?;
    let card_definitions = parsed
        .iter()
        .filter(|value| value.entity_kind == "card")
        .collect::<Vec<_>>();
    let capabilities = ContentDetailCapabilities {
        full_definition: false,
        display_name: card_definitions
            .iter()
            .all(|value| value.display_name.is_some()),
        aliases: true,
        rendered_description: card_definitions
            .iter()
            .all(|value| value.rendered_description.is_some()),
        character_or_pool: card_definitions
            .iter()
            .all(|value| value.character_or_pool.is_some()),
        rarity: card_definitions.iter().all(|value| value.rarity.is_some()),
        unlock_state: card_definitions
            .iter()
            .all(|value| value.unlock_state != ContentUnlockState::Unknown),
    };
    let catalog = sts2_game_mod::ContentCatalogSnapshot {
        generation_before: input.generation_before,
        generation_after: input.generation_after,
        game_build: input.game_build,
        locale: input.locale.clone(),
        packages: input
            .packages
            .into_iter()
            .map(|value| sts2_game_mod::ContentPackageInput {
                package_id: value.package_id,
                package_version: value.package_version,
                order: value.order,
            })
            .collect(),
        available_entity_kinds: input.available_entity_kinds,
        registry_definition_counts: input.registry_definition_counts,
        definitions: input
            .definitions
            .into_iter()
            .map(|value| sts2_game_mod::ContentDefinitionInput {
                entity_kind: value.entity_kind,
                namespaced_id: value.namespaced_id,
                semantic_inputs: value.semantic_inputs,
                localized_text: value.localized_text,
                origin: sts2_game_mod::ContentOriginInput {
                    package_id: value.origin.package_id,
                    package_version: value.origin.package_version,
                },
                override_chain: value.override_chain,
            })
            .collect(),
    };
    let producer =
        ContentManifestProducer::new(input.adapter_compatibility, vec!["card".to_owned()])
            .map_err(|error| error.to_string())?;
    let manifest = producer
        .produce(&input::SourceManifest { snapshot: catalog })
        .map_err(|error| error.to_string())?;
    let snapshot = ContentIndexSnapshot {
        manifest: manifest.cursor_binding(),
        locale: input.locale,
        definitions: parsed
            .into_iter()
            .filter(|value| value.entity_kind == "card")
            .collect(),
    };
    let card = ContentKindAdapter::supported(
        "card",
        [
            sts2_game_mod::ContentFilterKind::CharacterOrPool,
            sts2_game_mod::ContentFilterKind::Rarity,
            sts2_game_mod::ContentFilterKind::UnlockState,
        ],
        capabilities,
    )
    .map_err(|error| error.to_string())?;
    let registry =
        ContentKindAdapterRegistry::new(&manifest, [card]).map_err(|error| error.to_string())?;
    let index = ContentIndexProducer::new(
        registry,
        ContentReferenceVisibilityPolicy::AllowLockedReferences,
    )
    .produce(&manifest, &Source { snapshot })
    .map_err(|error| error.to_string())?;
    Ok((manifest, index, tags))
}

fn to_index_input(definition: &InputDefinition) -> Result<ContentIndexDefinitionInput, String> {
    let semantic: Value =
        serde_json::from_str(&definition.semantic_inputs).map_err(|_| "semantic".to_owned())?;
    let localized = definition
        .localized_text
        .as_deref()
        .map(serde_json::from_str::<Value>)
        .transpose()
        .map_err(|_| "localized".to_owned())?;
    let strings = |value: Option<&Value>, name: &str| -> Result<Vec<String>, String> {
        let Some(field) = value.and_then(|value| value.get(name)) else {
            return Ok(Vec::new());
        };
        let Some(values) = field.as_array() else {
            return Err(format!("{name} is not an array"));
        };
        values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| format!("{name} contains a non-string"))
            })
            .collect()
    };
    let string = |value: Option<&Value>, name: &str| -> Result<Option<String>, String> {
        let Some(field) = value.and_then(|value| value.get(name)) else {
            return Ok(None);
        };
        field
            .as_str()
            .map(|value| Some(value.to_owned()))
            .ok_or_else(|| format!("{name} is not a string"))
    };
    let rarity = string(Some(&semantic), "rarity")?
        .map(ContentRarity::new)
        .transpose()
        .map_err(|error| error.to_string())?;
    Ok(ContentIndexDefinitionInput {
        entity_kind: definition.entity_kind.clone(),
        namespaced_id: definition.namespaced_id.clone(),
        display_name: string(localized.as_ref(), "title")?,
        aliases: strings(localized.as_ref(), "aliases")?,
        rendered_description: string(localized.as_ref(), "description")?,
        character_or_pool: string(Some(&semantic), "character_or_pool")?,
        rarity,
        unlock_state: match string(Some(&semantic), "unlock_state")?.as_deref() {
            Some("unlocked") => ContentUnlockState::Unlocked,
            Some("locked") => ContentUnlockState::Locked,
            _ => ContentUnlockState::Unknown,
        },
        term_references: strings(Some(&semantic), "term_references")?,
    })
}

#[path = "content_index_ffi_input.rs"]
mod input;

#[path = "content_index_ffi_projection.rs"]
mod projection;

#[path = "content_index_ffi_abi.rs"]
mod abi;

#[cfg(test)]
#[path = "content_index_ffi_tests.rs"]
mod tests;
