// SPDX-License-Identifier: MIT

use crate::ContentCursorBinding;

use super::{GLOSSARY_MAX_IDENTITY_BYTES, GLOSSARY_MAX_TEXT_BYTES};

/// Static catalog identity for one manifest, locale, and producer.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GlossaryCatalogBinding {
    /// Content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used by every localized term field.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

/// Visibility of one term on owner-controlled read surfaces.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GlossaryTermVisibility {
    /// The term can be listed and retrieved in public scope.
    Public,
    /// The term can be used as a reference but is excluded from public scope.
    ReferenceOnly,
    /// The source knows about the term but must not expose its record.
    Hidden,
    /// The source could not classify visibility.
    Unknown,
}

/// Availability of a definition text or independently authored evidence.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GlossaryUnresolvedReason {
    /// The referenced term or content definition is absent from the inventory.
    Missing,
    /// The target exists but its visibility does not permit this reference.
    ExcludedByScope,
    /// The source has not implemented an extractor for the target.
    Unsupported,
    /// The source declined to disclose the target.
    Denied,
    /// Extraction failed without a safe value.
    Failed,
    /// The source cannot classify the target.
    Unknown,
}

/// A localized definition body or an explicit non-value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GlossaryDefinitionText {
    /// Text copied for the snapshot locale.
    Available(String),
    /// Text was not available; no inferred explanation is substituted.
    Unavailable(GlossaryUnresolvedReason),
}

/// Provenance class for the definition and evidence supplied by a source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GlossaryEvidence {
    /// A term was copied from an owner-observed tooltip/keyword record.
    NativeTooltip {
        /// Stable owner source identity.
        source_id: String,
    },
    /// A term was independently authored by this project.
    OwnerDocumentation {
        /// Stable document identity.
        document_id: String,
        /// Explicit evidence tag, such as `source-derived` or `unverified`.
        evidence_tag: String,
    },
    /// The source supplied no usable definition evidence.
    Unavailable(GlossaryUnresolvedReason),
}

/// Evidence category suitable for summaries without exposing source details.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GlossaryEvidenceKind {
    /// Source-observed tooltip/keyword record.
    NativeTooltip,
    /// Independently authored owner documentation.
    OwnerDocumentation,
    /// No evidence-backed definition is available.
    Unavailable,
}

impl GlossaryEvidence {
    /// Returns the coarse evidence category.
    #[must_use]
    pub const fn kind(&self) -> GlossaryEvidenceKind {
        match self {
            Self::NativeTooltip { .. } => GlossaryEvidenceKind::NativeTooltip,
            Self::OwnerDocumentation { .. } => GlossaryEvidenceKind::OwnerDocumentation,
            Self::Unavailable(_) => GlossaryEvidenceKind::Unavailable,
        }
    }
}

/// Surface on which a content definition references a glossary term.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GlossaryContentReferenceSurface {
    /// The term is attached to the structured content definition.
    Definition,
    /// The term occurs in the locale-bound rendered definition text.
    RenderedText,
}

/// Source-owned content reference before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossaryContentReferenceInput {
    /// Manifest family identity.
    pub entity_kind: String,
    /// Manifest namespaced definition ID.
    pub namespaced_id: String,
    /// Definition or rendered-text attachment surface.
    pub surface: GlossaryContentReferenceSurface,
}

/// One term record copied from an owner source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossaryTermInput {
    /// Stable owner-defined term identity.
    pub term_id: String,
    /// Localized canonical name.
    pub display_name: String,
    /// Localized aliases; aliases never merge distinct term IDs.
    pub aliases: Vec<String>,
    /// Localized definition text or an explicit unavailable state.
    pub definition: GlossaryDefinitionText,
    /// Localized parameter placeholders such as `{amount}`.
    pub parameter_placeholders: Vec<String>,
    /// Direct related-term IDs. Edges are not recursively expanded.
    pub related_terms: Vec<String>,
    /// Stable references owned by the rules feature.
    pub rule_references: Vec<String>,
    /// Content definitions and surfaces that use this term.
    pub content_references: Vec<GlossaryContentReferenceInput>,
    /// Visibility policy copied from the source.
    pub visibility: GlossaryTermVisibility,
    /// Evidence status for the definition.
    pub evidence: GlossaryEvidence,
}

impl GlossaryTermInput {
    /// Creates a public term with no optional references.
    #[must_use]
    pub fn new(term_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            term_id: term_id.into(),
            display_name: display_name.into(),
            aliases: Vec::new(),
            definition: GlossaryDefinitionText::Unavailable(GlossaryUnresolvedReason::Unknown),
            parameter_placeholders: Vec::new(),
            related_terms: Vec::new(),
            rule_references: Vec::new(),
            content_references: Vec::new(),
            visibility: GlossaryTermVisibility::Public,
            evidence: GlossaryEvidence::Unavailable(GlossaryUnresolvedReason::Unknown),
        }
    }
}

/// Query visibility requested by a caller.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GlossaryQueryScope {
    /// Only public terms are visible.
    Public,
    /// Public and reference-only terms are visible when policy allows them.
    Reference,
}

/// Producer policy controlling reference-only term exposure.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GlossaryReferenceVisibilityPolicy {
    /// Reference-only terms remain excluded in every scope.
    PublicOnly,
    /// Reference-only terms may be returned in reference scope.
    AllowReferenceTerms,
}

pub(super) fn validate_identity(
    value: &str,
    field: &'static str,
) -> Result<(), super::GlossaryCatalogError> {
    if value.is_empty()
        || value.len() > GLOSSARY_MAX_IDENTITY_BYTES
        || value.chars().any(char::is_control)
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric()
                && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-' | b'#' | b'{' | b'}')
        })
    {
        return Err(super::GlossaryCatalogError::InvalidIdentity(field));
    }
    Ok(())
}

pub(super) fn validate_text(value: &str) -> Result<(), super::GlossaryCatalogError> {
    if value.len() > GLOSSARY_MAX_TEXT_BYTES || value.chars().any(char::is_control) {
        return Err(super::GlossaryCatalogError::InvalidText);
    }
    Ok(())
}

pub(super) fn validate_non_empty_text(value: &str) -> Result<(), super::GlossaryCatalogError> {
    validate_text(value)?;
    if value.is_empty() {
        return Err(super::GlossaryCatalogError::InvalidText);
    }
    Ok(())
}
