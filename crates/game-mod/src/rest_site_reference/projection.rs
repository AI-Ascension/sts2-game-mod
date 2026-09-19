// SPDX-License-Identifier: MIT

use super::{
    definition::RestSiteDefinition,
    field::{RestFieldStatus, RestUnavailableReason},
    model::{RestVisibility, RestVisibilityScope},
    option::{RestCoverageRecord, RestOption},
    page::{RestOptionSummary, RestSiteDefinitionSummary},
};

/// Returns whether a visibility label may be observed under one scope.
pub(super) fn visibility_allowed(visibility: RestVisibility, scope: RestVisibilityScope) -> bool {
    match visibility {
        RestVisibility::Visible => true,
        RestVisibility::OwnerOnly => scope == RestVisibilityScope::Owner,
        RestVisibility::Hidden | RestVisibility::Unknown => false,
    }
}

/// Returns whether one rest option may be observed under one scope.
pub(super) fn visible_option(option: &RestOption, scope: RestVisibilityScope) -> bool {
    visibility_allowed(option.visibility, scope)
}

/// Returns whether one coverage record may be observed under one scope.
///
/// A record covering a typed option follows that option's visibility; a record covering an option
/// this producer did not type follows its owning definition's visibility, so an audit record never
/// discloses more than the site that carries it.
pub(super) fn visible_coverage(
    record: &RestCoverageRecord,
    definition: &RestSiteDefinition,
    scope: RestVisibilityScope,
) -> bool {
    match definition.options.get(&record.option_id) {
        Some(option) => visible_option(option, scope),
        None => visibility_allowed(definition.visibility, scope),
    }
}

/// Reports a collection as available, partially withheld, or fully withheld.
///
/// An observed empty collection is `Available`; a collection whose every record is withheld is
/// `Denied`; a mixed collection is `Withheld`. A withheld record is dropped rather than replaced
/// with a placeholder, so no restricted identity is disclosed.
fn scoped_status(total: usize, visible: usize, source: RestFieldStatus) -> RestFieldStatus {
    if source != RestFieldStatus::Available {
        return source;
    }
    if visible == total {
        RestFieldStatus::Available
    } else if visible == 0 {
        RestUnavailableReason::Denied.status()
    } else {
        RestUnavailableReason::Withheld.status()
    }
}

/// Reports the availability of one keyed collection after scope withholding.
pub(super) fn map_status<'a, T: 'a>(
    items: impl Iterator<Item = &'a T>,
    scope: RestVisibilityScope,
    visible: fn(&T, RestVisibilityScope) -> bool,
) -> RestFieldStatus {
    let mut total = 0;
    let mut shown = 0;
    for item in items {
        total += 1;
        if visible(item, scope) {
            shown += 1;
        }
    }
    scoped_status(total, shown, RestFieldStatus::Available)
}

/// Reports the availability of one coverage list after scope withholding.
fn coverage_status(definition: &RestSiteDefinition, scope: RestVisibilityScope) -> RestFieldStatus {
    let shown = definition
        .coverage
        .iter()
        .filter(|record| visible_coverage(record, definition, scope))
        .count();
    scoped_status(definition.coverage.len(), shown, RestFieldStatus::Available)
}

/// Projects a definition to the requested scope, dropping records the scope may not observe.
pub(super) fn project_definition(
    definition: &RestSiteDefinition,
    scope: RestVisibilityScope,
) -> RestSiteDefinition {
    RestSiteDefinition {
        reference: definition.reference.clone(),
        label: definition.label.clone(),
        description: definition.description.clone(),
        kind: definition.kind.clone(),
        visibility: definition.visibility,
        evidence: definition.evidence,
        option_set_generation: definition.option_set_generation,
        observed_option_count: definition.observed_option_count,
        options: definition
            .options
            .values()
            .filter(|option| visible_option(option, scope))
            .map(|option| (option.reference.option_id.clone(), option.clone()))
            .collect(),
        coverage: definition
            .coverage
            .iter()
            .filter(|record| visible_coverage(record, definition, scope))
            .cloned()
            .collect(),
        options_status: map_status(definition.options.values(), scope, visible_option),
        coverage_status: coverage_status(definition, scope),
        references: definition.references.clone(),
    }
}

/// Builds the bounded page summary for one rest-site definition under one scope.
pub(super) fn site_summary(
    definition: &RestSiteDefinition,
    scope: RestVisibilityScope,
) -> RestSiteDefinitionSummary {
    RestSiteDefinitionSummary {
        reference: definition.reference.clone(),
        label: definition.label.clone(),
        visibility: definition.visibility,
        evidence: definition.evidence,
        option_set_generation: definition.option_set_generation,
        option_count: definition
            .options
            .values()
            .filter(|option| visible_option(option, scope))
            .count(),
        options_status: map_status(definition.options.values(), scope, visible_option),
        coverage_count: definition
            .coverage
            .iter()
            .filter(|record| visible_coverage(record, definition, scope))
            .count(),
        coverage_status: coverage_status(definition, scope),
    }
}

/// Builds a bounded option summary that preserves availability and refusal state.
pub(super) fn option_summary(option: &RestOption) -> RestOptionSummary {
    RestOptionSummary {
        reference: option.reference.clone(),
        label: option.label.clone(),
        kind: option.kind.clone(),
        definition: option.definition.clone(),
        availability: option.availability.clone(),
        requirement_count: option.requirements.len(),
        effect_count: option.effects.len(),
        visibility: option.visibility,
    }
}
