// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::{
    catalog_reader::RelicCatalog,
    definition::{RelicCounterState, RelicResolvedParameter, RelicVisibilityScope},
    error::{RelicLiveError, RelicSourceError, map_live_source_error},
    live_model::{
        RelicAccumulatedValue, RelicActivationState, RelicInstanceInput, RelicLiveSnapshotInput,
        RelicPendingTrigger,
    },
    live_validation::{
        instance_bytes, validate_against_catalog, validate_binding, validate_instance_identity,
    },
    model::{
        RELIC_MAX_INSTANCES, RELIC_MAX_LIVE_DETAIL_BYTES, RELIC_PRODUCER_VERSION,
        RelicDefinitionReference, RelicField, RelicLiveBinding, RelicOwnerId,
    },
};

/// Owner-local live state source boundary.
pub trait RelicLiveSource {
    /// Copies one coherent, bounded live snapshot for the expected identity.
    fn read_live(
        &self,
        expected: &RelicLiveBinding,
    ) -> Result<RelicLiveSnapshotInput, RelicSourceError>;
}

/// Immutable coherent live snapshot retaining instances by distinct live identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicLiveSnapshot {
    binding: RelicLiveBinding,
    instances: BTreeMap<String, RelicInstanceInput>,
}

impl RelicLiveSnapshot {
    /// Validates source-owned snapshot shape without attaching a static catalog.
    pub fn from_input(input: RelicLiveSnapshotInput) -> Result<Self, RelicLiveError> {
        validate_binding(&input.binding)?;
        if input.instances.len() > RELIC_MAX_INSTANCES {
            return Err(RelicLiveError::InvalidInput("instances"));
        }
        let mut instances = BTreeMap::new();
        for instance in input.instances {
            validate_instance_identity(&instance)?;
            let instance_id = instance.instance_id.clone();
            if instances.insert(instance_id.clone(), instance).is_some() {
                return Err(RelicLiveError::DuplicateInstance(instance_id));
            }
        }
        Ok(Self {
            binding: input.binding,
            instances,
        })
    }

    /// Returns the exact snapshot identity fence.
    #[must_use]
    pub fn binding(&self) -> &RelicLiveBinding {
        &self.binding
    }

    /// Returns the number of retained live instances.
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
pub struct RelicInstanceReference {
    /// Coherent live identity.
    pub live: RelicLiveBinding,
    /// Live instance identity.
    pub instance_id: String,
    /// Static definition identity.
    pub definition_id: String,
    /// Owner identity at the same snapshot.
    pub owner_id: RelicOwnerId,
}

/// Live relic detail joined to a static definition reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicInstance {
    /// Exact live instance identity.
    pub reference: RelicInstanceReference,
    /// Static definition reference, without copying static rule values.
    pub definition: RelicDefinitionReference,
    /// Typed counter state.
    pub counters: RelicField<Vec<RelicCounterState>>,
    /// Activation and used state.
    pub activation: RelicField<RelicActivationState>,
    /// Accumulated values.
    pub accumulated: RelicField<Vec<RelicAccumulatedValue>>,
    /// Pending visible triggers.
    pub pending_triggers: RelicField<Vec<RelicPendingTrigger>>,
    /// Resolved visible parameters linked to static IDs.
    pub resolved_parameters: RelicField<Vec<RelicResolvedParameter>>,
}

/// Reader enforcing catalog, run, snapshot, epoch, definition, and instance fences.
#[derive(Clone, Debug)]
pub struct RelicLiveReader {
    catalog: RelicCatalog,
    snapshot: RelicLiveSnapshot,
    scope: RelicVisibilityScope,
}

impl RelicLiveReader {
    /// Reads and validates one source snapshot under an expected identity fence.
    pub fn from_source<S: RelicLiveSource>(
        catalog: &RelicCatalog,
        expected: &RelicLiveBinding,
        source: &S,
    ) -> Result<Self, RelicLiveError> {
        Self::from_source_with_scope(catalog, expected, source, RelicVisibilityScope::Public)
    }

