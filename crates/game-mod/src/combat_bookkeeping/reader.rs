// SPDX-License-Identifier: MIT

use super::model::CombatBookkeepingBinding;
use super::source::map_source_error;
use super::{
    CombatBookkeepingError, CombatBookkeepingSnapshot, CombatBookkeepingSource,
    CombatCardInstanceReference, CombatCardMembership, CombatCounter, CombatCounterKind,
    CombatField, CombatPending, CombatTurnIdentity, CombatVisibilityScope, CombatZone,
    CombatZoneKind, CombatZoneStatus,
};

/// Read-only owner-local reader for a coherent combat bookkeeping snapshot.
#[derive(Clone, Debug)]
pub struct CombatBookkeepingReader {
    snapshot: CombatBookkeepingSnapshot,
    scope: CombatVisibilityScope,
}

impl CombatBookkeepingReader {
    /// Adopts an already validated snapshot with public visibility.
    pub fn new(snapshot: CombatBookkeepingSnapshot) -> Result<Self, CombatBookkeepingError> {
        Self::new_with_scope(snapshot, CombatVisibilityScope::Public)
    }

    /// Adopts an already validated snapshot with explicit visibility.
    pub fn new_with_scope(
        snapshot: CombatBookkeepingSnapshot,
        scope: CombatVisibilityScope,
    ) -> Result<Self, CombatBookkeepingError> {
        Ok(Self { snapshot, scope })
    }

    /// Reads and validates one source snapshot against an exact identity fence.
    pub fn from_source<S: CombatBookkeepingSource>(
        source: &S,
        expected: &CombatBookkeepingBinding,
    ) -> Result<Self, CombatBookkeepingError> {
        Self::from_source_with_scope(source, expected, CombatVisibilityScope::Public)
    }

    /// Reads one source snapshot with explicit public/owner visibility.
    pub fn from_source_with_scope<S: CombatBookkeepingSource>(
        source: &S,
        expected: &CombatBookkeepingBinding,
        scope: CombatVisibilityScope,
    ) -> Result<Self, CombatBookkeepingError> {
        if let super::CombatBookkeepingCapability::Unavailable(reason) = source.capability() {
            return Err(CombatBookkeepingError::Unavailable(reason));
        }
        let input = source
            .read_snapshot(expected, scope)
            .map_err(map_source_error)?;
        if input.binding != *expected {
            return Err(CombatBookkeepingError::StaleReference);
        }
        Self::new_with_scope(CombatBookkeepingSnapshot::from_input(input)?, scope)
    }

    /// Returns the current identity fence.
    #[must_use]
    pub fn binding(&self) -> &CombatBookkeepingBinding {
        self.snapshot.binding()
    }

    /// Returns the selected visibility scope.
    #[must_use]
    pub const fn scope(&self) -> CombatVisibilityScope {
        self.scope
    }

    /// Returns the current turn identity field after scope checking.
    pub fn turn(&self) -> Result<&CombatField<CombatTurnIdentity>, CombatBookkeepingError> {
        self.visible(self.snapshot.turn(), "turn")
    }

    /// Returns the explicit status for one zone.
    #[must_use]
    pub fn zone_status(&self, kind: CombatZoneKind) -> CombatZoneStatus {
        self.snapshot.zone_inventory().status(kind)
    }

    /// Returns one available zone without inventing an empty unsupported zone.
    pub fn zone(&self, kind: CombatZoneKind) -> Result<&CombatZone, CombatBookkeepingError> {
        if self.zone_status(kind) != CombatZoneStatus::Available {
            return Err(CombatBookkeepingError::ZoneUnavailable {
                zone: kind,
                status: self.zone_status(kind),
            });
        }
        self.snapshot
            .zone(kind)
            .ok_or(CombatBookkeepingError::ZoneNotFound(kind))
    }

