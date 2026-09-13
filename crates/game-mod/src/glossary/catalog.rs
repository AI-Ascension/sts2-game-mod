// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::ContentDefinitionReference;

use super::model::GlossaryCatalogBinding;
use super::{
    GlossaryCatalogError, GlossaryContentReferenceSurface, GlossaryEvidence, GlossaryEvidenceKind,
    GlossaryQueryScope, GlossaryReferenceVisibilityPolicy, GlossaryTermVisibility,
    GlossaryUnresolvedReason,
};

/// Exact static reference to one glossary term.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GlossaryTermReference {
    /// Catalog witness that owns this term.
    pub catalog: GlossaryCatalogBinding,
    /// Stable term identity.
    pub term_id: String,
}

/// Resolution state for a related-term edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GlossaryRelatedTermResolution {
    /// The target term exists in this catalog.
    Resolved(GlossaryTermReference),
    /// The target is retained as an explicit unavailable edge.
    Unresolved(GlossaryUnresolvedReason),
}

/// One bounded direct edge to another term.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossaryRelatedTerm {
    /// Stable target identity copied from the source.
    pub term_id: String,
    /// Resolution status; no recursive expansion is performed.
    pub resolution: GlossaryRelatedTermResolution,
}

/// Resolution state for a content definition cross-reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GlossaryContentReferenceResolution {
    /// The target exists in the manifest.
    Resolved(ContentDefinitionReference),
    /// The target remains visible as an explicit unavailable record.
    Unresolved(GlossaryUnresolvedReason),
}

/// One bounded content definition cross-reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossaryContentReference {
    /// Family and definition target as supplied by the source.
    pub entity_kind: String,
    /// Namespaced definition ID as supplied by the source.
    pub namespaced_id: String,
    /// Structured definition or rendered-text attachment surface.
    pub surface: GlossaryContentReferenceSurface,
    /// Resolution against the bound content manifest.
    pub resolution: GlossaryContentReferenceResolution,
}

/// A complete glossary term bound to one immutable manifest and locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossaryTerm {
    /// Exact term reference.
    pub reference: GlossaryTermReference,
    /// Localized canonical name.
    pub display_name: String,
    /// Localized aliases.
    pub aliases: Vec<String>,
    /// Localized definition or explicit unavailable status.
    pub definition: super::GlossaryDefinitionText,
    /// Parameter placeholders retained as text, never evaluated.
    pub parameter_placeholders: Vec<String>,
    /// Direct related-term edges.
    pub related_terms: Vec<GlossaryRelatedTerm>,
    /// Stable rule references owned by the rules feature.
    pub rule_references: Vec<String>,
    /// Content definition cross-references.
    pub content_references: Vec<GlossaryContentReference>,
    /// Visibility copied from the source.
    pub visibility: GlossaryTermVisibility,
    /// Evidence/provenance classification.
    pub evidence: GlossaryEvidence,
}

/// Bounded summary returned by list and search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossaryTermSummary {
    /// Exact term reference.
    pub reference: GlossaryTermReference,
    /// Localized canonical name.
    pub display_name: String,
    /// Whether a definition body is available.
    pub definition_available: bool,
    /// Evidence category without source details.
    pub evidence_kind: GlossaryEvidenceKind,
    /// Visibility copied from the source.
    pub visibility: GlossaryTermVisibility,
    /// Number of direct related-term edges.
    pub related_term_count: usize,
    /// Number of unresolved related-term edges.
    pub unresolved_related_term_count: usize,
    /// Number of content cross-references.
    pub content_reference_count: usize,
    /// Number of unresolved content cross-references.
    pub unresolved_content_reference_count: usize,
    /// Number of rule references.
    pub rule_reference_count: usize,
}

/// Full exact term payload.
pub type GlossaryTermDetail = GlossaryTerm;

/// Immutable glossary catalog keyed by stable term ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossaryCatalog {
    pub(super) binding: GlossaryCatalogBinding,
    pub(super) locked_visibility: GlossaryReferenceVisibilityPolicy,
    pub(super) terms: BTreeMap<String, GlossaryTerm>,
    pub(super) coverage: super::GlossaryCoverage,
}

impl GlossaryCatalog {
    pub(super) fn from_parts(
        binding: GlossaryCatalogBinding,
        locked_visibility: GlossaryReferenceVisibilityPolicy,
        terms: BTreeMap<String, GlossaryTerm>,
        coverage: super::GlossaryCoverage,
    ) -> Self {
        Self {
            binding,
            locked_visibility,
            terms,
            coverage,
        }
    }

    /// Returns the manifest/locale/producer identity fence.
    #[must_use]
    pub fn binding(&self) -> &GlossaryCatalogBinding {
        &self.binding
    }

    /// Returns the exact locale used by localized term values.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.binding.locale
    }

    /// Returns source coverage, including unresolved bounded references.
    #[must_use]
    pub fn coverage(&self) -> &super::GlossaryCoverage {
        &self.coverage
    }

    /// Returns a cursor-owning reader with independent single-use continuations.
    #[must_use]
    pub fn reader(&self) -> super::GlossaryReader {
        super::GlossaryReader::new(self.clone())
    }

    /// Performs an exact term lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &GlossaryTermReference,
        scope: GlossaryQueryScope,
    ) -> Result<GlossaryTermDetail, GlossaryCatalogError> {
        self.reader().get(reference, scope)
    }

    pub(super) fn term(&self, term_id: &str) -> Option<&GlossaryTerm> {
        self.terms.get(term_id)
    }

    pub(super) fn visible(&self, term: &GlossaryTerm, scope: GlossaryQueryScope) -> bool {
        match term.visibility {
            GlossaryTermVisibility::Public => true,
            GlossaryTermVisibility::ReferenceOnly => {
                matches!(
                    (scope, self.locked_visibility),
                    (
                        GlossaryQueryScope::Reference,
                        GlossaryReferenceVisibilityPolicy::AllowReferenceTerms
                    )
                )
            }
            GlossaryTermVisibility::Hidden | GlossaryTermVisibility::Unknown => false,
        }
    }
}
