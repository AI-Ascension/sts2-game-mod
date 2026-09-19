// SPDX-License-Identifier: MIT

mod effects;
pub(super) mod manifest;
mod references;

use std::collections::{BTreeMap, BTreeSet};

use self::effects::{validate_comparison, validate_effects, validate_selection};
use self::references::RefContext;
use super::{
    definition::RestSiteDefinitionInput,
    error::RestSiteError,
    identity::{validate_identity, validate_text, validate_text_value},
    model::{
        REST_MAX_COSTS, REST_MAX_COVERAGE_RECORDS, REST_MAX_LIMITS, REST_MAX_OPTIONS,
        REST_MAX_REQUIREMENTS, RestOptionKind, RestVisibility,
    },
    option::{RestCoverageRecord, RestOptionInput, RestOptionState},
};

/// Visibility target available inside one bounded snapshot.
#[derive(Clone, Copy, Debug)]
pub(super) struct RestTarget {
    /// Visibility of the target definition.
    pub(super) visibility: RestVisibility,
}

/// Validates one rest-site definition against every same-snapshot target.
pub(super) fn validate_definition(
    input: &RestSiteDefinitionInput,
    sites: &BTreeMap<String, RestTarget>,
) -> Result<(), RestSiteError> {
    validate_identity(&input.site_id, "site_id")?;
    validate_text_value(&input.label, "label")?;
    validate_text_value(&input.description, "description")?;
    if input.options.len() > REST_MAX_OPTIONS {
        return Err(RestSiteError::InvalidInput("options"));
    }
    if input.coverage.len() > REST_MAX_COVERAGE_RECORDS {
        return Err(RestSiteError::InvalidInput("coverage"));
    }
    if input.observed_options.len() > REST_MAX_OPTIONS + REST_MAX_COVERAGE_RECORDS {
        return Err(RestSiteError::InvalidInput("observed_options"));
    }
    for observed in &input.observed_options {
        validate_identity(observed, "observed_options")?;
    }
    let option_ids = collect_option_ids(input)?;
    let covered = collect_coverage_ids(&input.coverage)?;
    ensure_observed_options_accounted(input, &option_ids, &covered)?;
    let context = RefContext::new(&input.site_id, sites, &option_ids);
    context.check_all(&input.references, input.visibility)?;
    for option in &input.options {
        validate_option(option, input, &covered, &context)?;
    }
    Ok(())
}

fn collect_option_ids(input: &RestSiteDefinitionInput) -> Result<BTreeSet<String>, RestSiteError> {
    let mut ids = BTreeSet::new();
    for option in &input.options {
        validate_identity(&option.option_id, "option_id")?;
        if !ids.insert(option.option_id.clone()) {
            return Err(RestSiteError::DuplicateOption {
                site_id: input.site_id.clone(),
                option_id: option.option_id.clone(),
            });
        }
    }
    Ok(ids)
}

fn collect_coverage_ids(
    coverage: &[RestCoverageRecord],
) -> Result<BTreeSet<String>, RestSiteError> {
    let mut ids = BTreeSet::new();
    for record in coverage {
        validate_identity(&record.option_id, "coverage")?;
        validate_text_value(&record.reason, "coverage")?;
        if !ids.insert(record.option_id.clone()) {
            return Err(RestSiteError::InvalidInput("coverage"));
        }
    }
    Ok(ids)
}

/// Requires every host-reported option to be described or explicitly covered by name.
///
/// This is what stops a newly audited button from being silently omitted: an option the host offers
/// with neither a typed record nor a coverage record rejects the whole snapshot.
fn ensure_observed_options_accounted(
    input: &RestSiteDefinitionInput,
    option_ids: &BTreeSet<String>,
    covered: &BTreeSet<String>,
) -> Result<(), RestSiteError> {
    for observed in &input.observed_options {
        if !option_ids.contains(observed) && !covered.contains(observed) {
            return Err(RestSiteError::UncoveredOption {
                site_id: input.site_id.clone(),
                option_id: observed.clone(),
            });
        }
    }
    Ok(())
}

fn validate_option(
    option: &RestOptionInput,
    input: &RestSiteDefinitionInput,
    covered: &BTreeSet<String>,
    context: &RefContext<'_>,
) -> Result<(), RestSiteError> {
    let site_id = input.site_id.clone();
    validate_text_value(&option.label, "label")?;
    validate_text_value(&option.description, "description")?;
    validate_kind_agreement(option, &site_id)?;
    ensure_unsupported_kind_is_covered(option, &site_id, covered)?;
    validate_availability(option, &site_id)?;
    if option.action.is_available() {
        return Err(RestSiteError::InvalidInput("rest_action"));
    }
    if let Some(action_kind) = option.action_kind.value() {
        validate_text(action_kind, "action_kind")?;
    }
    context.check_all(&option.references, option.visibility)?;
    validate_requirements(option, context)?;
    validate_costs(option, context)?;
    validate_limits(option, context)?;
    validate_effects(option, &site_id, context)?;
    validate_selection(&option.selection, &site_id, &option.option_id, context)?;
    if let Some(comparison) = &option.comparison {
        validate_comparison(comparison, &site_id, &option.option_id, context)?;
    }
    Ok(())
}

