// SPDX-License-Identifier: MIT

use super::{
    definition::RunConfigurationDefinition,
    error::RunConfigurationError,
    field::RunConfigurationFieldStatus,
    model::{
        RunFieldKind, RunFieldRecord, RunMode, RunModifierState, RunSensitivity, RunVisibility,
        RunVisibilityScope,
    },
    page::RunConfigurationSummary,
    value::RunValue,
};

/// Rejects an ordinary scope that asked for seed material it must never receive.
pub(super) fn reject_seed_blind(
    kind: RunFieldKind,
    scope: RunVisibilityScope,
) -> Result<(), RunConfigurationError> {
    if kind == RunFieldKind::Seed && scope == RunVisibilityScope::SeedBlind {
        return Err(RunConfigurationError::SeedBlindScopeViolation);
    }
    Ok(())
}

/// Returns whether a reader in this scope may observe one field record.
pub(super) fn field_visible(record: &RunFieldRecord, scope: RunVisibilityScope) -> bool {
    if record.sensitivity == RunSensitivity::Private {
        return scope == RunVisibilityScope::Owner;
    }
    if record.kind == RunFieldKind::Seed {
        return scope != RunVisibilityScope::SeedBlind;
    }
    matches!(
        (record.visibility, scope),
        (RunVisibility::Public, _) | (RunVisibility::OwnerOnly, RunVisibilityScope::Owner)
    )
}

/// Returns whether a definition exposes at least one field to this scope.
pub(super) fn definition_visible(
    definition: &RunConfigurationDefinition,
    scope: RunVisibilityScope,
) -> bool {
    definition
        .fields
        .values()
        .any(|record| field_visible(record, scope))
}

/// Drops seed material from a projection the caller must not receive.
pub(super) fn project(
    definition: &RunConfigurationDefinition,
    scope: RunVisibilityScope,
) -> RunConfigurationDefinition {
    let mut projected = definition.clone();
    if scope == RunVisibilityScope::SeedBlind {
        projected.fields.remove(&RunFieldKind::Seed);
    }
    projected
}

/// Builds the bounded page summary for one definition.
pub(super) fn summarize(definition: &RunConfigurationDefinition) -> RunConfigurationSummary {
    RunConfigurationSummary {
        reference: definition.reference.clone(),
        live: definition.live.clone(),
        seed_policy: definition.seed_policy,
        mode: status_of(definition, RunFieldKind::Mode),
        difficulty: status_of(definition, RunFieldKind::Difficulty),
        character: status_of(definition, RunFieldKind::Character),
        active_modifiers: definition
            .modifiers
            .iter()
            .filter(|modifier| modifier.state == RunModifierState::Active)
            .count(),
        completeness: definition.completeness.clone(),
        cache: definition.cache.clone(),
        seed_blind_cache: definition.seed_blind_cache.clone(),
    }
}

pub(super) fn status_of(
    definition: &RunConfigurationDefinition,
    kind: RunFieldKind,
) -> RunConfigurationFieldStatus {
    definition
        .field(kind)
        .map_or(RunConfigurationFieldStatus::NotObserved, |record| {
            record.settled.status()
        })
}

/// Returns the settled mode of one definition when the host reported a known shape.
pub(super) fn mode_of(definition: &RunConfigurationDefinition) -> Option<RunMode> {
    match definition
        .field(RunFieldKind::Mode)
        .and_then(|record| record.settled.value())
    {
        Some(RunValue::Mode(mode)) => Some(*mode),
        _ => None,
    }
}
