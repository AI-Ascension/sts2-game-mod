// SPDX-License-Identifier: MIT

//! The owner-local snapshot, source boundary and producer that bind results to one manifest.

use std::collections::{BTreeMap, BTreeSet};

use crate::ContentManifest;

use super::error::map_source_error;
use super::validation::{claim_run, validate_history, validate_result};
use super::{
    RUN_RESULT_MAX_RESULTS, RUN_RESULT_MAX_SUMMARIES, RUN_RESULT_REFERENCE_PRODUCER_VERSION,
    ResultFinalization, RunResultCatalogBinding, RunResultError, RunResultFamilyCoverage,
    RunResultFamilyState, RunResultInput, RunResultRecord, RunResultSourceError, RunSummaryInput,
    RunSummaryRecord, catalog_reader::RunResultCatalog,
};

/// Bounded source snapshot used to construct one immutable run-result catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResultCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale every localized result value was reported in.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
    /// Explicit support state for the result family with the counts the source reports.
    pub family: RunResultFamilyCoverage,
    /// Completed-run result records.
    pub results: Vec<RunResultInput>,
    /// Prior run-summary records.
    pub summaries: Vec<RunSummaryInput>,
}

/// Owner-local source boundary for copied completed-run records.
pub trait RunResultCatalogSource {
    /// Copies bounded completed-run records without reading a live run or touching a save.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<RunResultCatalogSnapshot, RunResultSourceError>;
}

/// Producer that binds completed-run records to one immutable content manifest and locale.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RunResultCatalogProducer;

impl RunResultCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: RunResultCatalogSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<RunResultCatalog, RunResultError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        validate_fences(manifest, &snapshot)?;
        let binding = RunResultCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            producer_version: snapshot.producer_version,
        };
        let family = snapshot.family;
        if family.state != RunResultFamilyState::Handled {
            if let Some(first) = snapshot.results.first() {
                return Err(RunResultError::UnknownResult(first.result_id.clone()));
            }
            if let Some(first) = snapshot.summaries.first() {
                return Err(RunResultError::UnknownResult(first.summary_id.clone()));
            }
            return Ok(RunResultCatalog::from_parts(
                binding,
                family,
                BTreeMap::new(),
                BTreeMap::new(),
            ));
        }
        let mut owners = BTreeMap::new();
        let results = collect_results(snapshot.results, manifest, &binding, &mut owners)?;
        let summaries = collect_summaries(snapshot.summaries, &binding, &results, &mut owners)?;
        Ok(RunResultCatalog::from_parts(
            binding, family, results, summaries,
        ))
    }
}

fn validate_fences(
    manifest: &ContentManifest,
    snapshot: &RunResultCatalogSnapshot,
) -> Result<(), RunResultError> {
    if snapshot.manifest != manifest.cursor_binding() {
        return Err(RunResultError::ManifestMismatch);
    }
    if snapshot.locale != manifest.locale {
        return Err(RunResultError::LocaleMismatch);
    }
    if snapshot.producer_version != RUN_RESULT_REFERENCE_PRODUCER_VERSION {
        return Err(RunResultError::ProducerVersionMismatch);
    }
    if snapshot.results.len() > RUN_RESULT_MAX_RESULTS
        || snapshot.summaries.len() > RUN_RESULT_MAX_SUMMARIES
    {
        return Err(RunResultError::InvalidInput("records"));
    }
    if snapshot.family.result_count != snapshot.results.len()
        || snapshot.family.summary_count != snapshot.summaries.len()
    {
        return Err(RunResultError::FamilyCountMismatch);
    }
    Ok(())
}

fn collect_results(
    inputs: Vec<RunResultInput>,
    manifest: &ContentManifest,
    binding: &RunResultCatalogBinding,
    owners: &mut BTreeMap<String, String>,
) -> Result<BTreeMap<String, RunResultRecord>, RunResultError> {
    let mut records = BTreeMap::new();
    let mut runs = BTreeSet::new();
    for input in inputs {
        validate_result(&input, manifest)?;
        claim_run(owners, &input.run_id, &input.profile_id)?;
        let result_id = input.result_id.clone();
        let run_id = input.run_id.clone();
        if records
            .insert(
                result_id.clone(),
                RunResultRecord::from_input(binding, input),
            )
            .is_some()
        {
            return Err(RunResultError::DuplicateResult(result_id));
        }
        if !runs.insert(run_id.clone()) {
            return Err(RunResultError::DuplicateRun(run_id));
        }
    }
    Ok(records)
}

fn collect_summaries(
    inputs: Vec<RunSummaryInput>,
    binding: &RunResultCatalogBinding,
    results: &BTreeMap<String, RunResultRecord>,
    owners: &mut BTreeMap<String, String>,
) -> Result<BTreeMap<String, RunSummaryRecord>, RunResultError> {
    let mut records = BTreeMap::new();
    let mut runs = BTreeSet::new();
    for input in inputs {
        validate_history(&input)?;
        validate_detail(&input, results)?;
        claim_run(owners, &input.run_id, &input.profile_id)?;
        let summary_id = input.summary_id.clone();
        let run_id = input.run_id.clone();
        if records
            .insert(
                summary_id.clone(),
                RunSummaryRecord::from_input(binding, input),
            )
            .is_some()
        {
            return Err(RunResultError::DuplicateSummary(summary_id));
        }
        if !runs.insert(run_id.clone()) {
            return Err(RunResultError::DuplicateRun(run_id));
        }
    }
    Ok(records)
}

/// Refuses a summary whose published detail the catalog cannot read as a persisted fact.
fn validate_detail(
    summary: &RunSummaryInput,
    results: &BTreeMap<String, RunResultRecord>,
) -> Result<(), RunResultError> {
    let Some(detail_id) = summary.detail_result_id.value() else {
        return Ok(());
    };
    let record = results
        .get(detail_id)
        .ok_or_else(|| RunResultError::MissingResultDetail(detail_id.clone()))?;
    if !matches!(record.result.finalization, ResultFinalization::Finalized) {
        return Err(RunResultError::PendingDetailPublished);
    }
    Ok(())
}
