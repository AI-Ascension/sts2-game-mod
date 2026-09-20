// SPDX-License-Identifier: MIT

//! Typed result values whose availability is stated, so an unknown value is never a zero.

use super::{RunResultFieldStatus, ScoreAuthority, ScoreMode};

/// A value whose availability is explicit.
///
/// The status and the value are one field, not two, because a caller that can read the value
/// without reading its status will eventually read a missing value as a default. The constructors
/// are the only way to build one, so `Present` always carries a value and every other status
/// carries none.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResultFieldValue<T> {
    status: RunResultFieldStatus,
    value: Option<T>,
}

impl<T> RunResultFieldValue<T> {
    /// States that a value was reported.
    #[must_use]
    pub fn present(value: T) -> Self {
        Self {
            status: RunResultFieldStatus::Present,
            value: Some(value),
        }
    }

    /// States that the host reports the field as absent for this run.
    #[must_use]
    pub const fn absent() -> Self {
        Self {
            status: RunResultFieldStatus::Absent,
            value: None,
        }
    }

    /// States that the supported build does not carry the field.
    #[must_use]
    pub const fn unsupported() -> Self {
        Self {
            status: RunResultFieldStatus::Unsupported,
            value: None,
        }
    }

    /// States that the field is withheld at this scope.
    #[must_use]
    pub const fn withheld() -> Self {
        Self {
            status: RunResultFieldStatus::Withheld,
            value: None,
        }
    }

    /// Returns the availability status.
    #[must_use]
    pub const fn status(&self) -> RunResultFieldStatus {
        self.status
    }

    /// Returns whether a value was reported.
    #[must_use]
    pub const fn is_present(&self) -> bool {
        self.status.is_present()
    }

    /// Returns the reported value, or `None` when the status states no value.
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Consumes the field and returns the reported value, or `None`.
    #[must_use]
    pub fn into_value(self) -> Option<T> {
        self.value
    }

    /// Returns whether the status and the value agree.
    ///
    /// Every constructor agrees, so a disagreement means a caller built one by hand through a
    /// path this module does not offer.
    #[must_use]
    pub const fn is_consistent(&self) -> bool {
        self.status.is_present() == self.value.is_some()
    }
}

/// Unit of a reported quantity, such as a game's own score unit or a count.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RunResultUnit {
    /// Bounded unit identity, empty when the host reports a unitless count.
    pub unit: String,
}

/// A typed quantity with its exact unit preserved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResultQuantity {
    /// Signed amount as reported, never rescaled.
    pub amount: i64,
    /// Exact unit the host reported.
    pub unit: RunResultUnit,
}

/// Whether a duration is wall-clock or the run's own clock.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunResultDurationKind {
    /// Real elapsed time.
    WallClock,
    /// The run's own in-game clock.
    GameClock,
}

/// A run duration with its clock kind preserved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResultDuration {
    /// Elapsed seconds as reported.
    pub seconds: u64,
    /// Which clock produced the value.
    pub kind: RunResultDurationKind,
}

/// One score component contributing to a displayed total.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResultScoreComponent {
    /// Owner-defined component identity, such as `score.floor`.
    pub component_id: String,
    /// Localized or owner-defined label.
    pub label: RunResultFieldValue<String>,
    /// Signed contribution as displayed.
    pub contribution: RunResultQuantity,
    /// Authority that produced this component.
    pub authority: ScoreAuthority,
}

/// A displayed score total and the components that must reconcile to it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResultScore {
    /// Authority that produced the score.
    pub authority: ScoreAuthority,
    /// Whether the mode reports a componentized total.
    pub mode: ScoreMode,
    /// The displayed total.
    pub total: RunResultFieldValue<RunResultQuantity>,
    /// The components, stated absent when the mode reports none.
    pub components: RunResultFieldValue<Vec<RunResultScoreComponent>>,
}

/// Kind of a supported run statistic.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunStatisticKind {
    /// Combat turns taken.
    CombatTurns,
    /// Enemies defeated.
    EnemiesDefeated,
    /// Damage dealt.
    DamageDealt,
    /// Damage taken.
    DamageTaken,
    /// Cards played.
    CardsPlayed,
    /// Floors climbed.
    FloorsClimbed,
    /// An owner-defined statistic the reference does not name, identified by `statistic_id`.
    Other,
}

/// One reported run statistic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunStatistic {
    /// Owner-defined statistic identity.
    pub statistic_id: String,
    /// Supported kind, or an owner-defined one.
    pub kind: RunStatisticKind,
    /// Localized or owner-defined label.
    pub label: RunResultFieldValue<String>,
    /// Reported value with its unit.
    pub value: RunResultFieldValue<RunResultQuantity>,
}
