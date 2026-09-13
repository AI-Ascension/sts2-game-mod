// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::ContentManifest;

use super::catalog::GlossaryContentReferenceResolution;
use super::error::map_source_error;
use super::model::{GlossaryReferenceVisibilityPolicy, GlossaryUnresolvedReason};
use super::validation::{manifest_definitions, validate_input};
use super::{
    GLOSSARY_MAX_COVERAGE_REFERENCES, GLOSSARY_MAX_TERM_COUNT, GLOSSARY_PRODUCER_VERSION,
    GlossaryCatalog, GlossaryCatalogBinding, GlossaryCatalogError, GlossarySourceError,
    GlossaryTerm, GlossaryTermInput,
};

/// One coherent source snapshot of glossary terms for one manifest and locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossarySnapshot {
    /// Content-manifest invalidation witness copied with source values.
    pub manifest: crate::ContentCursorBinding,
    /// Locale used by every localized term field.
    pub locale: String,
    /// Producer identity expected by the owner-local consumer.
    pub producer_version: String,
    /// Bounded source-owned term records.
    pub terms: Vec<GlossaryTermInput>,
}

/// Owner-local source boundary for copied glossary records.
pub trait GlossarySource {
    /// Copies one coherent glossary snapshot without constructing playable objects or mutating
    /// profile/run state.
    fn read_glossary(
        &self,
        manifest: &ContentManifest,
    ) -> Result<GlossarySnapshot, GlossarySourceError>;
}

/// Completeness of cross-reference coverage.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GlossaryCoverageStatus {
    /// Every bounded reference resolves against this glossary and manifest.
    Complete,
    /// One or more references remain explicit unavailable records.
    Partial,
}

/// One unresolved term edge retained in bounded coverage diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossaryUnresolvedTermReference {
    /// Source term carrying the edge.
    pub term_id: String,
    /// Missing target term ID.
    pub target_term_id: String,
    /// Explicit reason retained by the producer.
    pub reason: GlossaryUnresolvedReason,
}

/// One unresolved content definition edge retained in bounded coverage diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossaryUnresolvedContentReference {
    /// Source term carrying the edge.
    pub term_id: String,
    /// Target family.
    pub entity_kind: String,
    /// Target namespaced definition ID.
    pub namespaced_id: String,
    /// Explicit reason retained by the producer.
    pub reason: GlossaryUnresolvedReason,
}

/// Cross-reference coverage for one immutable glossary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossaryCoverage {
    /// Complete or partial reference coverage.
    pub status: GlossaryCoverageStatus,
    /// Number of terms retained.
    pub term_count: usize,
    /// Number of related-term edges.
    pub related_reference_count: usize,
    /// Number of content definition edges.
    pub content_reference_count: usize,
    /// Total unresolved related-term edges before diagnostic truncation.
    pub unresolved_related_count: usize,
    /// Total unresolved content edges before diagnostic truncation.
    pub unresolved_content_reference_count: usize,
    /// Unresolved related-term edges, bounded and deterministic.
    pub unresolved_related_terms: Vec<GlossaryUnresolvedTermReference>,
    /// Unresolved content edges, bounded and deterministic.
    pub unresolved_content_references: Vec<GlossaryUnresolvedContentReference>,
}

impl GlossaryCoverage {
    fn new(term_count: usize) -> Self {
        Self {
            status: GlossaryCoverageStatus::Complete,
            term_count,
            related_reference_count: 0,
            content_reference_count: 0,
            unresolved_related_count: 0,
            unresolved_content_reference_count: 0,
            unresolved_related_terms: Vec::new(),
            unresolved_content_references: Vec::new(),
        }
    }

    fn record_related(
        &mut self,
        term_id: &str,
        target_term_id: &str,
        reason: GlossaryUnresolvedReason,
    ) {
        self.status = GlossaryCoverageStatus::Partial;
        self.unresolved_related_count = self.unresolved_related_count.saturating_add(1);
        if self.unresolved_related_terms.len() < GLOSSARY_MAX_COVERAGE_REFERENCES {
            self.unresolved_related_terms
                .push(GlossaryUnresolvedTermReference {
                    term_id: term_id.to_owned(),
                    target_term_id: target_term_id.to_owned(),
                    reason,
                });
        }
    }

