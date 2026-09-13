// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{
    CharacterOrigin, CharacterText, CharacterUnlock, ContentCursorBinding, ContentUnlockState,
};

use super::{
    CharacterCatalog, CharacterCatalogBinding, CharacterCatalogError, CharacterDefinition,
    CharacterDefinitionReference, CharacterField, CharacterListQuery, CharacterVisibilityScope,
};

fn binding() -> CharacterCatalogBinding {
    CharacterCatalogBinding {
        manifest: ContentCursorBinding {
            catalog_generation: 1,
            adapter_compatibility: "test-adapter".to_owned(),
            content_set_revision: "test-content".to_owned(),
            localized_text_revision: "test-locale".to_owned(),
            inventory_revision: "test-inventory".to_owned(),
        },
        locale: "en-US".to_owned(),
        producer_version: "test-producer".to_owned(),
    }
}

fn definition(binding: &CharacterCatalogBinding, character_id: &str) -> CharacterDefinition {
    CharacterDefinition {
        reference: CharacterDefinitionReference {
            catalog: binding.clone(),
            character_id: character_id.to_owned(),
        },
        name: CharacterText::Available(character_id.to_owned()),
        description: CharacterText::Available(character_id.to_owned()),
        origin: CharacterOrigin {
            kind: "test".to_owned(),
            package_id: None,
            package_version: None,
        },
        loadouts: Vec::new(),
        unlock: CharacterField::Available(CharacterUnlock {
            state: ContentUnlockState::Unlocked,
            requirements: CharacterField::Available(Vec::new()),
        }),
    }
}

fn catalog() -> CharacterCatalog {
    let binding = binding();
    let mut definitions = BTreeMap::new();
    definitions.insert(
        "test:character:first".to_owned(),
        definition(&binding, "test:character:first"),
    );
    definitions.insert(
        "test:character:second".to_owned(),
        definition(&binding, "test:character:second"),
    );
    CharacterCatalog::from_parts(
        binding,
        super::CharacterFamilyCoverage {
            entity_kind: "character".to_owned(),
            state: super::CharacterFamilyState::Handled,
            definition_count: 2,
        },
        definitions,
    )
}

fn first_page_query() -> CharacterListQuery {
    CharacterListQuery {
        locale: "en-US".to_owned(),
        scope: CharacterVisibilityScope::Public,
        limit: 1,
        continuation: None,
    }
}

#[test]
fn cursor_token_is_consumed_once_even_if_a_duplicate_token_is_constructed()
-> Result<(), CharacterCatalogError> {
    let mut reader = catalog().reader();
    let query = first_page_query();
    let first = reader.list(&query)?;
    let continuation = first
        .continuation
        .ok_or(CharacterCatalogError::InvalidContinuation)?;
    let duplicate = super::CharacterContinuation {
        token: continuation.token.clone(),
        scope: Arc::clone(&continuation.scope),
    };

    let mut next_query = query;
    next_query.continuation = Some(continuation);
    reader.list(&next_query)?;

    next_query.continuation = Some(duplicate);
    assert_eq!(
        reader.list(&next_query),
        Err(super::CharacterCatalogError::InvalidContinuation)
    );
    Ok(())
}

#[test]
fn borrowed_continuation_replay_across_catalog_clone_fails_closed()
-> Result<(), CharacterCatalogError> {
    let catalog = catalog();
    let cloned_catalog = catalog.clone();
    let mut owner_reader = catalog.reader();
    let mut cloned_reader = cloned_catalog.reader();
    let first = owner_reader.list(&first_page_query())?;
    let continuation = first
        .continuation
        .ok_or(CharacterCatalogError::InvalidContinuation)?;
    let replay_query = CharacterListQuery {
        locale: "en-US".to_owned(),
        scope: CharacterVisibilityScope::Public,
        limit: 1,
        continuation: Some(continuation),
    };

    assert_eq!(
        cloned_reader.list(&replay_query),
        Err(CharacterCatalogError::InvalidContinuation)
    );
    let second = owner_reader.list(&replay_query)?;
    assert!(second.complete);
    Ok(())
}
