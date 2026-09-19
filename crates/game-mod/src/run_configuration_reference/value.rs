// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::{
    error::RunConfigurationError,
    identity::{validate_identity, validate_text},
    model::{
        RUN_CONFIGURATION_MAX_IDS, RUN_CONFIGURATION_MAX_SEED_BYTES, RunDifficulty, RunFieldKind,
        RunMode, RunModifierInput, RunModifierState,
    },
};

/// Typed run-configuration field value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunValue {
    /// Admitted mode.
    Mode(RunMode),
    /// Advertised difficulty or ascension.
    Difficulty(RunDifficulty),
    /// One namespaced identity.
    Identifier(String),
    /// An ordered identity set.
    Identifiers(Vec<String>),
    /// A bounded count.
    Count(u32),
    /// A settled toggle.
    Toggle(bool),
    /// Bounded free text.
    Text(String),
    /// Visible run seed.
    Seed(String),
}

impl RunValue {
    /// Returns the stable wire token for the value shape.
    #[must_use]
    pub const fn shape(&self) -> &'static str {
        match self {
            Self::Mode(_) => "mode",
            Self::Difficulty(_) => "difficulty",
            Self::Identifier(_) => "identifier",
            Self::Identifiers(_) => "identifiers",
            Self::Count(_) => "count",
            Self::Toggle(_) => "toggle",
            Self::Text(_) => "text",
            Self::Seed(_) => "seed",
        }
    }
}

/// Returns the canonical token used by the configuration fingerprint.
#[must_use]
pub(super) fn canonical_token(value: &RunValue) -> String {
    match value {
        RunValue::Mode(mode) => format!("mode:{}", mode_token(*mode)),
        RunValue::Difficulty(difficulty) => format!("difficulty:{}", difficulty_token(difficulty)),
        RunValue::Identifier(identifier) => format!("id:{identifier}"),
        RunValue::Identifiers(identifiers) => {
            let mut sorted = identifiers.clone();
            sorted.sort();
            format!("ids:{}", sorted.join(","))
        }
        RunValue::Count(count) => format!("count:{count}"),
        RunValue::Toggle(settled) => format!("toggle:{settled}"),
        RunValue::Text(text) => format!("text:{text}"),
        RunValue::Seed(seed) => format!("seed:{seed}"),
    }
}

fn mode_token(mode: RunMode) -> &'static str {
    match mode {
        RunMode::Standard => "standard",
        RunMode::Custom => "custom",
        RunMode::Daily => "daily",
        RunMode::Cooperative => "cooperative",
        RunMode::Unknown => "unknown",
    }
}

fn difficulty_token(difficulty: &RunDifficulty) -> String {
    match difficulty {
        RunDifficulty::Base => "base".to_owned(),
        RunDifficulty::Ascension(level) => format!("ascension:{level}"),
        RunDifficulty::Custom(name) => format!("custom:{name}"),
        RunDifficulty::Unknown => "unknown".to_owned(),
    }
}

/// Returns whether the value shape is permitted for the field kind.
#[must_use]
pub(super) fn accepts(kind: RunFieldKind, value: &RunValue) -> bool {
    matches!(
        (kind, value),
        (RunFieldKind::Mode, RunValue::Mode(_))
            | (RunFieldKind::Difficulty, RunValue::Difficulty(_))
            | (RunFieldKind::Character, RunValue::Identifier(_))
            | (RunFieldKind::ActSequence, RunValue::Identifiers(_))
            | (RunFieldKind::Modifiers, RunValue::Identifiers(_))
            | (RunFieldKind::ActiveContent, RunValue::Identifiers(_))
            | (RunFieldKind::Seed, RunValue::Seed(_))
            | (
                RunFieldKind::Loadout | RunFieldKind::UnlockRule,
                RunValue::Identifier(_) | RunValue::Identifiers(_),
            )
            | (
                RunFieldKind::MultiplayerScaling,
                RunValue::Identifier(_) | RunValue::Count(_)
            )
            | (RunFieldKind::UnlockRule, RunValue::Toggle(_))
    )
}

/// Validates one value against its kind without substituting a default.
pub(super) fn validate_value(
    kind: RunFieldKind,
    value: &RunValue,
) -> Result<(), RunConfigurationError> {
    match value {
        RunValue::Mode(RunMode::Unknown) | RunValue::Difficulty(RunDifficulty::Unknown) => Ok(()),
        RunValue::Mode(_) | RunValue::Difficulty(_) | RunValue::Toggle(_) => Ok(()),
        RunValue::Identifier(identifier) => validate_identity(identifier, "identifier"),
        RunValue::Identifiers(identifiers) => validate_identifiers(identifiers),
        RunValue::Count(count) => {
            if *count == 0 {
                return Err(RunConfigurationError::InvalidInput("count"));
            }
            Ok(())
        }
        RunValue::Text(text) => validate_text(text, "text"),
        RunValue::Seed(seed) => validate_seed(seed),
    }
    .and_then(|()| {
        if accepts(kind, value) {
            Ok(())
        } else {
            Err(RunConfigurationError::FieldShapeMismatch {
                run_id: String::new(),
                kind,
            })
        }
    })
}

fn validate_identifiers(identifiers: &[String]) -> Result<(), RunConfigurationError> {
    if identifiers.is_empty() || identifiers.len() > RUN_CONFIGURATION_MAX_IDS {
        return Err(RunConfigurationError::InvalidInput("identifiers"));
    }
    let mut seen = BTreeSet::new();
    for identifier in identifiers {
        validate_identity(identifier, "identifier")?;
        if !seen.insert(identifier.as_str()) {
            return Err(RunConfigurationError::InvalidInput("duplicate_identifier"));
        }
    }
    Ok(())
}

fn validate_seed(seed: &str) -> Result<(), RunConfigurationError> {
    if seed.is_empty() || seed.len() > RUN_CONFIGURATION_MAX_SEED_BYTES {
        return Err(RunConfigurationError::InvalidInput("seed"));
    }
    if !seed
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "-_".contains(character))
    {
        return Err(RunConfigurationError::InvalidInput("seed"));
    }
    Ok(())
}

/// Returns the identities of every modifier the host reports as active.
#[must_use]
pub(super) fn active_modifier_ids(modifiers: &[RunModifierInput]) -> Vec<String> {
    let mut identifiers = modifiers
        .iter()
        .filter(|modifier| modifier.state == RunModifierState::Active)
        .map(|modifier| modifier.modifier_id.clone())
        .collect::<Vec<_>>();
    identifiers.sort();
    identifiers
}

/// Returns every field kind the active modifiers claim to change.
#[must_use]
pub(super) fn altered_kinds(modifiers: &[RunModifierInput]) -> Vec<RunFieldKind> {
    let mut kinds = modifiers
        .iter()
        .filter(|modifier| modifier.state == RunModifierState::Active)
        .flat_map(|modifier| modifier.alters.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    kinds.sort();
    kinds
}
