// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::{
    definition::{PotionDefinitionInput, PotionEffectKind, PotionEffectMagnitude},
    model::{
        POTION_MAX_ACQUISITION_RULES, POTION_MAX_ALTERNATIVES, POTION_MAX_EFFECTS,
        POTION_MAX_PARAMETERS, POTION_MAX_REFERENCES, PotionField, validate_identity,
        validate_text,
    },
};

/// Validates one static input record before it can enter a catalog.
pub(super) fn validate_definition(input: &PotionDefinitionInput) -> Result<(), &'static str> {
    validate_identity(&input.potion_id, "potion_id")?;
    validate_text(&input.title, "title")?;
    validate_text(&input.description, "description")?;
    validate_identity(input.rarity.as_str(), "rarity")?;
    if let Some(pool) = &input.pool {
        validate_identity(pool.as_str(), "pool")?;
    }
    validate_identity(&input.origin.kind, "origin_kind")?;
    if let Some(package_id) = &input.origin.package_id {
        validate_identity(package_id, "package_id")?;
    }
    if let Some(package_version) = &input.origin.package_version {
        validate_identity(package_version, "package_version")?;
    }
    if input.acquisition.rules.is_empty()
        || input.acquisition.rules.len() > POTION_MAX_ACQUISITION_RULES
    {
        return Err("acquisition_rules");
    }
    for rule in &input.acquisition.rules {
        validate_identity(&rule.kind, "acquisition_kind")?;
        for value in [&rule.reference, &rule.requirement].into_iter().flatten() {
            validate_text(value, "acquisition_value")?;
        }
    }
    if input.acquisition.unlock.requirements.len() > POTION_MAX_ACQUISITION_RULES {
        return Err("unlock_requirements");
    }
    for requirement in &input.acquisition.unlock.requirements {
        validate_identity(requirement, "unlock_requirement")?;
    }
    ensure_unique_ids(
        input
            .acquisition
            .unlock
            .requirements
            .iter()
            .map(String::as_str),
        "unlock_requirement",
    )?;

    if input.effects.is_empty() || input.effects.len() > POTION_MAX_EFFECTS {
        return Err("effects");
    }
    ensure_unique_ids(
        input.effects.iter().map(|effect| effect.id.as_str()),
        "effect_id",
    )?;
    let effect_ids = input
        .effects
        .iter()
        .map(|effect| effect.id.as_str())
        .collect::<BTreeSet<_>>();
    for effect in &input.effects {
        validate_identity(&effect.id, "effect_id")?;
        validate_text(&effect.label, "effect_label")?;
        validate_magnitude(&effect.magnitude)?;
        if let Some(condition) = &effect.condition {
            validate_identity(&condition.id, "effect_condition_id")?;
            validate_text(&condition.label, "effect_condition_label")?;
        }
        if effect.alternatives.len() > POTION_MAX_ALTERNATIVES {
            return Err("effect_alternatives");
        }
        ensure_unique_ids(
            effect
                .alternatives
                .iter()
                .map(|alternative| alternative.id.as_str()),
            "alternative_id",
        )?;
        for alternative in &effect.alternatives {
            validate_identity(&alternative.id, "alternative_id")?;
            validate_text(&alternative.label, "alternative_label")?;
            if alternative.effect_ids.is_empty()
                && alternative.rule_reference.is_none()
                && effect.kind != PotionEffectKind::Unknown
            {
                return Err("alternative_effect_ids");
            }
            for effect_id in &alternative.effect_ids {
                validate_identity(effect_id, "alternative_effect_id")?;
                if !effect_ids.contains(effect_id.as_str()) {
                    return Err("alternative_effect_id");
                }
            }
            if let Some(rule_reference) = &alternative.rule_reference {
                validate_text(rule_reference, "alternative_rule_reference")?;
            }
        }
        if effect.kind == PotionEffectKind::RandomChoice && effect.alternatives.is_empty() {
            return Err("random_choice_alternatives");
        }
        if effect.kind == PotionEffectKind::Multiple && effect.alternatives.is_empty() {
            return Err("multiple_alternatives");
        }
        if effect.kind == PotionEffectKind::Conditional && effect.condition.is_none() {
            return Err("conditional_condition");
        }
        if effect.references.len() > POTION_MAX_REFERENCES {
            return Err("effect_references");
        }
        validate_references(&effect.references)?;
    }
    if input.parameters.len() > POTION_MAX_PARAMETERS {
        return Err("parameters");
    }
    ensure_unique_ids(
        input
            .parameters
            .iter()
            .map(|parameter| parameter.id.as_str()),
        "parameter_id",
    )?;
    for parameter in &input.parameters {
        validate_identity(&parameter.id, "parameter_id")?;
        validate_text(&parameter.label, "parameter_label")?;
        validate_identity(parameter.unit.as_str(), "parameter_unit")?;
    }
    if input.references.len() > POTION_MAX_REFERENCES {
        return Err("references");
    }
    validate_references(&input.references)?;
    if let super::model::PotionUseRule::Conditional(condition) = &input.use_rule {
        validate_text(condition, "use_rule_condition")?;
    }
    Ok(())
}

