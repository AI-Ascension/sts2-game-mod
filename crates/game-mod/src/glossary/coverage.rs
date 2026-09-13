// SPDX-License-Identifier: MIT

use crate::ContentDefinitionReference;

use super::{GLOSSARY_MAX_COVERAGE_REFERENCES, GlossaryUnresolvedReason};

/// Completeness of cross-reference coverage.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GlossaryCoverageStatus {
    /// The glossary snapshot was validated, but content-index term references were not composed.
    Unverified,
    /// Every bounded glossary and composed content-index reference resolves.
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

/// One unresolved content-definition-to-term edge retained in bounded coverage diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossaryUnresolvedDefinitionReference {
    /// Target family.
    pub entity_kind: String,
    /// Target namespaced definition ID.
    pub namespaced_id: String,
    /// Glossary term ID supplied by the content index.
    pub term_id: String,
    /// Explicit reason retained by the composition pass.
    pub reason: GlossaryUnresolvedReason,
}

/// Cross-reference coverage for one immutable glossary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossaryCoverage {
    /// Unverified until content-index term references are composed; then complete or partial.
    pub status: GlossaryCoverageStatus,
    /// Number of terms retained.
    pub term_count: usize,
    /// Number of related-term edges.
    pub related_reference_count: usize,
    /// Number of content definition edges.
    pub content_reference_count: usize,
    /// Number of content-definition-to-term edges composed from the content index.
    pub definition_reference_count: usize,
    /// Total unresolved related-term edges before diagnostic truncation.
    pub unresolved_related_count: usize,
    /// Total unresolved content edges before diagnostic truncation.
    pub unresolved_content_reference_count: usize,
    /// Total unresolved content-definition-to-term edges before diagnostic truncation.
    pub unresolved_definition_reference_count: usize,
    /// Unresolved related-term edges, bounded and deterministic.
    pub unresolved_related_terms: Vec<GlossaryUnresolvedTermReference>,
    /// Unresolved content edges, bounded and deterministic.
    pub unresolved_content_references: Vec<GlossaryUnresolvedContentReference>,
    /// Unresolved content-definition-to-term edges, bounded and deterministic.
    pub unresolved_definition_references: Vec<GlossaryUnresolvedDefinitionReference>,
}

impl GlossaryCoverage {
    pub(super) fn new(term_count: usize) -> Self {
        Self {
            status: GlossaryCoverageStatus::Unverified,
            term_count,
            related_reference_count: 0,
            content_reference_count: 0,
            definition_reference_count: 0,
            unresolved_related_count: 0,
            unresolved_content_reference_count: 0,
            unresolved_definition_reference_count: 0,
            unresolved_related_terms: Vec::new(),
            unresolved_content_references: Vec::new(),
            unresolved_definition_references: Vec::new(),
        }
    }

    pub(super) fn record_related(
        &mut self,
        term_id: &str,
        target_term_id: &str,
        reason: GlossaryUnresolvedReason,
    ) {
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

    pub(super) fn record_content(
        &mut self,
        term_id: &str,
        entity_kind: &str,
        namespaced_id: &str,
        reason: GlossaryUnresolvedReason,
    ) {
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

    pub(super) fn record_definition(
        &mut self,
        definition: &ContentDefinitionReference,
        term_id: &str,
        reason: GlossaryUnresolvedReason,
    ) {
        self.unresolved_definition_reference_count =
            self.unresolved_definition_reference_count.saturating_add(1);
        if self.unresolved_definition_references.len() < GLOSSARY_MAX_COVERAGE_REFERENCES {
            self.unresolved_definition_references
                .push(GlossaryUnresolvedDefinitionReference {
                    entity_kind: definition.entity_kind.clone(),
                    namespaced_id: definition.namespaced_id.clone(),
                    term_id: term_id.to_owned(),
                    reason,
                });
        }
    }

    pub(super) fn finalize(&mut self) {
        self.status = if self.unresolved_related_count != 0
            || self.unresolved_content_reference_count != 0
            || self.unresolved_definition_reference_count != 0
        {
            GlossaryCoverageStatus::Partial
        } else {
            GlossaryCoverageStatus::Complete
        };
    }
}
