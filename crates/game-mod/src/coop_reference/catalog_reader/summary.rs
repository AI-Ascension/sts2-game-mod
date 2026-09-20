// SPDX-License-Identifier: MIT

use super::super::{CoopPeerReference, CoopPeerView};
use super::page::CoopPeerSummary;

/// Builds one bounded member entry that preserves every field's stated availability.
pub(super) fn peer_summary(view: &CoopPeerView) -> CoopPeerSummary {
    CoopPeerSummary {
        reference: CoopPeerReference {
            catalog: view.reference.catalog.clone(),
            party_id: view.reference.party_id.clone(),
            peer_id: view.reference.peer_id.clone(),
            generation: view.reference.generation,
        },
        role: view.role,
        membership: view.membership,
        freshness: view.freshness,
        character_id: view.character_id.clone(),
    }
}
