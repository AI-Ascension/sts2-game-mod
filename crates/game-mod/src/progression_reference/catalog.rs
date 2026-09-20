// SPDX-License-Identifier: MIT

//! The owner-local snapshot, read port and producer that bind progression to one manifest.

use std::collections::BTreeMap;

use crate::ContentManifest;

use super::error::map_source_error;
use super::validation::{validate_capability, validate_entry};
use super::{
    PROGRESSION_MAX_ENTRIES, PROGRESSION_REFERENCE_PRODUCER_VERSION, ProgressionCatalogBinding,
    ProgressionDomainCoverage, ProgressionEntry, ProgressionEntryInput, ProgressionProfileInput,
    ProgressionProfileKind, ProgressionProfilePermit, ProgressionProfileQuery,
    ProgressionReferenceError, ProgressionSourceError, catalog_reader::ProgressionCatalog,
};

/// Bounded source snapshot used to construct one immutable progression catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressionCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale every localized progression value was reported in.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
    /// Profile identity, freshness witness and explicit permission for this read.
    pub profile: ProgressionProfileInput,
    /// Per-domain support state with the counts the source reports.
    pub domains: Vec<ProgressionDomainCoverage>,
    /// Progression entries.
    pub entries: Vec<ProgressionEntryInput>,
}

/// Explicitly supported read seam for one profile's progression.
///
/// An offline or foreign profile is only readable through this seam, so a caller cannot reach one
/// by naming it to a boundary that only holds the active profile.
pub trait ProgressionReadPort {
    /// Reports which read seam this port actually implements.
    fn read_availability(&self) -> super::ProgressionReadAvailability;

    /// Copies bounded progression records without changing profile or save state.
    fn read_progression(
        &self,
        manifest: &ContentManifest,
    ) -> Result<ProgressionCatalogSnapshot, ProgressionSourceError>;
}

/// Fail-closed host boundary used until an exact-host progression read is authorized and verified.
#[derive(Debug, Default)]
pub struct UnavailableProgressionHost;

impl ProgressionReadPort for UnavailableProgressionHost {
    fn read_availability(&self) -> super::ProgressionReadAvailability {
        super::ProgressionReadAvailability::UnavailableHost
    }

    fn read_progression(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<ProgressionCatalogSnapshot, ProgressionSourceError> {
        Err(ProgressionSourceError::NoActiveSource)
    }
}

/// Producer that binds progression records to one manifest, locale, profile and revision.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProgressionCatalogProducer;

impl ProgressionCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<P: ProgressionReadPort>(
        &self,
        manifest: &ContentManifest,
        port: &P,
        profile: ProgressionProfileQuery,
    ) -> Result<ProgressionCatalog, ProgressionReferenceError> {
        validate_profile_query(&profile)?;
        let snapshot = port.read_progression(manifest).map_err(map_source_error)?;
        validate_fences(manifest, &snapshot)?;
        let binding = ProgressionCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            profile: snapshot.profile.profile,
            baseline: snapshot.profile.baseline,
            producer_version: snapshot.producer_version,
        };
        validate_profile_query_against(&profile, &binding)?;
        let entries = collect_entries(snapshot.entries, manifest, &binding)?;
        validate_capability(&snapshot.domains, &entries)?;
        Ok(ProgressionCatalog::from_parts(
            binding,
            snapshot.domains,
            entries,
        ))
    }
}

/// Refuses a query that could only be answered by reading another profile.
fn validate_profile_query(
    query: &ProgressionProfileQuery,
) -> Result<(), ProgressionReferenceError> {
    match query {
        ProgressionProfileQuery::Active | ProgressionProfileQuery::Named { .. } => Ok(()),
    }
}

fn validate_profile_query_against(
    query: &ProgressionProfileQuery,
    binding: &ProgressionCatalogBinding,
) -> Result<(), ProgressionReferenceError> {
    match query {
        ProgressionProfileQuery::Active => Ok(()),
        ProgressionProfileQuery::Named { user_data_id, kind } => {
            if !matches!(kind, ProgressionProfileKind::Active) {
                return Err(ProgressionReferenceError::ForeignProfileRequiresReadPort);
            }
            if *user_data_id != binding.profile.user_data_id {
                return Err(ProgressionReferenceError::ImplicitProfileSwitch);
            }
            Ok(())
        }
    }
}

fn validate_fences(
    manifest: &ContentManifest,
    snapshot: &ProgressionCatalogSnapshot,
) -> Result<(), ProgressionReferenceError> {
    if snapshot.manifest != manifest.cursor_binding() {
        return Err(ProgressionReferenceError::ManifestMismatch);
    }
    if snapshot.locale != manifest.locale {
        return Err(ProgressionReferenceError::LocaleMismatch);
    }
    if snapshot.producer_version != PROGRESSION_REFERENCE_PRODUCER_VERSION {
        return Err(ProgressionReferenceError::ProducerVersionMismatch);
    }
    if snapshot.entries.len() > PROGRESSION_MAX_ENTRIES {
        return Err(ProgressionReferenceError::InvalidInput("entries"));
    }
    let ProgressionProfilePermit::Permitted { .. } = snapshot.profile.permit else {
        return Err(ProgressionReferenceError::ProfileNotPermitted);
    };
    if snapshot.profile.baseline.user_data_id() != &snapshot.profile.profile.user_data_id {
        return Err(ProgressionReferenceError::ProfileBaselineMismatch);
    }
    match snapshot.profile.profile.kind {
        ProgressionProfileKind::Active => Ok(()),
        ProgressionProfileKind::Offline | ProgressionProfileKind::Foreign => {
            Err(ProgressionReferenceError::ForeignProfileRequiresReadPort)
        }
    }
}

fn collect_entries(
    inputs: Vec<ProgressionEntryInput>,
    manifest: &ContentManifest,
    binding: &ProgressionCatalogBinding,
) -> Result<BTreeMap<String, ProgressionEntry>, ProgressionReferenceError> {
    let mut entries = BTreeMap::new();
    for input in inputs {
        validate_entry(&input, manifest, binding)?;
        let entry_id = input.entry_id.clone();
        if entries
            .insert(
                entry_id.clone(),
                ProgressionEntry::from_input(binding, input),
            )
            .is_some()
        {
            return Err(ProgressionReferenceError::DuplicateEntry(entry_id));
        }
    }
    Ok(entries)
}
