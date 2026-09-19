// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::{
    binding::{RunConfigurationBinding, RunConfigurationCacheKey, RunConfigurationLiveBinding},
    model::{
        RunConfigurationFamilyCoverage, RunFieldKind, RunFieldRecord, RunModifier, RunSeedPolicy,
    },
    reader::RunConfigurationReader,
};

/// Whether every required field carries a settled host value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunConfigurationCompleteness {
    /// Every required field carries a settled host value.
    Complete,
    /// The named required fields carry no settled host value.
    Partial {
        /// Required field kinds without a settled host value.
        missing: Vec<RunFieldKind>,
    },
}

impl RunConfigurationCompleteness {
    /// Returns whether the record may be presented as a complete configuration.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }

    /// Returns the required field kinds that carry no settled value.
    #[must_use]
    pub fn missing(&self) -> &[RunFieldKind] {
        match self {
            Self::Complete => &[],
            Self::Partial { missing } => missing,
        }
    }
}

/// Exact static reference to one run configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunConfigurationDefinitionReference {
    /// Catalog witness the reference was produced under.
    pub catalog: RunConfigurationBinding,
    /// Run identity settled at admission.
    pub run_id: String,
    /// Configuration revision the reference was produced for.
    pub revision: u64,
}

/// Immutable run configuration bound to one catalog and live witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunConfigurationDefinition {
    /// Exact static definition reference.
    pub reference: RunConfigurationDefinitionReference,
    /// Catalog witness.
    pub binding: RunConfigurationBinding,
    /// Live witness.
    pub live: RunConfigurationLiveBinding,
    /// Seed visibility policy applied to the run.
    pub seed_policy: RunSeedPolicy,
    /// Requested and settled field records by kind.
    pub fields: BTreeMap<RunFieldKind, RunFieldRecord>,
    /// Declared modifiers.
    pub modifiers: Vec<RunModifier>,
    /// Whether every required field carries a settled host value.
    pub completeness: RunConfigurationCompleteness,
    /// Cache key that includes seed material when the policy makes the seed visible.
    pub cache: RunConfigurationCacheKey,
    /// Cache key computed without seed material.
    pub seed_blind_cache: RunConfigurationCacheKey,
}

impl RunConfigurationDefinition {
    /// Returns the settled field record for one kind.
    #[must_use]
    pub fn field(&self, kind: RunFieldKind) -> Option<&RunFieldRecord> {
        self.fields.get(&kind)
    }
}

/// Immutable run-configuration catalog fenced by one content manifest and profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunConfigurationCatalog {
    pub(super) binding: RunConfigurationBinding,
    pub(super) family: RunConfigurationFamilyCoverage,
    pub(super) definitions: BTreeMap<String, RunConfigurationDefinition>,
}

impl RunConfigurationCatalog {
    pub(super) fn from_parts(
        binding: RunConfigurationBinding,
        family: RunConfigurationFamilyCoverage,
        definitions: BTreeMap<String, RunConfigurationDefinition>,
    ) -> Self {
        Self {
            binding,
            family,
            definitions,
        }
    }

    /// Returns the catalog witness every definition is bound to.
    #[must_use]
    pub fn binding(&self) -> &RunConfigurationBinding {
        &self.binding
    }

    /// Returns the declared family support state and count.
    #[must_use]
    pub fn family(&self) -> &RunConfigurationFamilyCoverage {
        &self.family
    }

    /// Returns the number of retained run configurations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Returns whether no run configuration was retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Returns one retained run configuration by identity.
    #[must_use]
    pub fn definition(&self, run_id: &str) -> Option<&RunConfigurationDefinition> {
        self.definitions.get(run_id)
    }

    /// Consumes the catalog into an independent reader.
    #[must_use]
    pub fn reader(self) -> RunConfigurationReader {
        RunConfigurationReader::new(self)
    }
}
