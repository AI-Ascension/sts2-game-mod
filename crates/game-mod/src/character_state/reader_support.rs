// SPDX-License-Identifier: MIT

use super::{
    catalog::CharacterStateCatalog,
    error::CharacterStateCatalogError,
    model::{CharacterMechanicState, CharacterStateVisibility, CharacterStateVisibilityScope},
    reader::{CharacterStateDefinitionKind, CharacterStateListQuery},
};

/// Rejects a static list query that touches unsupported or otherwise unclassified coverage.
pub(super) fn ensure_query_coverage(
    catalog: &CharacterStateCatalog,
    query: &CharacterStateListQuery,
) -> Result<(), CharacterStateCatalogError> {
    for coverage in catalog.coverage.values() {
        if matches_filter(
            &coverage.character_id,
            &coverage.mode_id,
            query.character_id.as_deref(),
            query.mode_id.as_deref(),
        ) {
            let state = match query.kind {
                CharacterStateDefinitionKind::Resource => coverage.resources,
                CharacterStateDefinitionKind::SecondaryEntity => coverage.secondary_entities,
            };
            ensure_supported(state)?;
        }
    }
    Ok(())
}

/// Maps one explicit source support state to a fail-closed catalog error.
pub(super) fn ensure_supported(
    state: CharacterMechanicState,
) -> Result<(), CharacterStateCatalogError> {
    match state {
        CharacterMechanicState::Supported => Ok(()),
        CharacterMechanicState::Unsupported => Err(CharacterStateCatalogError::UnsupportedMechanic),
        CharacterMechanicState::NotApplicable => {
            Err(CharacterStateCatalogError::NotApplicableMechanic)
        }
        CharacterMechanicState::Unavailable => Err(CharacterStateCatalogError::UnavailableMechanic),
        CharacterMechanicState::Unknown => Err(CharacterStateCatalogError::UnknownMechanic),
    }
}

pub(super) fn visible(
    visibility: CharacterStateVisibility,
    scope: CharacterStateVisibilityScope,
) -> bool {
    match visibility {
        CharacterStateVisibility::Visible => true,
        CharacterStateVisibility::OwnerOnly => {
            matches!(scope, CharacterStateVisibilityScope::Owner)
        }
        CharacterStateVisibility::Hidden | CharacterStateVisibility::Unknown => false,
    }
}

pub(super) fn matches_filter(
    character_id: &str,
    mode_id: &str,
    expected_character: Option<&str>,
    expected_mode: Option<&str>,
) -> bool {
    expected_character.is_none_or(|expected| expected == character_id)
        && expected_mode.is_none_or(|expected| expected == mode_id)
}
