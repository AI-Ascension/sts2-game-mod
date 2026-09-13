// SPDX-License-Identifier: MIT

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use super::{
    CardCostSemantics, CardEffectValue, CardFlags, CardLocation, CardModifier, CardOwner,
    CardUpgrade,
};
use crate::live_card_state::LiveCardField;
use crate::live_card_state::identity::{
    CardDefinitionReference, CardInstanceReference, LiveCardReadReference,
};

/// Shared typed values returned by a field-level detail read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LiveCardValue {
    /// Common content definition link.
    Definition(CardDefinitionReference),
    /// Owner identity.
    Owner(CardOwner),
    /// Current location.
    Location(CardLocation),
    /// Upgrade/variant state.
    Upgrade(CardUpgrade),
    /// Ordered modifiers.
    Modifiers(Vec<CardModifier>),
    /// Retain/exhaust/ethereal flags.
    Flags(CardFlags),
    /// Effect-parameter overrides.
    EffectParameters(BTreeMap<String, CardEffectValue>),
    /// Cost semantics.
    Cost(CardCostSemantics),
}

/// Complete bounded projection of one live card instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveCardProjection {
    /// Distinct instance reference bound to one read fence.
    pub instance: CardInstanceReference,
    /// Common static definition reference.
    pub definition: CardDefinitionReference,
    /// Current owner.
    pub owner: CardOwner,
    /// Current pile or selector location.
    pub location: CardLocation,
    /// Upgrade count/path/variant.
    pub upgrade: CardUpgrade,
    /// Ordered permanent and temporary modifiers.
    pub modifiers: Vec<CardModifier>,
    /// Retain/exhaust/ethereal and other flags.
    pub flags: CardFlags,
    /// Effect parameter overrides keyed by stable parameter name.
    pub effect_parameter_overrides: BTreeMap<String, CardEffectValue>,
    /// Base/current/effective cost semantics.
    pub cost: CardCostSemantics,
}

impl LiveCardProjection {
    /// Returns the distinct live instance ID.
    #[must_use]
    pub fn instance_id(&self) -> &str {
        &self.instance.instance_id
    }

    /// Returns the content definition identity.
    #[must_use]
    pub fn definition_ref(&self) -> &CardDefinitionReference {
        &self.definition
    }
}

/// Synthetic source record used by deterministic owner-local tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveCardFixture {
    /// Complete projection copied from a synthetic source.
    pub projection: LiveCardProjection,
    /// Conservative encoded-size estimate for bounded reads.
    pub estimated_bytes: usize,
    /// Fields deliberately unavailable on this fixture's source surface.
    pub unsupported_fields: BTreeSet<LiveCardField>,
}

impl LiveCardFixture {
    /// Creates a fixture with every projection field available.
    #[must_use]
    pub fn new(projection: LiveCardProjection, estimated_bytes: usize) -> Self {
        Self {
            projection,
            estimated_bytes,
            unsupported_fields: BTreeSet::new(),
        }
    }

    /// Marks fields unavailable without inventing a replacement value.
    #[must_use]
    pub fn with_unsupported_fields(
        mut self,
        fields: impl IntoIterator<Item = LiveCardField>,
    ) -> Self {
        self.unsupported_fields.extend(fields);
        self
    }
}

/// One coherent live-card snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveCardSnapshot {
    /// Fence shared by every instance in the snapshot.
    pub reference: LiveCardReadReference,
    /// Source records in owner-provided order.
    pub cards: Vec<LiveCardFixture>,
    /// Whether the source can report an exact collection total.
    pub total_known: bool,
}

impl LiveCardSnapshot {
    /// Creates a snapshot; structural checks run when a store adopts it.
    #[must_use]
    pub fn new(
        reference: LiveCardReadReference,
        cards: impl IntoIterator<Item = LiveCardFixture>,
        total_known: bool,
    ) -> Self {
        Self {
            reference,
            cards: cards.into_iter().collect(),
            total_known,
        }
    }
}

/// Collection scope for a bounded page.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LiveCardCollection {
    /// Every card visible to the owner-local source.
    AllVisible,
    /// Cards in one typed pile.
    Pile(super::CardPile),
    /// Cards in an explicit selector/reward offer.
    Selector(String),
}

/// Bounded page request for pile and selector detail.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveCardQuery {
    /// Pile/selector scope.
    pub collection: LiveCardCollection,
    /// Maximum entries in this page.
    pub limit: usize,
    /// Opaque single-use continuation.
    pub continuation: Option<LiveCardContinuation>,
}

impl LiveCardQuery {
    /// Creates a first-page query.
    #[must_use]
    pub const fn new(collection: LiveCardCollection, limit: usize) -> Self {
        Self {
            collection,
            limit,
            continuation: None,
        }
    }
}

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct LiveCardContinuationScope;

/// Opaque single-use local page continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LiveCardContinuation {
    pub(crate) token: String,
    pub(crate) scope: Arc<LiveCardContinuationScope>,
}

impl LiveCardContinuation {
    pub(crate) fn scoped(token: impl Into<String>, scope: Arc<LiveCardContinuationScope>) -> Self {
        Self {
            token: token.into(),
            scope,
        }
    }

    pub(crate) fn scope_matches(&self, scope: &Arc<LiveCardContinuationScope>) -> bool {
        Arc::ptr_eq(&self.scope, scope)
    }

    /// Returns the opaque token for deterministic fixture assertions.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Whether a page has returned every matching instance.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LiveCardPageCompleteness {
    /// No matching instances remain.
    Complete,
    /// More matching instances exist and a continuation is supplied.
    Partial,
}

/// Bounded page of complete card projections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveCardPage {
    /// Requested pile/selector scope.
    pub collection: LiveCardCollection,
    /// Projections in source order.
    pub entries: Vec<LiveCardProjection>,
    /// Exact matching count when known.
    pub total: Option<usize>,
    /// Complete or explicitly partial page.
    pub completeness: LiveCardPageCompleteness,
    /// Present only for a partial page.
    pub continuation: Option<LiveCardContinuation>,
    /// Fence shared by every entry.
    pub reference: LiveCardReadReference,
}

impl LiveCardPage {
    /// Number of instances returned in this page.
    #[must_use]
    pub const fn returned_count(&self) -> usize {
        self.entries.len()
    }

    /// Whether the source supplied an exact count.
    #[must_use]
    pub const fn count_known(&self) -> bool {
        self.total.is_some()
    }
}

/// A detail response with an explicit field map.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveCardDetail {
    /// Stable instance identity.
    pub instance: CardInstanceReference,
    /// Requested fields and their typed values.
    pub fields: BTreeMap<LiveCardField, LiveCardValue>,
    /// Fence shared by every field.
    pub reference: LiveCardReadReference,
}

/// A compatibility alias for callers that call a definition link a card definition link.
pub type CardDefinitionLink = CardDefinitionReference;
