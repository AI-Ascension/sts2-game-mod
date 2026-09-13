// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

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

/// Availability of a pile or selector in the current source inventory.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LiveCardCollectionStatus {
    /// The source supports this collection, including an explicitly empty result.
    Available,
    /// The source supports the concept but did not observe it in this snapshot.
    NotObserved,
    /// The source has no extractor for this collection kind.
    Unsupported,
}

impl LiveCardCollectionStatus {
    /// Returns a stable owner-local spelling for diagnostics and fixtures.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::NotObserved => "not_observed",
            Self::Unsupported => "unsupported",
        }
    }
}

/// Explicit collection inventory carried by a coherent snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveCardCollectionInventory {
    /// Status keyed by an exact pile or selector identity.
    pub statuses: BTreeMap<LiveCardCollection, LiveCardCollectionStatus>,
}

impl LiveCardCollectionInventory {
    /// Creates an inventory with the all-visible collection available.
    #[must_use]
    pub fn new() -> Self {
        Self {
            statuses: BTreeMap::from([(
                LiveCardCollection::AllVisible,
                LiveCardCollectionStatus::Available,
            )]),
        }
    }

    /// Sets a bounded source status for one collection.
    pub fn set(&mut self, collection: LiveCardCollection, status: LiveCardCollectionStatus) {
        self.statuses.insert(collection, status);
    }

    /// Returns the explicit status, defaulting to not observed.
    #[must_use]
    pub fn status(&self, collection: &LiveCardCollection) -> LiveCardCollectionStatus {
        self.statuses
            .get(collection)
            .copied()
            .unwrap_or(LiveCardCollectionStatus::NotObserved)
    }
}

impl Default for LiveCardCollectionInventory {
    fn default() -> Self {
        Self::new()
    }
}
