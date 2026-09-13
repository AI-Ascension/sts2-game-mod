// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::{ContentIndex, ContentManifest};

use super::catalog::GlossaryContentReferenceResolution;
use super::coverage::GlossaryCoverage;
use super::error::map_source_error;
use super::model::GlossaryReferenceVisibilityPolicy;
use super::validation::{manifest_definitions, validate_input};
use super::{
    GLOSSARY_MAX_TERM_COUNT, GLOSSARY_PRODUCER_VERSION, GlossaryCatalog, GlossaryCatalogBinding,
    GlossaryCatalogError, GlossarySourceError, GlossaryTerm, GlossaryTermInput,
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
            Vec::new(),
        ))
    }

    /// Produces a glossary and composes every content-index term reference before reporting
    /// coverage. This is the only producer path that can transition coverage from `Unverified`
    /// to `Complete` or `Partial`.
    pub fn produce_with_content_index<S: GlossarySource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
        content_index: &ContentIndex,
    ) -> Result<GlossaryCatalog, GlossaryCatalogError> {
        self.produce(manifest, source)?
            .with_content_index(content_index)
    }
}

impl Default for GlossaryProducer {
    fn default() -> Self {
        Self::new(GlossaryReferenceVisibilityPolicy::PublicOnly)
    }
}
