// SPDX-License-Identifier: MIT

use super::*;

pub(super) fn project(
    mut reader: ContentIndexReader,
    input: &InputQuery,
    manifest_id: &str,
    continuation: Option<sts2_game_mod::ContentContinuation>,
    tags: &BTreeMap<(String, String), Option<Vec<String>>>,
) -> Result<(Output, Option<CursorState>), String> {
    let locale =
        ContentQueryLocale::new(reader.index().locale()).map_err(|error| error.to_string())?;
    let scope = match input.scope.as_str() {
        "reference" => ContentQueryScope::Reference,
        "public" => ContentQueryScope::Public,
        _ => return Err("unsupported_scope".to_owned()),
    };
    // The core reader currently exposes entity-kind and literal search filters.
    // Reject additional identity filters at the native boundary instead of
    // applying them after a paged result (which could produce false empty
    // final pages when a match is beyond the first page).
    if !input.namespaced_ids.is_empty() || !input.definition_refs.is_empty() {
        return Err("unsupported_filter".to_owned());
    }
    match input.operation.as_str() {
        "list" | "availability" => {
            if input.literal.is_some() {
                return Err("unsupported_filter".to_owned());
            }
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
            let entries = page.entries;
            let output = Output {
                manifest_id: manifest_id.to_owned(),
                items: entries
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
            let entries = page.entries;
            let output = Output {
                manifest_id: manifest_id.to_owned(),
                items: entries
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
            if let Err(error) = reader.get(&reference, scope) {
                match error {
                    ContentIndexError::DetailUnavailable => {}
                    ContentIndexError::ExcludedByScope => return Err("denied_scope".to_owned()),
                    ContentIndexError::NotFound => return Err("not_found".to_owned()),
                    other => return Err(other.to_string()),
                }
            }
            // `get` above performs the authoritative scope check. Build the summary directly
            // from the immutable index so exact lookup does not depend on the first paged list
            // (valid definitions can occur after the reader's 64-item page bound).
            let value = reader
                .index()
                .definitions()
                .find(|definition| {
                    definition.reference.entity_kind == input.entity_kind
                        && Some(definition.reference.namespaced_id.as_str())
                            == input.namespaced_id.as_deref()
                })
                .map(|definition| sts2_game_mod::ContentDefinitionSummary {
                    reference: definition.reference.clone(),
                    origin: definition.origin.clone(),
                    semantic_revision: definition.semantic_revision.clone(),
                    localized_text_revision: definition.localized_text_revision.clone(),
                    display_name: definition.display_name.clone(),
                    character_or_pool: definition.character_or_pool.clone(),
                    rarity: definition.rarity.clone(),
                    unlock_state: definition.unlock_state,
                    detail_capabilities: definition.detail_capabilities.clone(),
                    term_references: definition.term_references.clone(),
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

pub(super) fn summary(value: sts2_game_mod::ContentDefinitionSummary) -> OutputItem {
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

pub(super) fn output_item(
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
