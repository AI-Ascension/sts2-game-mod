// SPDX-License-Identifier: MIT

use super::identity::LiveCardIdentityError;

/// Fields that can be requested from a live card projection.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LiveCardField {
    /// Common static definition link.
    Definition,
    /// Owning player, creature, or selector.
    Owner,
    /// Current pile/selector and position witness.
    Location,
    /// Upgrade count and variant/path.
    Upgrade,
    /// Ordered permanent and temporary modifiers.
    Modifiers,
    /// Retain/exhaust/ethereal and other supported flags.
    Flags,
    /// Effect parameter overrides.
    EffectParameters,
    /// Base/current/effective cost semantics.
    Cost,
}

impl LiveCardField {
    /// Returns a stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Definition => "definition",
            Self::Owner => "owner",
            Self::Location => "location",
            Self::Upgrade => "upgrade",
            Self::Modifiers => "modifiers",
            Self::Flags => "flags",
            Self::EffectParameters => "effect_parameters",
            Self::Cost => "cost",
        }
    }

    /// Returns every field in deterministic order.
    #[must_use]
    pub const fn all() -> &'static [Self; 8] {
        &[
            Self::Definition,
            Self::Owner,
            Self::Location,
            Self::Upgrade,
            Self::Modifiers,
            Self::Flags,
            Self::EffectParameters,
            Self::Cost,
        ]
    }
}

/// Local classification for a card field that cannot be returned as a value.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LiveCardFieldStatus {
    /// A value was copied coherently.
    Available,
    /// The field is supported but outside the current source surface.
    NotObserved,
    /// The current owner/kind has no extractor.
    Unsupported,
    /// Caller scope does not permit this field.
    Denied,
    /// The reference no longer names the current snapshot.
    Stale,
    /// Bounded extraction failed.
    Failed,
}

impl LiveCardFieldStatus {
    /// Returns a stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::NotObserved => "not_observed",
            Self::Unsupported => "unsupported",
            Self::Denied => "denied",
            Self::Stale => "stale",
            Self::Failed => "failed",
        }
    }
}

/// Why a live card source is not advertised as available.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LiveCardUnavailableReason {
    /// No authorized source has supplied an owned snapshot.
    NoActiveSource,
    /// Exact-host compatibility evidence is still required.
    ExactHostEvidenceRequired,
    /// The selected host/build has no supported card extractor.
    UnsupportedBuild,
    /// The caller's scope disallows the projection.
    ScopeDenied,
}

impl LiveCardUnavailableReason {
    /// Returns a stable owner-local spelling.
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

/// Sanitized local read, validation, and identity failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LiveCardError {
    /// Identity construction failed before a source read.
    InvalidIdentity(LiveCardIdentityError),
    /// A fixture or source projection violated an allowlist invariant.
    InvalidProjection(&'static str),
    /// The page bound is zero or exceeds the configured maximum.
    InvalidPageSize,
    /// A continuation was consumed, cross-store, or mismatched.
    InvalidContinuation,
    /// A page/detail identity does not match the current coherent snapshot.
    StaleReference,
    /// The current snapshot has no card with this instance identity.
    InstanceNotFound,
    /// A pile or selector query has no supported source mapping.
    UnknownCollection,
    /// The requested field has no extractor for this card source.
    UnsupportedField(LiveCardField),
    /// A bounded card detail payload exceeds the local limit.
    DetailTooLarge {
        /// Maximum accepted bytes.
        limit: usize,
        /// Measured/declared bytes.
        actual: usize,
    },
    /// A replacement snapshot reused or moved the epoch backwards.
    NonMonotonicEpoch {
        /// Current epoch.
        current: u64,
        /// Supplied replacement epoch.
        supplied: u64,
    },
    /// The source is deliberately unavailable.
    Unavailable(LiveCardUnavailableReason),
}

impl std::fmt::Display for LiveCardError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIdentity(error) => error.fmt(formatter),
            Self::InvalidProjection(reason) => {
                write!(formatter, "invalid card projection: {reason}")
            }
            Self::InvalidPageSize => formatter.write_str("invalid live card page size"),
            Self::InvalidContinuation => formatter.write_str("invalid live card continuation"),
            Self::StaleReference => formatter.write_str("stale live card reference"),
            Self::InstanceNotFound => formatter.write_str("live card instance not found"),
            Self::UnknownCollection => formatter.write_str("unknown live card collection"),
            Self::UnsupportedField(field) => {
                write!(formatter, "unsupported live card field {}", field.code())
            }
            Self::DetailTooLarge { limit, actual } => {
                write!(
                    formatter,
                    "live card detail is {actual} bytes; limit is {limit}"
                )
            }
            Self::NonMonotonicEpoch { current, supplied } => {
                write!(
                    formatter,
                    "live card epoch {supplied} is not newer than {current}"
                )
            }
            Self::Unavailable(reason) => {
                write!(formatter, "live card source unavailable: {}", reason.code())
            }
        }
    }
}

impl std::error::Error for LiveCardError {}
