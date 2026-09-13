// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::{ContentCursorBinding, ContentDefinitionReference};

use super::catalog::{
    GlossaryContentReference, GlossaryContentReferenceResolution, GlossaryRelatedTerm,
    GlossaryRelatedTermResolution, GlossaryTerm, GlossaryTermReference, GlossaryTermSummary,
};
use super::model::{
    GlossaryCatalogBinding, GlossaryDefinitionText, GlossaryEvidence,
    GlossaryReferenceVisibilityPolicy, GlossaryTermInput, GlossaryTermVisibility,
    GlossaryUnresolvedReason,
};
use super::{GLOSSARY_MAX_DETAIL_BYTES, GlossaryCatalogError};

impl GlossaryTerm {
    pub(super) fn from_input(
        binding: &GlossaryCatalogBinding,
        input: GlossaryTermInput,
        terms: &BTreeMap<String, GlossaryTermInput>,
        manifest_definitions: &BTreeMap<(String, String), ContentCursorBinding>,
        locked_visibility: GlossaryReferenceVisibilityPolicy,
    ) -> (Self, usize, usize) {
        let related_terms = input
            .related_terms
            .iter()
            .map(|term_id| GlossaryRelatedTerm {
                term_id: term_id.clone(),
                resolution: match terms.get(term_id).map(|term| term.visibility) {
                    None => {
                        GlossaryRelatedTermResolution::Unresolved(GlossaryUnresolvedReason::Missing)
                    }
                    Some(GlossaryTermVisibility::Public) => {
                        GlossaryRelatedTermResolution::Resolved(GlossaryTermReference {
                            catalog: binding.clone(),
                            term_id: term_id.clone(),
                        })
                    }
                    Some(GlossaryTermVisibility::ReferenceOnly)
                        if matches!(
                            locked_visibility,
                            GlossaryReferenceVisibilityPolicy::AllowReferenceTerms
                        ) =>
                    {
                        GlossaryRelatedTermResolution::Resolved(GlossaryTermReference {
                            catalog: binding.clone(),
                            term_id: term_id.clone(),
                        })
                    }
                    Some(GlossaryTermVisibility::ReferenceOnly)
                    | Some(GlossaryTermVisibility::Hidden)
                    | Some(GlossaryTermVisibility::Unknown) => {
                        GlossaryRelatedTermResolution::Unresolved(
                            GlossaryUnresolvedReason::ExcludedByScope,
                        )
                    }
                },
            })
            .collect::<Vec<_>>();
        let unresolved_related = related_terms
            .iter()
            .filter(|reference| {
                matches!(
                    reference.resolution,
                    GlossaryRelatedTermResolution::Unresolved(_)
                )
            })
            .count();

        let content_references = input
            .content_references
            .iter()
            .map(|reference| {
                let key = (
                    reference.entity_kind.clone(),
                    reference.namespaced_id.clone(),
                );
                let resolution = manifest_definitions
                    .get(&key)
                    .map(|manifest| {
                        GlossaryContentReferenceResolution::Resolved(ContentDefinitionReference {
                            manifest: manifest.clone(),
                            entity_kind: reference.entity_kind.clone(),
                            namespaced_id: reference.namespaced_id.clone(),
                        })
                    })
                    .unwrap_or(GlossaryContentReferenceResolution::Unresolved(
                        GlossaryUnresolvedReason::Missing,
                    ));
                GlossaryContentReference {
                    entity_kind: reference.entity_kind.clone(),
                    namespaced_id: reference.namespaced_id.clone(),
                    surface: reference.surface,
                    resolution,
                }
            })
            .collect::<Vec<_>>();
        let unresolved_content = content_references
            .iter()
            .filter(|reference| {
                matches!(
                    reference.resolution,
                    GlossaryContentReferenceResolution::Unresolved(_)
                )
            })
            .count();
        (
            Self {
                reference: GlossaryTermReference {
                    catalog: binding.clone(),
                    term_id: input.term_id,
                },
                display_name: input.display_name,
                aliases: input.aliases,
                definition: input.definition,
                parameter_placeholders: input.parameter_placeholders,
                related_terms,
                rule_references: input.rule_references,
                content_references,
                visibility: input.visibility,
                evidence: input.evidence,
            },
            unresolved_related,
            unresolved_content,
        )
    }
}

impl GlossaryTermSummary {
    pub(super) fn from_term(term: &GlossaryTerm) -> Self {
        Self {
            reference: term.reference.clone(),
            display_name: term.display_name.clone(),
            definition_available: matches!(term.definition, GlossaryDefinitionText::Available(_)),
            evidence_kind: term.evidence.kind(),
            visibility: term.visibility,
            related_term_count: term.related_terms.len(),
            unresolved_related_term_count: term
                .related_terms
                .iter()
                .filter(|reference| {
                    matches!(
                        reference.resolution,
                        GlossaryRelatedTermResolution::Unresolved(_)
                    )
                })
                .count(),
            content_reference_count: term.content_references.len(),
            unresolved_content_reference_count: term
                .content_references
                .iter()
                .filter(|reference| {
                    matches!(
                        reference.resolution,
                        GlossaryContentReferenceResolution::Unresolved(_)
                    )
                })
                .count(),
            rule_reference_count: term.rule_references.len(),
        }
    }
}

pub(super) fn detail_bytes(term: &GlossaryTerm) -> Result<usize, GlossaryCatalogError> {
    let mut actual = 0usize;
    let mut add = |value: &str| -> Result<(), GlossaryCatalogError> {
        actual = actual
            .checked_add(value.len())
            .ok_or(GlossaryCatalogError::DetailTooLarge {
                limit: GLOSSARY_MAX_DETAIL_BYTES,
                actual: usize::MAX,
            })?;
        if actual > GLOSSARY_MAX_DETAIL_BYTES {
            return Err(GlossaryCatalogError::DetailTooLarge {
                limit: GLOSSARY_MAX_DETAIL_BYTES,
                actual,
            });
        }
        Ok(())
    };
    add(&term.reference.catalog.manifest.adapter_compatibility)?;
    add(&term.reference.catalog.manifest.content_set_revision)?;
    add(&term.reference.catalog.manifest.localized_text_revision)?;
    add(&term.reference.catalog.manifest.inventory_revision)?;
    add(&term.reference.catalog.locale)?;
    add(&term.reference.catalog.producer_version)?;
    add(&term.reference.term_id)?;
    add(&term.display_name)?;
    for alias in &term.aliases {
        add(alias)?;
    }
    if let GlossaryDefinitionText::Available(definition) = &term.definition {
        add(definition)?;
    }
    for placeholder in &term.parameter_placeholders {
        add(placeholder)?;
    }
    for related in &term.related_terms {
        add(&related.term_id)?;
        if let GlossaryRelatedTermResolution::Unresolved(reason) = related.resolution {
            add(&format!("{reason:?}"))?;
        }
    }
    for rule_reference in &term.rule_references {
        add(rule_reference)?;
    }
    for content_reference in &term.content_references {
        add(&content_reference.entity_kind)?;
        add(&content_reference.namespaced_id)?;
        if let GlossaryContentReferenceResolution::Unresolved(reason) = content_reference.resolution
        {
            add(&format!("{reason:?}"))?;
        }
    }
    match &term.evidence {
        GlossaryEvidence::NativeTooltip { source_id } => add(source_id)?,
        GlossaryEvidence::OwnerDocumentation {
            document_id,
            evidence_tag,
        } => {
            add(document_id)?;
            add(evidence_tag)?;
        }
        GlossaryEvidence::Unavailable(reason) => add(&format!("{reason:?}"))?,
    }
    Ok(actual)
}
