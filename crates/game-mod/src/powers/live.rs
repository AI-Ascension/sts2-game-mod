// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::definition::PowerStatusFamilyState;
use super::{
    catalog_reader::PowerStatusCatalog,
    error::{PowerStatusLiveError, PowerStatusSourceError, map_live_source_error},
    live_model::{
        PowerStatusAmount, PowerStatusDurationState, PowerStatusInstanceInput,
        PowerStatusLiveSnapshotInput, PowerStatusOwner, PowerStatusOwnerKind,
        PowerStatusPendingExpiry, PowerStatusSourceReference,
    },
    live_validation::{
        instance_bytes, validate_against_catalog, validate_binding, validate_instance_identity,
    },
    model::{
        POWER_STATUS_MAX_INSTANCES, POWER_STATUS_MAX_LIVE_DETAIL_BYTES,
        POWER_STATUS_PRODUCER_VERSION, PowerStatusDefinitionReference, PowerStatusField,
        PowerStatusLiveBinding, PowerStatusVisibilityScope,
    },
};

/// Owner-local live status source boundary.
pub trait PowerStatusLiveSource {
    /// Copies one coherent, bounded live snapshot for the expected identity.
    fn read_live(
        &self,
        expected: &PowerStatusLiveBinding,
    ) -> Result<PowerStatusLiveSnapshotInput, PowerStatusSourceError>;
}

/// Immutable coherent live snapshot retaining instances by distinct live identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusLiveSnapshot {
    binding: PowerStatusLiveBinding,
    instances: BTreeMap<String, PowerStatusInstanceInput>,
}

impl PowerStatusLiveSnapshot {
    /// Validates source-owned snapshot shape without attaching a static catalog.
    pub fn from_input(input: PowerStatusLiveSnapshotInput) -> Result<Self, PowerStatusLiveError> {
        validate_binding(&input.binding)?;
        if input.instances.len() > POWER_STATUS_MAX_INSTANCES {
            return Err(PowerStatusLiveError::InvalidInput("instances"));
        }
        let mut instances = BTreeMap::new();
        for instance in input.instances {
            validate_instance_identity(&instance)?;
            let instance_id = instance.instance_id.clone();
            if instances.insert(instance_id.clone(), instance).is_some() {
                return Err(PowerStatusLiveError::DuplicateInstance(instance_id));
            }
        }
        Ok(Self {
            binding: input.binding,
            instances,
        })
    }

    /// Returns the exact snapshot identity fence.
    #[must_use]
    pub fn binding(&self) -> &PowerStatusLiveBinding {
        &self.binding
    }

    /// Returns the number of retained instances.
    #[must_use]
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// Whether the snapshot has no live instances.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }
}

/// Exact live instance reference; definition and instance identities remain distinct.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PowerStatusInstanceReference {
    /// Coherent live identity.
    pub live: PowerStatusLiveBinding,
    /// Live instance identity.
    pub instance_id: String,
    /// Static definition identity.
    pub definition_id: String,
    /// Owner family.
    pub owner_kind: PowerStatusOwnerKind,
    /// Owner identity at the same snapshot.
    pub owner_id: super::model::PowerStatusOwnerId,
}

/// Live detail joined to a static definition reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusInstance {
    /// Exact live instance identity.
    pub reference: PowerStatusInstanceReference,
    /// Static definition reference, without copying static rules.
    pub definition: PowerStatusDefinitionReference,
    /// Owner entity.
    pub owner: PowerStatusOwner,
    /// Visible source/creator.
    pub source: PowerStatusField<PowerStatusSourceReference>,
    /// Typed amount with explicit availability.
    pub amount: PowerStatusField<PowerStatusAmount>,
    /// Duration state with explicit availability.
    pub duration: PowerStatusField<PowerStatusDurationState>,
    /// Application order with explicit availability.
    pub application_order: PowerStatusField<u32>,
    /// Active state.
    pub active: PowerStatusField<bool>,
    /// Suppressed state.
    pub suppressed: PowerStatusField<bool>,
    /// Pending visible expiry.
    pub pending_expiry: PowerStatusField<PowerStatusPendingExpiry>,
}

/// Reader enforcing catalog, game, run, snapshot, epoch, definition, and instance fences.
#[derive(Clone, Debug)]
pub struct PowerStatusLiveReader {
    catalog: PowerStatusCatalog,
    snapshot: PowerStatusLiveSnapshot,
    scope: PowerStatusVisibilityScope,
}

impl PowerStatusLiveReader {
    /// Reads and validates one source snapshot under an expected identity fence.
    pub fn from_source<S: PowerStatusLiveSource>(
        catalog: &PowerStatusCatalog,
        expected: &PowerStatusLiveBinding,
        source: &S,
    ) -> Result<Self, PowerStatusLiveError> {
        Self::from_source_with_scope(
            catalog,
            expected,
            source,
            PowerStatusVisibilityScope::Public,
        )
    }

    /// Reads one source snapshot with explicit visibility authorization.
    pub fn from_source_with_scope<S: PowerStatusLiveSource>(
        catalog: &PowerStatusCatalog,
        expected: &PowerStatusLiveBinding,
        source: &S,
        scope: PowerStatusVisibilityScope,
    ) -> Result<Self, PowerStatusLiveError> {
        let input = source.read_live(expected).map_err(map_live_source_error)?;
        if input.binding != *expected {
            return Err(PowerStatusLiveError::StaleReference);
        }
        Self::new_with_scope(catalog, PowerStatusLiveSnapshot::from_input(input)?, scope)
    }