    /// Returns a draw-pile count while keeping its composition independent.
    pub fn draw_count(&self) -> Result<&CombatField<u32>, CombatBookkeepingError> {
        Ok(&self.zone(CombatZoneKind::Draw)?.total)
    }

    /// Returns public/owner-permitted composition without exposing secret order.
    pub fn zone_composition(
        &self,
        kind: CombatZoneKind,
    ) -> Result<&CombatField<Vec<CombatCardMembership>>, CombatBookkeepingError> {
        self.visible(&self.zone(kind)?.composition, "zone.composition")
    }

    /// Returns one named typed counter.
    #[must_use]
    pub fn counter(&self, kind: CombatCounterKind) -> &CombatCounter {
        self.snapshot.counter(kind)
    }

    /// Returns permanent/combat/temporary reconciliation fields.
    #[must_use]
    pub fn deck(&self) -> &super::CombatDeckReconciliation {
        self.snapshot.deck()
    }

    /// Returns the current resolving state.
    pub fn resolution(
        &self,
    ) -> Result<&CombatField<super::CombatResolutionState>, CombatBookkeepingError> {
        self.visible(self.snapshot.resolution(), "resolution")
    }

    /// Returns pending public selections/effects.
    pub fn pending(&self) -> Result<&CombatField<CombatPending>, CombatBookkeepingError> {
        self.visible(self.snapshot.pending(), "pending")
    }

    /// Looks up a live card by an identity reference from this exact snapshot.
    pub fn card(
        &self,
        reference: &CombatCardInstanceReference,
    ) -> Result<&CombatCardMembership, CombatBookkeepingError> {
        if reference.binding != *self.snapshot.binding() {
            return Err(CombatBookkeepingError::StaleReference);
        }
        for kind in CombatZoneKind::all() {
            let Some(zone) = self.snapshot.zone(*kind) else {
                continue;
            };
            let Some(cards) = zone.composition.value() else {
                continue;
            };
            if let Some(card) = cards
                .iter()
                .find(|card| card.card.instance_id == reference.instance_id)
            {
                if card.card.definition_id != reference.definition_id
                    || card.card.owner_id != reference.owner_id
                {
                    return Err(CombatBookkeepingError::StaleReference);
                }
                return Ok(card);
            }
        }
        Err(CombatBookkeepingError::CardNotFound)
    }

    /// Replaces the source snapshot only within the same combat identity.
    pub fn replace_snapshot(
        &mut self,
        next: CombatBookkeepingSnapshot,
    ) -> Result<(), CombatBookkeepingError> {
        if next.binding().content_manifest != self.snapshot.binding().content_manifest {
            return Err(CombatBookkeepingError::InvalidBinding("content_manifest"));
        }
        if next.binding().game_instance_id != self.snapshot.binding().game_instance_id {
            return Err(CombatBookkeepingError::GameInstanceMismatch);
        }
        if next.binding().run_id != self.snapshot.binding().run_id {
            return Err(CombatBookkeepingError::RunMismatch);
        }
        if next.binding().combat_id != self.snapshot.binding().combat_id {
            return Err(CombatBookkeepingError::CombatMismatch);
        }
        if next.binding().epoch <= self.snapshot.binding().epoch {
            return Err(CombatBookkeepingError::NonMonotonicEpoch {
                current: self.snapshot.binding().epoch,
                supplied: next.binding().epoch,
            });
        }
        self.snapshot = next;
        Ok(())
    }

    fn visible<'a, T>(
        &self,
        field: &'a CombatField<T>,
        name: &'static str,
    ) -> Result<&'a CombatField<T>, CombatBookkeepingError> {
        if self.scope == CombatVisibilityScope::Public
            && matches!(
                field.status(),
                super::CombatFieldStatus::OwnerOnly | super::CombatFieldStatus::Denied
            )
        {
            return Err(CombatBookkeepingError::VisibilityDenied(name));
        }
        Ok(field)
    }
}