fn validate_magnitude(magnitude: &PotionField<PotionEffectMagnitude>) -> Result<(), &'static str> {
    let Some(magnitude) = magnitude.value() else {
        return Ok(());
    };
    match magnitude {
        PotionEffectMagnitude::Fixed(_) => {}
        PotionEffectMagnitude::Range { min, max } => {
            if min > max {
                return Err("effect_range");
            }
        }
        PotionEffectMagnitude::Formula(formula) => {
            validate_text(formula, "effect_formula")?;
        }
        PotionEffectMagnitude::Unknown => {}
    }
    Ok(())
}

fn validate_references(
    references: &[super::definition::PotionSemanticReference],
) -> Result<(), &'static str> {
    ensure_unique_ids(
        references.iter().map(|reference| reference.id.as_str()),
        "reference_id",
    )?;
    for reference in references {
        validate_identity(&reference.id, "reference_id")?;
        validate_text(&reference.label, "reference_label")?;
    }
    Ok(())
}

fn ensure_unique_ids<'a>(
    values: impl IntoIterator<Item = &'a str>,
    field: &'static str,
) -> Result<(), &'static str> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .all(|value| seen.insert(value))
        .then_some(())
        .ok_or(field)
}

/// Measures bounded source-owned static payload bytes conservatively.
pub(super) fn definition_bytes(input: &PotionDefinitionInput) -> usize {
    let mut total = input.potion_id.len()
        + input.title.len()
        + input.description.len()
        + input.rarity.as_str().len()
        + input.pool.as_ref().map_or(0, |pool| pool.as_str().len())
        + input.origin.kind.len();
    total += input.origin.package_id.as_deref().map_or(0, str::len)
        + input.origin.package_version.as_deref().map_or(0, str::len);
    for rule in &input.acquisition.rules {
        total += rule.kind.len()
            + rule.reference.as_deref().map_or(0, str::len)
            + rule.requirement.as_deref().map_or(0, str::len);
    }
    total += input
        .acquisition
        .unlock
        .requirements
        .iter()
        .map(String::len)
        .sum::<usize>();
    for effect in &input.effects {
        total += effect.id.len()
            + effect.label.len()
            + magnitude_bytes(&effect.magnitude)
            + condition_bytes(effect.condition.as_ref());
        total += effect
            .alternatives
            .iter()
            .map(|alternative| {
                alternative.id.len()
                    + alternative.label.len()
                    + alternative
                        .effect_ids
                        .iter()
                        .map(String::len)
                        .sum::<usize>()
                    + alternative.rule_reference.as_deref().map_or(0, str::len)
            })
            .sum::<usize>();
        total += references_bytes(&effect.references);
    }
    total += input
        .parameters
        .iter()
        .map(|parameter| parameter.id.len() + parameter.label.len() + parameter.unit.as_str().len())
        .sum::<usize>();
    total += references_bytes(&input.references);
    total += match &input.use_rule {
        super::model::PotionUseRule::Conditional(value) => value.len(),
        _ => 1,
    };
    total
}

fn magnitude_bytes(magnitude: &PotionField<PotionEffectMagnitude>) -> usize {
    magnitude.value().map_or(1, |value| match value {
        PotionEffectMagnitude::Fixed(_) | PotionEffectMagnitude::Range { .. } => 8,
        PotionEffectMagnitude::Formula(value) => value.len(),
        PotionEffectMagnitude::Unknown => 1,
    })
}

fn condition_bytes(condition: Option<&super::definition::PotionCondition>) -> usize {
    condition.map_or(1, |condition| {
        1 + condition.id.len() + condition.label.len()
    })
}

fn references_bytes(references: &[super::definition::PotionSemanticReference]) -> usize {
    references
        .iter()
        .map(|reference| reference.id.len() + reference.label.len() + 1)
        .sum()
}