    /// Joins a live snapshot to the exact static catalog it references.
    pub fn new(
        catalog: &PowerStatusCatalog,
        snapshot: PowerStatusLiveSnapshot,
    ) -> Result<Self, PowerStatusLiveError> {
        Self::new_with_scope(catalog, snapshot, PowerStatusVisibilityScope::Public)
    }

    /// Joins a live snapshot with explicit visibility authorization.
    pub fn new_with_scope(
        catalog: &PowerStatusCatalog,
        snapshot: PowerStatusLiveSnapshot,
        scope: PowerStatusVisibilityScope,
    ) -> Result<Self, PowerStatusLiveError> {
        if snapshot.binding.catalog != *catalog.binding() {
            return Err(PowerStatusLiveError::CatalogMismatch);
        }
        if snapshot.binding.catalog.producer_version != POWER_STATUS_PRODUCER_VERSION {
            return Err(PowerStatusLiveError::ProducerVersionMismatch);
        }
        match catalog.family().state {
            PowerStatusFamilyState::Handled => {}
            PowerStatusFamilyState::Unsupported => {
                return Err(PowerStatusLiveError::UnsupportedFamily);
            }
            PowerStatusFamilyState::Unavailable => {
                return Err(PowerStatusLiveError::UnavailableFamily);
            }
        }
        for instance in snapshot.instances.values() {
            validate_against_catalog(catalog, instance, scope)?;
        }
        Ok(Self {
            catalog: catalog.clone(),
            snapshot,
            scope,
        })
    }

    /// Returns the current coherent live binding.
    #[must_use]
    pub fn binding(&self) -> &PowerStatusLiveBinding {
        &self.snapshot.binding
    }

    /// Returns the scope used for live values.
    #[must_use]
    pub fn scope(&self) -> PowerStatusVisibilityScope {
        self.scope
    }

    /// Replaces the snapshot only when catalog, game, and run identities match and epoch grows.
    pub fn replace_snapshot(
        &mut self,
        snapshot: PowerStatusLiveSnapshot,
    ) -> Result<(), PowerStatusLiveError> {
        if snapshot.binding.catalog != *self.catalog.binding() {
            return Err(PowerStatusLiveError::CatalogMismatch);
        }
        if snapshot.binding.game_instance_id != self.snapshot.binding.game_instance_id {
            return Err(PowerStatusLiveError::GameInstanceMismatch);
        }
        if snapshot.binding.run_id != self.snapshot.binding.run_id {
            return Err(PowerStatusLiveError::RunMismatch);
        }
        if snapshot.binding.epoch <= self.snapshot.binding.epoch {
            return Err(PowerStatusLiveError::NonMonotonicEpoch {
                current: self.snapshot.binding.epoch,
                supplied: snapshot.binding.epoch,
            });
        }
        for instance in snapshot.instances.values() {
            validate_against_catalog(&self.catalog, instance, self.scope)?;
        }
        self.snapshot = snapshot;
        Ok(())
    }

    /// Returns references in deterministic live-instance identity order.
    #[must_use]
    pub fn references(&self) -> Vec<PowerStatusInstanceReference> {
        self.snapshot
            .instances
            .values()
            .map(|instance| self.reference_for(instance))
            .collect()
    }

    /// Reads one instance without mutating the snapshot or source.
    pub fn get(
        &self,
        reference: &PowerStatusInstanceReference,
    ) -> Result<PowerStatusInstance, PowerStatusLiveError> {
        if reference.live != self.snapshot.binding {
            return Err(PowerStatusLiveError::StaleReference);
        }
        let instance = self
            .snapshot
            .instances
            .get(&reference.instance_id)
            .ok_or(PowerStatusLiveError::NotFound)?;
        if instance.definition_id != reference.definition_id
            || instance.owner.id != reference.owner_id
            || instance.owner.kind != reference.owner_kind
        {
            return Err(PowerStatusLiveError::StaleReference);
        }
        let detail_bytes = instance_bytes(instance);
        if detail_bytes > POWER_STATUS_MAX_LIVE_DETAIL_BYTES {
            return Err(PowerStatusLiveError::DetailTooLarge {
                limit: POWER_STATUS_MAX_LIVE_DETAIL_BYTES,
                actual: detail_bytes,
            });
        }
        Ok(PowerStatusInstance {
            reference: reference.clone(),
            definition: PowerStatusDefinitionReference {
                catalog: self.catalog.binding().clone(),
                definition_id: instance.definition_id.clone(),
            },
            owner: instance.owner.clone(),
            source: instance.source.clone(),
            amount: instance.amount.clone(),
            duration: instance.duration.clone(),
            application_order: instance.application_order.clone(),
            active: instance.active.clone(),
            suppressed: instance.suppressed.clone(),
            pending_expiry: instance.pending_expiry.clone(),
        })
    }

    fn reference_for(&self, instance: &PowerStatusInstanceInput) -> PowerStatusInstanceReference {
        PowerStatusInstanceReference {
            live: self.snapshot.binding.clone(),
            instance_id: instance.instance_id.clone(),
            definition_id: instance.definition_id.clone(),
            owner_kind: instance.owner.kind.clone(),
            owner_id: instance.owner.id.clone(),
        }
    }
}
