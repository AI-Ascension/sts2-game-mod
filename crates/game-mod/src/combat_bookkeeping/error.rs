// SPDX-License-Identifier: MIT

use super::model::{CombatZoneKind, CombatZoneStatus};

/// Named public counters with an owner-defined reset boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CombatCounterKind {
    /// Number of cards played in the current combat.
    CardsPlayed,
    /// Damage received by the player in the current combat.
    DamageTaken,
    /// Damage dealt by the player in the current combat.
    DamageDealt,
    /// Number of enemies defeated in the current combat.
    EnemiesDefeated,
}

impl CombatCounterKind {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::CardsPlayed => "cards_played",
            Self::DamageTaken => "damage_taken",
            Self::DamageDealt => "damage_dealt",
            Self::EnemiesDefeated => "enemies_defeated",
        }
    }

    /// Returns every supported counter in deterministic order.
    #[must_use]
    pub const fn all() -> &'static [Self; 4] {
        &[
            Self::CardsPlayed,
            Self::DamageTaken,
            Self::DamageDealt,
            Self::EnemiesDefeated,
        ]
    }
}

/// Why a source cannot currently provide a combat snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatUnavailableReason {
    /// No authorized source is attached.
    NoActiveSource,
    /// Exact-host source evidence is still required.
    ExactHostEvidenceRequired,
    /// The selected host/build has no supported extractor.
    UnsupportedBuild,
    /// The caller's scope cannot read the source.
    ScopeDenied,
}

impl CombatUnavailableReason {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NoActiveSource => "no_active_source",
            Self::ExactHostEvidenceRequired => "exact_host_evidence_required",
            Self::UnsupportedBuild => "unsupported_build",
            Self::ScopeDenied => "scope_denied",
        }
    }
}

/// Sanitized source failures before an owned snapshot exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatSourceError {
    /// No active source exists.
    NoActiveSource,
    /// The caller is not permitted to read the source.
    AccessDenied,
    /// A transient host resolution is in progress.
    Busy,
    /// The source changed while it was being copied.
    Stale,
    /// The source returned malformed data.
    Malformed,
}

impl std::fmt::Display for CombatSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CombatSourceError {}

/// Sanitized validation and read failures for the owner-local combat projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatBookkeepingError {
    /// A binding or nested identity was malformed.
    InvalidBinding(&'static str),
    /// A source-owned value violated a local bound or invariant.
    InvalidInput(&'static str),
    /// Two zones used the same identity.
    DuplicateZone(CombatZoneKind),
    /// One live card instance appeared in more than one zone.
    DuplicateCard(String),
    /// A source exposed a position that is not public for the zone.
    SecretOrderExposed,
    /// A public ordered zone omitted a position or repeated one.
    InvalidPublicOrder,
    /// A zone total disagreed with a complete composition.
    CountMismatch(CombatZoneKind),
    /// The aggregate combat card count disagreed with known zone totals.
    CombatTotalMismatch,
    /// The temporary-card count disagreed with its known temporary zone.
    TemporaryTotalMismatch,
    /// A zone was not available on this source surface.
    ZoneUnavailable {
        /// Exact requested zone.
        zone: CombatZoneKind,
        /// Explicit source status.
        status: CombatZoneStatus,
    },
    /// A requested field is not visible in the selected scope.
    VisibilityDenied(&'static str),
    /// The source capability is deliberately unavailable.
    Unavailable(CombatUnavailableReason),
    /// The source reported a transient resolving read.
    Busy,
    /// The source reported a stale or changing read.
    SourceStale,
    /// The requested reference belongs to an older snapshot.
    StaleReference,
    /// A replacement snapshot belongs to another game instance.
    GameInstanceMismatch,
    /// A replacement snapshot belongs to another run.
    RunMismatch,
    /// A replacement snapshot belongs to another combat.
    CombatMismatch,
    /// A replacement snapshot did not advance the epoch.
    NonMonotonicEpoch {
        /// Current epoch.
        current: u64,
        /// Supplied replacement epoch.
        supplied: u64,
    },
    /// A referenced card instance is absent from the current snapshot.
    CardNotFound,
    /// A requested named counter was not represented.
    CounterUnavailable(CombatCounterKind),
    /// A requested zone is not represented.
    ZoneNotFound(CombatZoneKind),
}

impl std::fmt::Display for CombatBookkeepingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBinding(field) => write!(formatter, "invalid combat binding: {field}"),
            Self::InvalidInput(field) => write!(formatter, "invalid combat input: {field}"),
            Self::DuplicateZone(zone) => write!(formatter, "duplicate combat zone: {zone:?}"),
            Self::DuplicateCard(card) => write!(formatter, "duplicate combat card: {card}"),
            Self::SecretOrderExposed => {
                formatter.write_str("draw-pile order is not public on this source")
            }
            Self::InvalidPublicOrder => formatter.write_str("public card order is invalid"),
            Self::CountMismatch(zone) => {
                write!(formatter, "zone total disagrees with composition: {zone:?}")
            }
            Self::CombatTotalMismatch => {
                formatter.write_str("combat card total disagrees with known zone totals")
            }
            Self::TemporaryTotalMismatch => {
                formatter.write_str("temporary card total disagrees with its zone")
            }
            Self::ZoneUnavailable { zone, status } => {
                write!(formatter, "combat zone {zone:?} is {}", status.code())
            }
            Self::VisibilityDenied(field) => {
                write!(formatter, "combat field is not visible: {field}")
            }
            Self::Unavailable(reason) => {
                write!(formatter, "combat source unavailable: {}", reason.code())
            }
            Self::Busy => formatter.write_str("combat read is busy"),
            Self::SourceStale => formatter.write_str("combat source read is stale"),
            Self::StaleReference => formatter.write_str("combat reference is stale"),
            Self::GameInstanceMismatch => formatter.write_str("combat game instance changed"),
            Self::RunMismatch => formatter.write_str("combat run changed"),
            Self::CombatMismatch => formatter.write_str("combat identity changed"),
            Self::NonMonotonicEpoch { current, supplied } => {
                write!(
                    formatter,
                    "combat epoch {supplied} is not newer than {current}"
                )
            }
            Self::CardNotFound => formatter.write_str("combat card instance not found"),
            Self::CounterUnavailable(counter) => {
                write!(formatter, "counter {} is unavailable", counter.code())
            }
            Self::ZoneNotFound(zone) => write!(formatter, "zone {zone:?} is not represented"),
        }
    }
}

impl std::error::Error for CombatBookkeepingError {}