/// Rejects an option that contradicts the definition family it resolves to.
///
/// A supported option may not report itself as unknown while its own reference resolves to a known
/// definition family, so no supported option can stay hard-coded unknown.
fn validate_kind_agreement(option: &RestOptionInput, site_id: &str) -> Result<(), RestSiteError> {
    if !matches!(option.kind, RestOptionKind::Unknown) {
        return Ok(());
    }
    if manifest::reference_kind_token(&option.definition.kind).is_none() {
        return Ok(());
    }
    Err(RestSiteError::KindDisagreesWithDefinition {
        site_id: site_id.to_owned(),
        option_id: option.option_id.clone(),
        reported: option.kind.clone(),
        family: manifest::reference_family(&option.definition.kind).unwrap_or_default(),
    })
}

/// Requires a named coverage record for an option this producer cannot type.
fn ensure_unsupported_kind_is_covered(
    option: &RestOptionInput,
    site_id: &str,
    covered: &BTreeSet<String>,
) -> Result<(), RestSiteError> {
    let untyped = matches!(
        option.kind,
        RestOptionKind::Unsupported(_) | RestOptionKind::Unknown
    );
    if untyped && !covered.contains(&option.option_id) {
        return Err(RestSiteError::UncoveredOption {
            site_id: site_id.to_owned(),
            option_id: option.option_id.clone(),
        });
    }
    Ok(())
}

/// Requires a refused option to explain itself and an offered option not to carry a refusal.
fn validate_availability(option: &RestOptionInput, site_id: &str) -> Result<(), RestSiteError> {
    let refused = matches!(option.availability.state, RestOptionState::Disabled);
    let refuses_silently = refused && !option.availability.reason.is_available();
    let offered_with_reason = !refused
        && matches!(
            option.availability.state,
            RestOptionState::Available | RestOptionState::NotOffered
        )
        && option.availability.reason.is_available();
    if refuses_silently || offered_with_reason {
        return Err(RestSiteError::InvalidAvailability {
            site_id: site_id.to_owned(),
            option_id: option.option_id.clone(),
        });
    }
    Ok(())
}

fn validate_requirements(
    option: &RestOptionInput,
    context: &RefContext<'_>,
) -> Result<(), RestSiteError> {
    if option.requirements.len() > REST_MAX_REQUIREMENTS {
        return Err(RestSiteError::InvalidInput("requirements"));
    }
    let mut seen = BTreeSet::new();
    for requirement in &option.requirements {
        validate_identity(&requirement.requirement_id, "requirement_id")?;
        if !seen.insert(requirement.requirement_id.clone()) {
            return Err(RestSiteError::DuplicateRequirement {
                site_id: context.site_id.to_owned(),
                option_id: option.option_id.clone(),
                requirement_id: requirement.requirement_id.clone(),
            });
        }
        validate_text_value(&requirement.label, "label")?;
        if let Some(detail) = requirement.detail.value() {
            validate_text(detail, "detail")?;
        }
        context.check_all(&requirement.references, option.visibility)?;
    }
    Ok(())
}

fn validate_costs(option: &RestOptionInput, context: &RefContext<'_>) -> Result<(), RestSiteError> {
    if option.costs.len() > REST_MAX_COSTS {
        return Err(RestSiteError::InvalidInput("costs"));
    }
    let mut seen = BTreeSet::new();
    for cost in &option.costs {
        validate_identity(&cost.cost_id, "cost_id")?;
        if !seen.insert(cost.cost_id.clone()) {
            return Err(RestSiteError::DuplicateCost {
                site_id: context.site_id.to_owned(),
                option_id: option.option_id.clone(),
                cost_id: cost.cost_id.clone(),
            });
        }
        validate_text_value(&cost.label, "label")?;
        if let Some(amount) = cost.amount.value()
            && *amount < 0
        {
            return Err(RestSiteError::InvalidInput("cost_amount"));
        }
        context.check_all(&cost.references, option.visibility)?;
    }
    Ok(())
}

fn validate_limits(
    option: &RestOptionInput,
    context: &RefContext<'_>,
) -> Result<(), RestSiteError> {
    if option.limits.len() > REST_MAX_LIMITS {
        return Err(RestSiteError::InvalidInput("limits"));
    }
    let mut seen = BTreeSet::new();
    for limit in &option.limits {
        validate_identity(&limit.limit_id, "limit_id")?;
        if !seen.insert(limit.limit_id.clone()) {
            return Err(RestSiteError::DuplicateCost {
                site_id: context.site_id.to_owned(),
                option_id: option.option_id.clone(),
                cost_id: limit.limit_id.clone(),
            });
        }
        validate_text_value(&limit.label, "label")?;
        context.check_all(&limit.references, option.visibility)?;
    }
    Ok(())
}

/// Builds the same-snapshot visibility targets for one rest-site identity.
pub(super) fn target_map(definitions: &[RestSiteDefinitionInput]) -> BTreeMap<String, RestTarget> {
    definitions
        .iter()
        .map(|definition| {
            (
                definition.site_id.clone(),
                RestTarget {
                    visibility: definition.visibility,
                },
            )
        })
        .collect()
}
