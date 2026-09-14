// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::model::{validate_identity, validate_text};
use super::{
    ENEMY_INTENT_MAX_TARGETS, EnemyIntentError, EnemyIntentField, EnemyIntentTargetDomain,
    EnemyIntentTargetInfo, EnemyIntentTargetReference, EnemyIntentTargets, EnemyIntentUnit,
};

pub(super) fn validate_target_field(
    field: &EnemyIntentField<EnemyIntentTargetInfo>,
) -> Result<(), EnemyIntentError> {
    let Some(info) = field.value() else {
        return Ok(());
    };
    let EnemyIntentTargets::Visible(targets) = &info.targets else {
        if matches!(info.targets, EnemyIntentTargets::None)
            && !matches!(info.domain, EnemyIntentTargetDomain::None)
        {
            return Err(EnemyIntentError::InvalidInput("target domain"));
        }
        return Ok(());
    };
    if targets.len() > ENEMY_INTENT_MAX_TARGETS {
        return Err(EnemyIntentError::InvalidInput("targets"));
    }
    let mut ids = BTreeSet::new();
    for target in targets {
        validate_target(target)?;
        if !ids.insert(target.target_id.as_str()) {
            return Err(EnemyIntentError::DuplicateTarget(target.target_id.clone()));
        }
    }
    Ok(())
}

fn validate_target(target: &EnemyIntentTargetReference) -> Result<(), EnemyIntentError> {
    validate_identity(&target.target_id, "target_id").map_err(EnemyIntentError::InvalidInput)?;
    validate_text_field(&target.label, "target_label")?;
    Ok(())
}

pub(super) fn validate_identity_field(
    field: &EnemyIntentField<String>,
    name: &'static str,
) -> Result<(), EnemyIntentError> {
    if let Some(value) = field.value() {
        validate_identity(value, name).map_err(EnemyIntentError::InvalidInput)?;
    }
    Ok(())
}

pub(super) fn validate_text_field(
    field: &EnemyIntentField<String>,
    name: &'static str,
) -> Result<(), EnemyIntentError> {
    if let Some(value) = field.value() {
        validate_text(value, name).map_err(EnemyIntentError::InvalidInput)?;
    }
    Ok(())
}

pub(super) fn validate_unit_field(
    field: &EnemyIntentField<EnemyIntentUnit>,
    name: &'static str,
) -> Result<(), EnemyIntentError> {
    if let Some(value) = field.value() {
        validate_identity(value.as_str(), name).map_err(EnemyIntentError::InvalidInput)?;
    }
    Ok(())
}
