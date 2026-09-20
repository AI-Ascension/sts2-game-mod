// SPDX-License-Identifier: MIT

//! Completion outcome, finalization, score-authority and origin vocabulary.

/// Reported completion outcome of a run.
///
/// A run that reached no terminal state is [`RunOutcome::Unfinished`] rather than a defeat, because
/// the two answer different questions.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunOutcome {
    /// The run completed with the host's victory result.
    Victory,
    /// The run ended in defeat, with or without a reason.
    Defeat,
    /// The run was abandoned or retired without victory or defeat.
    Abandonment,
    /// No terminal state was reported; this is not a result.
    Unfinished,
}

/// Whether a reported result is a persisted fact or an in-progress observation.
///
/// The distinction exists because the host starts its terminal presentation before it persists the
/// result, so an observation of that presentation is not evidence that a result was written.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResultFinalization {
    /// The host confirmed the result was persisted.
    Finalized,
    /// A terminal outcome was observed, but persistence is not confirmed.
    PendingPersistence,
    /// The reported outcome is partial or unsettled and may still change.
    Unsettled,
    /// The host reports no result for this run.
    Unavailable,
}

/// Authority that produced a score.
///
/// The game's own score, a harness evaluator score and a synthetic fixture metric are three
/// different claims, so an evaluator or synthetic score is never published as the game's score.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ScoreAuthority {
    /// The game's own authoritative score.
    HostGameScore,
    /// A harness evaluator's score, which is not the game's score.
    HarnessEvaluatorScore,
    /// A synthetic metric produced by a fixture or analysis, not by the host.
    SyntheticMetric,
    /// No score is reported for this run or mode.
    NoScore,
}

/// Whether the supported build can report a score for the completed result.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ScoreMode {
    /// The mode reports score components and a displayed total.
    Componentized,
    /// The mode reports a single displayed total without components.
    TotalOnly,
    /// The mode reports no score at all.
    Unsupported,
}

/// Origin of a retained result or summary record.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResultOrigin {
    /// Read from the host's own run history.
    Native,
    /// Copied from an imported artifact rather than this host.
    Imported,
    /// Produced by harness instrumentation rather than the host.
    Harness,
}

/// What the host reports about persisting the result.
///
/// A terminal presentation is not a persisted result, so finalization is only claimed when the
/// host confirms the write.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunResultPersistence {
    /// The host confirmed the result was written.
    Confirmed,
    /// The host reported that writing the result failed.
    Failed,
    /// The host reported nothing about persistence.
    Unknown,
}

/// Visibility class of a result or summary record.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunResultVisibility {
    /// May be observed at any scope.
    Public,
    /// May be observed only at the owning profile's scope.
    OwnerOnly,
    /// Withheld entirely.
    Hidden,
}

/// Scope a caller observes from.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunResultVisibilityScope {
    /// The caller owns the profile.
    Owner,
    /// The caller observes anonymously.
    Anonymous,
}

impl RunResultVisibilityScope {
    /// Returns whether this scope may observe the given visibility class.
    #[must_use]
    pub const fn observes(self, visibility: RunResultVisibility) -> bool {
        match visibility {
            RunResultVisibility::Public => true,
            RunResultVisibility::OwnerOnly => matches!(self, Self::Owner),
            RunResultVisibility::Hidden => false,
        }
    }
}
