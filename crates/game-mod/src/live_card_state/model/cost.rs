// SPDX-License-Identifier: MIT

use super::{CardExpiration, CardModifierScope};

/// Why a cost could not be represented as a normal fixed amount.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CardCostUnknownReason {
    /// A negative or sentinel host value was observed.
    NegativeOrSentinel,
    /// The source did not expose the value on the current surface.
    NotObserved,
    /// No extractor exists for this cost kind.
    Unsupported,
    /// An unresolved contributor prevents a safe effective cost.
    UnresolvedContributor,
    /// Bounded extraction failed.
    Failed,
}

/// Amount used by an alternate-resource cost.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CardCostAmount {
    /// Fixed non-negative amount.
    Fixed(u16),
    /// Cost scales with the current X value.
    X,
    /// Amount remains explicit but unresolved.
    Unknown {
        /// Raw signed/sentinel value when observed.
        observed: Option<i64>,
        /// Why no ordinary amount was returned.
        reason: CardCostUnknownReason,
    },
}

/// Cost shape for a card instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CardCost {
    /// Ordinary fixed energy/resource amount.
    Fixed(u16),
    /// X cost.
    X,
    /// Card is free.
    Free,
    /// Card cannot be played.
    Unplayable,
    /// Card consumes an alternate resource.
    AlternateResource {
        /// Owner-defined resource identity.
        resource: String,
        /// Fixed, X, or explicit unknown alternate amount.
        amount: CardCostAmount,
    },
    /// Source observed an unsupported/unknown cost without coercing it.
    Unknown {
        /// Raw signed/sentinel value when observed.
        observed: Option<i64>,
        /// Why no typed cost was returned.
        reason: CardCostUnknownReason,
    },
}

/// One visible contributor to current/effective cost.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardCostContributor {
    /// Source effect/enchantment/rule identity.
    pub source_ref: String,
    /// Host-semantic ordering; vector order remains authoritative.
    pub order: u16,
    /// Scope in which the contributor applies.
    pub scope: CardModifierScope,
    /// Optional signed delta or amount.
    pub amount: Option<i64>,
    /// Cost replacement or explicit unknown semantics.
    pub value: CardCost,
    /// Expiration copied from the source modifier when known.
    pub expiration: CardExpiration,
}

/// Base/current/effective cost semantics and visible contributors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardCostSemantics {
    /// Definition/base cost before instance modifiers.
    pub base: CardCost,
    /// Cost after active instance modifiers.
    pub current: CardCost,
    /// Cost applicable to the current decision/action.
    pub effective: CardCost,
    /// Ordered visible contributors; unknown contributors remain entries.
    pub contributors: Vec<CardCostContributor>,
}
