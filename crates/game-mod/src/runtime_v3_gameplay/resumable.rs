// SPDX-License-Identifier: MIT

//! Owner-local continuation offer gate for the runtime-v3 gameplay profile.
//!
//! `start_run` begins a new run; nothing else could re-enter a run that already exists on disk,
//! so every episode abandoned the previous one (`sts2-game-mod#172`). This module composes the
//! additive `continue_run` offer the host places beside `start_run` and the fence that stops it
//! from being offered for an absent, stale, or incompatible saved run.
//!
//! The mod owns only the offer decision. Detecting a saved run and reading its compatibility
//! identity stay with the native owner: the mod never opens a save, resolves a path, or infers a
//! run from a filename. `RuntimeV3GameplayResumableRun` is the bounded, redacted result the host
//! hands across that seam.

use super::contract::{
    RUNTIME_V3_GAMEPLAY_MAX_LEGAL_ACTIONS, RuntimeV3GameplayAction, RuntimeV3GameplayLegalAction,
    RuntimeV3GameplayState,
};

/// Identity prefix of a host-offered continuation.
const CONTINUE_RUN_KIND: &str = "continue_run";

/// Owner-local verdict on the saved run shown by the current screen.
///
/// The host produces one of these from its own read of the save slot and its own comparison
/// against the active content/version fence. No variant carries a path, payload, or account.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeV3GameplayResumableRun {
    /// No saved run is present on this screen.
    Absent,
    /// A saved run is present but does not match the active compatibility fence.
    Incompatible,
    /// A compatible saved run can be resumed.
    Compatible {
        /// Optional host-owned discriminator, absent when the screen already determines the run.
        run_id: Option<String>,
    },
}

/// Owner-reported compatibility of a saved run with the active content/version identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeV3GameplayRunCompatibility {
    /// The saved run was written by the active content/version identity.
    Matches,
    /// The saved run does not match the active content/version identity.
    Mismatches,
}

impl RuntimeV3GameplayResumableRun {
    /// Classifies one owner read of the saved-run slot.
    ///
    /// Only a present run that matches the active fence becomes `Compatible`; a mismatched run is
    /// reported as `Incompatible` rather than silently offered, so a stale save can never be
    /// resumed through this producer.
    #[must_use]
    pub fn detect(
        present: bool,
        compatibility: RuntimeV3GameplayRunCompatibility,
        run_id: Option<String>,
    ) -> Self {
        match (present, compatibility) {
            (false, _) => Self::Absent,
            (true, RuntimeV3GameplayRunCompatibility::Mismatches) => Self::Incompatible,
            (true, RuntimeV3GameplayRunCompatibility::Matches) => Self::Compatible { run_id },
        }
    }

    /// Returns the discriminator when this is a compatible resumable run.
    #[must_use]
    pub fn compatible_run_id(&self) -> Option<&Option<String>> {
        match self {
            Self::Compatible { run_id } => Some(run_id),
            Self::Absent | Self::Incompatible => None,
        }
    }
}

/// Why a continuation offer could not be composed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeV3GameplayResumeOfferError {
    /// The screen is not the setup screen that offers `start_run`.
    NotOnSetupScreen,
    /// The catalog does not offer exactly one `start_run`, so a continuation must not be added.
    StartRunNotOffered,
    /// The composed continuation identity is not a bounded, valid identity.
    InvalidContinuation,
    /// The composed continuation would duplicate an existing action identity.
    DuplicateActionId,
    /// The bounded catalog has no room for another action.
    CatalogFull,
}

impl std::fmt::Display for RuntimeV3GameplayResumeOfferError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::NotOnSetupScreen => "a continuation is only offered on the setup screen",
            Self::StartRunNotOffered => "a continuation requires the offered start_run",
            Self::InvalidContinuation => "the continuation identity is invalid",
            Self::DuplicateActionId => "the continuation identity is already offered",
            Self::CatalogFull => "the legal-action catalog is full",
        })
    }
}

impl std::error::Error for RuntimeV3GameplayResumeOfferError {}

/// Offers `continue_run` beside `start_run` for a compatible resumable run.
///
/// The host calls this while composing the setup-screen catalog and reports what its own save
/// read found. The continuation is appended only when the screen is `Setup`, the catalog already
/// offers exactly one `start_run`, and the owner reported `Compatible`; an `Absent` or
/// `Incompatible` run adds nothing and returns `Ok(false)`. The host supplies the sequence it
/// uses for the host-generated identity, which this producer preserves untouched.
pub fn offer_continue_run(
    state: &RuntimeV3GameplayState,
    offered: &mut Vec<RuntimeV3GameplayLegalAction>,
    resumable: &RuntimeV3GameplayResumableRun,
    sequence: u32,
) -> Result<bool, RuntimeV3GameplayResumeOfferError> {
    if !matches!(state, RuntimeV3GameplayState::Setup { .. }) {
        return Err(RuntimeV3GameplayResumeOfferError::NotOnSetupScreen);
    }
    let start_runs = offered
        .iter()
        .filter(|candidate| matches!(candidate.action, RuntimeV3GameplayAction::StartRun { .. }))
        .count();
    if start_runs != 1 {
        return Err(RuntimeV3GameplayResumeOfferError::StartRunNotOffered);
    }
    let Some(run_id) = resumable.compatible_run_id() else {
        return Ok(false);
    };
    let action_id = match run_id {
        Some(run_id) => format!("{CONTINUE_RUN_KIND}:{sequence}:{run_id}"),
        None => format!("{CONTINUE_RUN_KIND}:{sequence}"),
    };
    let action = RuntimeV3GameplayLegalAction {
        action_id,
        action: RuntimeV3GameplayAction::ContinueRun {
            run_id: run_id.clone(),
        },
    };
    action
        .validate()
        .map_err(|_| RuntimeV3GameplayResumeOfferError::InvalidContinuation)?;
    if offered
        .iter()
        .any(|candidate| candidate.action_id == action.action_id)
    {
        return Err(RuntimeV3GameplayResumeOfferError::DuplicateActionId);
    }
    if offered.len() >= RUNTIME_V3_GAMEPLAY_MAX_LEGAL_ACTIONS {
        return Err(RuntimeV3GameplayResumeOfferError::CatalogFull);
    }
    offered.push(action);
    Ok(true)
}