    /// Reads one source snapshot with an explicit visibility authorization.
    pub fn from_source_with_scope<S: RelicLiveSource>(
        catalog: &RelicCatalog,
        expected: &RelicLiveBinding,
        source: &S,
        scope: RelicVisibilityScope,
    ) -> Result<Self, RelicLiveError> {
        let input = source.read_live(expected).map_err(map_live_source_error)?;
        if input.binding != *expected {
            return Err(RelicLiveError::StaleReference);
        }
        Self::new_with_scope(catalog, RelicLiveSnapshot::from_input(input)?, scope)
    }

    /// Joins a live snapshot to the exact static catalog it references.
    pub fn new(
        catalog: &RelicCatalog,
        snapshot: RelicLiveSnapshot,
    ) -> Result<Self, RelicLiveError> {
        Self::new_with_scope(catalog, snapshot, RelicVisibilityScope::Public)
    }

    /// Joins a live snapshot with an explicit visibility authorization.
    pub fn new_with_scope(
        catalog: &RelicCatalog,
        snapshot: RelicLiveSnapshot,
        scope: RelicVisibilityScope,
    ) -> Result<Self, RelicLiveError> {
        if snapshot.binding.catalog != *catalog.binding() {
            return Err(RelicLiveError::CatalogMismatch);
        }
        if snapshot.binding.catalog.producer_version != RELIC_PRODUCER_VERSION {
            return Err(RelicLiveError::ProducerVersionMismatch);
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
    pub fn binding(&self) -> &RelicLiveBinding {
        self.snapshot.binding()
    }

    /// Returns the authorization scope used for live values.
    #[must_use]
    pub fn scope(&self) -> RelicVisibilityScope {
        self.scope
    }

    /// Replaces the snapshot only when it names the same catalog identity.
    pub fn replace_snapshot(&mut self, snapshot: RelicLiveSnapshot) -> Result<(), RelicLiveError> {
        if snapshot.binding.catalog != *self.catalog.binding() {
            return Err(RelicLiveError::CatalogMismatch);
        }
        for instance in snapshot.instances.values() {
            validate_against_catalog(&self.catalog, instance, self.scope)?;
        }
        self.snapshot = snapshot;
        Ok(())
    }

    /// Returns references in deterministic live-instance identity order.
    #[must_use]
    pub fn references(&self) -> Vec<RelicInstanceReference> {
        self.snapshot
            .instances
            .values()
            .map(|instance| self.reference_for(instance))
            .collect()
    }

    /// Reads one instance without mutating the snapshot or game state.
    pub fn get(&self, reference: &RelicInstanceReference) -> Result<RelicInstance, RelicLiveError> {
        if reference.live != self.snapshot.binding {
            return Err(RelicLiveError::StaleReference);
        }
        let instance = self
            .snapshot
            .instances
            .get(&reference.instance_id)
            .ok_or(RelicLiveError::NotFound)?;
        if instance.definition_id != reference.definition_id
            || instance.owner_id != reference.owner_id
        {
            return Err(RelicLiveError::StaleReference);
        }
        let detail_bytes = instance_bytes(instance);
        if detail_bytes > RELIC_MAX_LIVE_DETAIL_BYTES {
            return Err(RelicLiveError::DetailTooLarge {
                limit: RELIC_MAX_LIVE_DETAIL_BYTES,
                actual: detail_bytes,
            });
        }
        Ok(RelicInstance {
            reference: reference.clone(),
            definition: RelicDefinitionReference {
                catalog: self.catalog.binding().clone(),
                relic_id: instance.definition_id.clone(),
            },
            counters: instance.counters.clone(),
            activation: instance.activation.clone(),
            accumulated: instance.accumulated.clone(),
            pending_triggers: instance.pending_triggers.clone(),
            resolved_parameters: instance.resolved_parameters.clone(),
        })
    }

    fn reference_for(&self, instance: &RelicInstanceInput) -> RelicInstanceReference {
        RelicInstanceReference {
            live: self.snapshot.binding.clone(),
            instance_id: instance.instance_id.clone(),
            definition_id: instance.definition_id.clone(),
            owner_id: instance.owner_id.clone(),
        }
    }
}
