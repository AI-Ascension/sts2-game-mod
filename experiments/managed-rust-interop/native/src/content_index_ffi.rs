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
const STATUS_BAD_REQUEST: i32 = 400;
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
struct InputQuery {
    operation: String,
    literal: Option<String>,
    entity_kind: String,
    namespaced_id: Option<String>,
    limit: usize,
    cursor: Option<String>,
    binding_key: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Output {
    manifest_id: String,
    items: Vec<OutputItem>,
    total_count: usize,
    final_page: bool,
    next_cursor: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct OutputItem {
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

struct CursorState {
    reader: ContentIndexReader,
    continuation: sts2_game_mod::ContentContinuation,
    manifest_id: String,
    binding_key: String,
    tags: BTreeMap<(String, String), Option<Vec<String>>>,
}

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

fn build_index(
    input: InputSnapshot,
) -> Result<
    (
        ContentManifest,
        sts2_game_mod::ContentIndex,
        BTreeMap<(String, String), Option<Vec<String>>>,
    ),
    String,
> {
    let tags = input
        .definitions
        .iter()
        .map(|definition| {
            Ok((
                (
                    definition.entity_kind.clone(),
                    definition.namespaced_id.clone(),
                ),
                optional_string_list(&definition.semantic_inputs, "tags")?,
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
        .produce(&SourceManifest { snapshot: catalog })
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

struct SourceManifest {
    snapshot: sts2_game_mod::ContentCatalogSnapshot,
}

impl sts2_game_mod::ContentCatalogSource for SourceManifest {
    fn read_catalog(
        &self,
    ) -> Result<sts2_game_mod::ContentCatalogSnapshot, sts2_game_mod::ContentSourceError> {
        Ok(self.snapshot.clone())
    }
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

fn optional_string_list(json: &str, name: &str) -> Result<Option<Vec<String>>, String> {
    let value: Value = serde_json::from_str(json).map_err(|_| "semantic".to_owned())?;
    let Some(field) = value.get(name) else {
        return Ok(None);
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
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn project(
    mut reader: ContentIndexReader,
    input: &InputQuery,
    manifest_id: &str,
    continuation: Option<sts2_game_mod::ContentContinuation>,
    tags: &BTreeMap<(String, String), Option<Vec<String>>>,
) -> Result<(Output, Option<CursorState>), String> {
    let locale =
        ContentQueryLocale::new(reader.index().locale()).map_err(|error| error.to_string())?;
    let scope = ContentQueryScope::Public;
    match input.operation.as_str() {
        "list" | "availability" => {
            let page = reader
                .list(&ContentListQuery {
                    locale,
                    scope,
                    filters: ContentQueryFilters {
                        entity_kind: Some(input.entity_kind.clone()),
                        ..ContentQueryFilters::default()
                    },
                    limit: input.limit,
                    continuation,
                })
                .map_err(|error| error.to_string())?;
            let next = page.continuation.clone();
            let output = Output {
                manifest_id: manifest_id.to_owned(),
                items: page
                    .entries
                    .into_iter()
                    .map(|value| output_item(&reader, value, tags))
                    .collect(),
                total_count: page.total,
                final_page: next.is_none(),
                next_cursor: next.as_ref().map(|value| value.token().to_owned()),
            };
            let state = next.map(|continuation| CursorState {
                reader,
                continuation,
                manifest_id: manifest_id.to_owned(),
                binding_key: input.binding_key.clone(),
                tags: tags.clone(),
            });
            Ok((output, state))
        }
        "search" => {
            let page = reader
                .search(&ContentSearchQuery {
                    locale,
                    literal: input.literal.clone().unwrap_or_default(),
                    scope,
                    filters: ContentQueryFilters {
                        entity_kind: Some(input.entity_kind.clone()),
                        ..ContentQueryFilters::default()
                    },
                    limit: input.limit,
                    continuation,
                })
                .map_err(|error| error.to_string())?;
            let next = page.continuation.clone();
            let output = Output {
                manifest_id: manifest_id.to_owned(),
                items: page
                    .entries
                    .into_iter()
                    .map(|value| output_item(&reader, value.summary, tags))
                    .collect(),
                total_count: page.total,
                final_page: next.is_none(),
                next_cursor: next.as_ref().map(|value| value.token().to_owned()),
            };
            let state = next.map(|continuation| CursorState {
                reader,
                continuation,
                manifest_id: manifest_id.to_owned(),
                binding_key: input.binding_key.clone(),
                tags: tags.clone(),
            });
            Ok((output, state))
        }
        "get" | "detail" => {
            let reference = reader
                .index()
                .definitions()
                .find(|definition| {
                    definition.reference.entity_kind == input.entity_kind
                        && Some(definition.reference.namespaced_id.as_str())
                            == input.namespaced_id.as_deref()
                })
                .map(|definition| definition.reference.clone())
                .ok_or_else(|| "not_found".to_owned())?;
            if let Err(error) = reader.get(&reference, scope)
                && !matches!(error, ContentIndexError::DetailUnavailable)
            {
                return Err(error.to_string());
            }
            let page = reader
                .list(&ContentListQuery {
                    locale,
                    scope,
                    filters: ContentQueryFilters {
                        entity_kind: Some(input.entity_kind.clone()),
                        ..ContentQueryFilters::default()
                    },
                    limit: 64,
                    continuation: None,
                })
                .map_err(|error| error.to_string())?;
            let value = page
                .entries
                .into_iter()
                .find(|value| {
                    value.reference.namespaced_id == input.namespaced_id.clone().unwrap_or_default()
                })
                .ok_or_else(|| "not_found".to_owned())?;
            Ok((
                Output {
                    manifest_id: manifest_id.to_owned(),
                    items: vec![output_item(&reader, value, tags)],
                    total_count: 1,
                    final_page: true,
                    next_cursor: None,
                },
                None,
            ))
        }
        _ => Err("unsupported_operation".to_owned()),
    }
}

fn summary(value: sts2_game_mod::ContentDefinitionSummary) -> OutputItem {
    OutputItem {
        entity_kind: value.reference.entity_kind,
        namespaced_id: value.reference.namespaced_id,
        display_name: value.display_name,
        aliases: Vec::new(),
        rendered_description: None,
        character_or_pool: value.character_or_pool,
        rarity: value.rarity.map(|value| value.as_str().to_owned()),
        unlock_state: match value.unlock_state {
            ContentUnlockState::Unlocked => "unlocked",
            ContentUnlockState::Locked => "locked",
            ContentUnlockState::Unknown => "unknown",
        }
        .to_owned(),
        tags: None,
    }
}

fn output_item(
    reader: &ContentIndexReader,
    value: sts2_game_mod::ContentDefinitionSummary,
    tags: &BTreeMap<(String, String), Option<Vec<String>>>,
) -> OutputItem {
    let Some(definition) = reader.index().definitions().find(|definition| {
        definition.reference.entity_kind == value.reference.entity_kind
            && definition.reference.namespaced_id == value.reference.namespaced_id
    }) else {
        return summary(value);
    };
    OutputItem {
        entity_kind: definition.reference.entity_kind.clone(),
        namespaced_id: definition.reference.namespaced_id.clone(),
        display_name: definition.display_name.clone(),
        aliases: definition.aliases.clone(),
        rendered_description: definition.rendered_description.clone(),
        character_or_pool: definition.character_or_pool.clone(),
        rarity: definition
            .rarity
            .as_ref()
            .map(|value| value.as_str().to_owned()),
        unlock_state: match definition.unlock_state {
            ContentUnlockState::Unlocked => "unlocked",
            ContentUnlockState::Locked => "locked",
            ContentUnlockState::Unknown => "unknown",
        }
        .to_owned(),
        tags: tags
            .get(&(
                definition.reference.entity_kind.clone(),
                definition.reference.namespaced_id.clone(),
            ))
            .cloned()
            .flatten(),
    }
}

fn run(input: &[u8], query: &[u8]) -> (i32, Vec<u8>) {
    if input.len() > MAX_INPUT_BYTES || query.len() > MAX_INPUT_BYTES {
        return (
            STATUS_BAD_REQUEST,
            br#"{"error_code":"query_input_limit"}"#.to_vec(),
        );
    }
    let Ok(query) = serde_json::from_slice::<InputQuery>(query) else {
        return (
            STATUS_BAD_REQUEST,
            br#"{"error_code":"query_input_malformed"}"#.to_vec(),
        );
    };
    if query.limit == 0 || query.limit > 64 || query.entity_kind != "card" {
        return (
            STATUS_BAD_REQUEST,
            br#"{"error_code":"query_bounds"}"#.to_vec(),
        );
    }
    if let Some(cursor) = &query.cursor {
        let Some(state) = cursors()
            .lock()
            .ok()
            .and_then(|mut values| values.remove(cursor))
        else {
            return (409, br#"{"error_code":"stale_cursor"}"#.to_vec());
        };
        if state.manifest_id.is_empty() {
            return (409, br#"{"error_code":"stale_cursor"}"#.to_vec());
        }
        if state.binding_key != query.binding_key {
            return (409, br#"{"error_code":"stale_cursor"}"#.to_vec());
        }
        let CursorState {
            reader,
            continuation,
            manifest_id,
            tags,
            ..
        } = state;
        match project(reader, &query, &manifest_id, Some(continuation), &tags) {
            Ok((output, next)) => {
                if let Some(next) = next
                    && let Some(cursor) = &output.next_cursor
                {
                    retain_cursor(cursor.clone(), next);
                }
                return serde_json::to_vec(&output)
                    .map_or((STATUS_UNAVAILABLE, Vec::new()), |bytes| (STATUS_OK, bytes));
            }
            Err(_) => return (409, br#"{"error_code":"stale_cursor"}"#.to_vec()),
        }
    }
    let Ok(snapshot) = serde_json::from_slice::<InputSnapshot>(input) else {
        return (
            STATUS_BAD_REQUEST,
            br#"{"error_code":"query_snapshot_malformed"}"#.to_vec(),
        );
    };
    let Ok((manifest, index, tags)) = build_index(snapshot) else {
        return (
            STATUS_UNAVAILABLE,
            br#"{"error_code":"query_index_unavailable"}"#.to_vec(),
        );
    };
    let manifest_id = manifest.inventory_revision.clone();
    match project(index.reader(), &query, &manifest_id, None, &tags) {
        Ok((output, next)) => {
            if let Some(next) = next
                && let Some(cursor) = &output.next_cursor
            {
                retain_cursor(cursor.clone(), next);
            }
            serde_json::to_vec(&output)
                .map_or((STATUS_UNAVAILABLE, Vec::new()), |bytes| (STATUS_OK, bytes))
        }
        Err(error) if error == "not_found" => (404, br#"{"error_code":"unknown_id"}"#.to_vec()),
        Err(_) => (
            STATUS_UNAVAILABLE,
            br#"{"error_code":"query_failed"}"#.to_vec(),
        ),
    }
}

fn write_output(output: *mut u8, capacity: usize, length: *mut usize, bytes: &[u8]) -> i32 {
    if output.is_null()
        || length.is_null()
        || bytes.len() > capacity
        || bytes.len() > MAX_OUTPUT_BYTES
    {
        return STATUS_UNAVAILABLE;
    }
    // SAFETY: the caller supplies writable storage for the declared capacity and a valid length.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len());
        *length = bytes.len();
    }
    STATUS_OK
}

/// Runs one bounded static content-index query through the Rust ContentIndexReader.
///
/// The input snapshot and query are copied before this call and the output is written only to
/// caller-owned bounded storage. Cursor state retains one native reader and continuation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sts2_game_mod_content_index_query(
    input: *const u8,
    input_length: usize,
    query: *const u8,
    query_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> i32 {
    let result = std::panic::catch_unwind(|| {
        if query.is_null() || output_length.is_null() {
            return (STATUS_BAD_REQUEST, Vec::new());
        }
        // SAFETY: caller guarantees readable buffers for the declared lengths.
        let query_bytes = unsafe { std::slice::from_raw_parts(query, query_length) };
        let input_bytes = if input.is_null() {
            &[][..]
        } else {
            // SAFETY: caller guarantees readable snapshot buffer for the declared length.
            unsafe { std::slice::from_raw_parts(input, input_length) }
        };
        run(input_bytes, query_bytes)
    });
    match result {
        Ok((status, bytes)) => {
            if bytes.is_empty() {
                return status;
            }
            let write_status = write_output(output, output_capacity, output_length, &bytes);
            if write_status == STATUS_OK {
                status
            } else {
                write_status
            }
        }
        Err(_) => STATUS_UNAVAILABLE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn must<T, E: std::fmt::Debug>(value: Result<T, E>, label: &str) -> T {
        match value {
            Ok(value) => value,
            Err(error) => {
                eprintln!("{label}: {error:?}");
                std::process::abort();
            }
        }
    }

    fn must_some<T>(value: Option<T>, label: &str) -> T {
        match value {
            Some(value) => value,
            None => {
                eprintln!("{label}");
                std::process::abort();
            }
        }
    }

    fn snapshot() -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "generation_before": 7,
            "generation_after": 7,
            "game_build": "build:probe",
            "locale": "en-US",
            "packages": [
                {"package_id": "base", "package_version": "1", "order": 0}
            ],
            "available_entity_kinds": ["card", "relic"],
            "registry_definition_counts": {"card": 4, "relic": 0},
            "definitions": [
                {
                    "entity_kind": "card", "namespaced_id": "ironclad:bash",
                    "semantic_inputs": "{\"rarity\":\"attack\",\"unlock_state\":\"unlocked\"}",
                    "localized_text": "{\"title\":\"Bash\",\"description\":\"Deal damage\"}",
                    "origin": {"package_id":"base","package_version":"1"},
                    "override_chain": []
                },
                {
                    "entity_kind": "card", "namespaced_id": "ironclad:defend",
                    "semantic_inputs": "{\"unlock_state\":\"unlocked\"}",
                    "localized_text": "{\"title\":\"Defend\",\"aliases\":[\"Guard\"]}",
                    "origin": {"package_id":"base","package_version":"1"},
                    "override_chain": []
                },
                {
                    "entity_kind": "card", "namespaced_id": "ironclad:strike",
                    "semantic_inputs": "{\"rarity\":\"attack\",\"unlock_state\":\"unlocked\"}",
                    "localized_text": "{\"title\":\"Strike\",\"aliases\":[\"Hit\"],\"description\":\"Deal damage\"}",
                    "origin": {"package_id":"base","package_version":"1"},
                    "override_chain": []
                },
                {
                    "entity_kind": "card", "namespaced_id": "ironclad:hidden",
                    "semantic_inputs": "{\"unlock_state\":\"unknown\"}",
                    "localized_text": "{\"title\":\"Hidden\"}",
                    "origin": {"package_id":"base","package_version":"1"},
                    "override_chain": []
                }
            ],
            "adapter_compatibility": "content-index-v1"
        }))
        .unwrap_or_else(|error| {
            eprintln!("fixture serializes: {error}");
            std::process::abort();
        })
    }

    fn query(operation: &str, limit: usize, cursor: Option<&str>) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "operation": operation,
            "literal": null,
            "entity_kind": "card",
            "namespaced_id": null,
            "limit": limit,
            "cursor": cursor,
            "binding_key": "binding:fixture"
        }))
        .unwrap_or_else(|error| {
            eprintln!("query serializes: {error}");
            std::process::abort();
        })
    }

    #[test]
    fn reader_controls_visibility_order_and_single_use_cursor() {
        let fixture: InputSnapshot = must(serde_json::from_slice(&snapshot()), "snapshot");
        assert!(build_index(fixture).is_ok(), "index build failed");
        let (status, first_bytes) = run(&snapshot(), &query("list", 1, None));
        assert_eq!(status, STATUS_OK);
        let first: Output = must(serde_json::from_slice(&first_bytes), "first output");
        assert_eq!(first.total_count, 3);
        assert!(!first.final_page);
        assert_eq!(first.items[0].namespaced_id, "ironclad:bash");
        let cursor = must_some(first.next_cursor, "continuation");

        let (status, second_bytes) = run(&[], &query("list", 1, Some(&cursor)));
        assert_eq!(status, STATUS_OK);
        let second: Output = must(serde_json::from_slice(&second_bytes), "second output");
        assert!(!second.final_page);
        assert_eq!(second.items[0].namespaced_id, "ironclad:defend");
        let third_cursor = must_some(second.next_cursor, "third continuation");

        let (status, third_bytes) = run(&[], &query("list", 1, Some(&third_cursor)));
        assert_eq!(status, STATUS_OK);
        let third: Output = must(serde_json::from_slice(&third_bytes), "third output");
        assert!(third.final_page);
        assert_eq!(third.items[0].namespaced_id, "ironclad:strike");

        let (status, _) = run(&[], &query("list", 1, Some(&cursor)));
        assert_eq!(status, 409);
    }

    #[test]
    fn reader_uses_literal_folded_search_and_unknown_is_excluded() {
        let (status, bytes) = run(
            &snapshot(),
            &serde_json::to_vec(&serde_json::json!({
                "operation": "search",
                "literal": "guard",
                "entity_kind": "card",
                "namespaced_id": null,
                "limit": 8,
                "cursor": null,
                "binding_key": "binding:search"
            }))
            .unwrap_or_else(|error| {
                eprintln!("query serializes: {error}");
                std::process::abort();
            }),
        );
        assert_eq!(status, STATUS_OK);
        let output: Output = must(serde_json::from_slice(&bytes), "search output");
        assert_eq!(output.total_count, 1);
        assert_eq!(output.items[0].namespaced_id, "ironclad:defend");
    }

    #[test]
    fn exact_unknown_returns_not_found_status() {
        let unknown = serde_json::to_vec(&serde_json::json!({
            "operation": "get", "literal": null, "entity_kind": "card",
            "namespaced_id": "ironclad:missing", "limit": 1, "cursor": null,
            "binding_key": "binding:unknown"
        }))
        .unwrap_or_else(|error| {
            eprintln!("query serializes: {error}");
            std::process::abort();
        });
        let (status, _) = run(&snapshot(), &unknown);
        assert_eq!(status, 404);
    }
}
