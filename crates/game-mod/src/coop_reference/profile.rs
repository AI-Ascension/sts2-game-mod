// SPDX-License-Identifier: MIT

//! The local member's own view, and the single-player save identity a party read never touches.

use super::{
    CoopCatalogReader, CoopError, CoopPartyInput, CoopPartyReference, CoopPeerView,
    CoopReadAuthority, CoopReadScope,
};

/// Refuses a party whose own identity aliases the run or the instance it belongs to.
///
/// A party is scoped to one live instance and one run; it is never the save profile. Reusing those
/// identities would let a caller resolve a party read against a save-scoped name, so the identity
/// namespaces are kept disjoint instead of being compared later.
pub(super) fn validate_party_scope(party: &CoopPartyInput) -> Result<(), CoopError> {
    if party.party_id == party.instance_id {
        return Err(CoopError::IdentityNamespaceCollision("party_id"));
    }
    if party.party_id == party.run_id {
        return Err(CoopError::IdentityNamespaceCollision("party_id"));
    }
    Ok(())
}

impl CoopCatalogReader {
    /// Reads the local member's own view together with every ally the party scope observes.
    ///
    /// This is the only projection that may carry a local-only field, and it carries it for the
    /// local member alone. It selects no save profile and loads no save.
    pub fn local_view(&self) -> Result<CoopLocalView, CoopError> {
        self.validate_family()?;
        let record = self.party_record()?;
        let local = record
            .party
            .local_peer()
            .ok_or(CoopError::MissingLocalPeer)?;
        let local_view = self.peer(&record.peer_reference(&local.peer_id), CoopReadScope::Peer)?;
        let mut allies = Vec::new();
        for peer in &record.party.peers {
            if peer.peer_id != local.peer_id {
                allies
                    .push(self.peer(&record.peer_reference(&peer.peer_id), CoopReadScope::Party)?);
            }
        }
        Ok(CoopLocalView {
            reference: CoopPartyReference {
                catalog: record.binding.clone(),
                party_id: record.party.party_id.clone(),
            },
            authority: CoopReadAuthority::NotGranted,
            local: local_view,
            allies,
        })
    }
}

/// The local member's own view and the allies a party scope observes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopLocalView {
    /// Exact static reference to the party this view was taken from.
    pub reference: CoopPartyReference,
    /// Capability this read does not grant.
    pub authority: CoopReadAuthority,
    /// The local member, the only row that may carry a local-only field.
    pub local: CoopPeerView,
    /// Every other member, projected at the party scope.
    pub allies: Vec<CoopPeerView>,
}