    fn record_content(
        &mut self,
        term_id: &str,
        entity_kind: &str,
        namespaced_id: &str,
        reason: GlossaryUnresolvedReason,
    ) {
        self.status = GlossaryCoverageStatus::Partial;
        self.unresolved_content_reference_count =
            self.unresolved_content_reference_count.saturating_add(1);
        if self.unresolved_content_references.len() < GLOSSARY_MAX_COVERAGE_REFERENCES {
            self.unresolved_content_references
                .push(GlossaryUnresolvedContentReference {
                    term_id: term_id.to_owned(),
                    entity_kind: entity_kind.to_owned(),
                    namespaced_id: namespaced_id.to_owned(),
                    reason,
                });
        }
    }
}

/// Producer that binds terms and cross-references to one content manifest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlossaryProducer {
    locked_visibility: GlossaryReferenceVisibilityPolicy,
}

impl GlossaryProducer {
    /// Creates a producer with explicit reference-only visibility policy.
    #[must_use]
    pub const fn new(locked_visibility: GlossaryReferenceVisibilityPolicy) -> Self {
        Self { locked_visibility }
    }

    /// Produces an immutable glossary without constructing playable objects or mutating state.
    pub fn produce<S: GlossarySource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<GlossaryCatalog, GlossaryCatalogError> {
        let snapshot = source.read_glossary(manifest).map_err(map_source_error)?;
        let binding = manifest.cursor_binding();
        if snapshot.manifest != binding {
            return Err(GlossaryCatalogError::ManifestMismatch);
        }
        if snapshot.locale != manifest.locale {
            return Err(GlossaryCatalogError::LocaleMismatch);
        }
        if snapshot.producer_version != GLOSSARY_PRODUCER_VERSION {
            return Err(GlossaryCatalogError::ProducerVersionMismatch);
        }
        if snapshot.terms.len() > GLOSSARY_MAX_TERM_COUNT {
            return Err(GlossaryCatalogError::TermCountTooLarge {
                limit: GLOSSARY_MAX_TERM_COUNT,
                actual: snapshot.terms.len(),
            });
        }

        let mut inputs = BTreeMap::new();
        for input in snapshot.terms {
            validate_input(&input)?;
            let term_id = input.term_id.clone();
            if inputs.insert(term_id.clone(), input).is_some() {
                return Err(GlossaryCatalogError::DuplicateTerm(term_id));
            }
        }
        let catalog_binding = GlossaryCatalogBinding {
            manifest: binding.clone(),
            locale: snapshot.locale,
            producer_version: GLOSSARY_PRODUCER_VERSION.to_owned(),
        };
        let manifest_definitions = manifest_definitions(manifest);
        let mut coverage = GlossaryCoverage::new(inputs.len());
        let mut terms = BTreeMap::new();
        for (term_id, input) in &inputs {
            let (term, unresolved_related, unresolved_content) = GlossaryTerm::from_input(
                &catalog_binding,
                input.clone(),
                &inputs,
                &manifest_definitions,
                self.locked_visibility,
            );
            coverage.related_reference_count = coverage
                .related_reference_count
                .saturating_add(term.related_terms.len());
            coverage.content_reference_count = coverage
                .content_reference_count
                .saturating_add(term.content_references.len());
            if unresolved_related != 0 {
                for related in &term.related_terms {
                    if let super::GlossaryRelatedTermResolution::Unresolved(reason) =
                        related.resolution
                    {
                        coverage.record_related(term_id, &related.term_id, reason);
                    }
                }
            }
            if unresolved_content != 0 {
                for content_reference in &term.content_references {
                    if let GlossaryContentReferenceResolution::Unresolved(reason) =
                        content_reference.resolution
                    {
                        coverage.record_content(
                            term_id,
                            &content_reference.entity_kind,
                            &content_reference.namespaced_id,
                            reason,
                        );
                    }
                }
            }
            terms.insert(term_id.clone(), term);
        }
        coverage.unresolved_related_terms.sort_by(|left, right| {
            (&left.term_id, &left.target_term_id, left.reason).cmp(&(
                &right.term_id,
                &right.target_term_id,
                right.reason,
            ))
        });
        coverage
            .unresolved_content_references
            .sort_by(|left, right| {
                (
                    &left.term_id,
                    &left.entity_kind,
                    &left.namespaced_id,
                    left.reason,
                )
                    .cmp(&(
                        &right.term_id,
                        &right.entity_kind,
                        &right.namespaced_id,
                        right.reason,
                    ))
            });
        Ok(GlossaryCatalog::from_parts(
            catalog_binding,
            self.locked_visibility,
            terms,
            coverage,
        ))
    }
}

impl Default for GlossaryProducer {
    fn default() -> Self {
        Self::new(GlossaryReferenceVisibilityPolicy::PublicOnly)
    }
}
