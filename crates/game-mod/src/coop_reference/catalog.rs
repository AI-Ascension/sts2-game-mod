// SPDX-License-Identifier: MIT

//! The owner-local snapshot, source boundary and producer that bind a party to one manifest.

use crate::ContentManifest;

use super::error::map_source_error;
use super::validation::validate_party;
use super::{
    COOP_REFERENCE_PRODUCER_VERSION, CoopCatalogBinding, CoopError, CoopFamilyCoverage,
    CoopFamilyState, CoopPartyInput, CoopPartyRecord, CoopSourceError, catalog_reader::CoopCatalog,
};

/// Bounded source snapshot used to construct one immutable co-op party catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale every localized party value was reported in.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
    /// Explicit support state for the party family with the counts the source reports.
    pub family: CoopFamilyCoverage,
    /// The party record, empty when the source declares the family unavailable.
    pub party: CoopPartyInput,
}

/// Owner-local source boundary for copied co-op party records.
pub trait CoopCatalogSource {
    /// Copies a bounded party record without acting in the party or touching a save.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<CoopCatalogSnapshot, CoopSourceError>;
}

/// Producer that binds one co-op party to an immutable content manifest and locale.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CoopCatalogProducer;

impl CoopCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: CoopCatalogSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<CoopCatalog, CoopError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        validate_fences(manifest, &snapshot)?;
        let binding = CoopCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            producer_version: snapshot.producer_version,
        };
        let family = snapshot.family;
        if family.state != CoopFamilyState::Handled {
            if !is_empty_party(&snapshot.party) {
                return Err(CoopError::InvalidInput("party"));
            }
            return Ok(CoopCatalog::from_parts(binding, family, None));
        }
        validate_party(&snapshot.party, manifest)?;
        if family.peer_count != snapshot.party.peers.len()
            || family.effect_count != snapshot.party.effects.len()
            || family.scaling_count != snapshot.party.scaling.len()
        {
            return Err(CoopError::FamilyCountMismatch);
        }
        let record = CoopPartyRecord {
            binding: binding.clone(),
            party: snapshot.party,
        };
        Ok(CoopCatalog::from_parts(binding, family, Some(record)))
    }
}

/// Returns whether a declared-unavailable family carries no party record at all.
fn is_empty_party(party: &CoopPartyInput) -> bool {
    party.party_id.is_empty()
        && party.instance_id.is_empty()
        && party.run_id.is_empty()
        && party.peers.is_empty()
        && party.effects.is_empty()
        && party.scaling.is_empty()
}

fn validate_fences(
    manifest: &ContentManifest,
    snapshot: &CoopCatalogSnapshot,
) -> Result<(), CoopError> {
    if snapshot.manifest != manifest.cursor_binding() {
        return Err(CoopError::ManifestMismatch);
    }
    if snapshot.locale != manifest.locale {
        return Err(CoopError::LocaleMismatch);
    }
    if snapshot.producer_version != COOP_REFERENCE_PRODUCER_VERSION {
        return Err(CoopError::ProducerVersionMismatch);
    }
    Ok(())
}
