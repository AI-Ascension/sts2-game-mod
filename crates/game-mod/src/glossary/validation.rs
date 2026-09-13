// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use crate::ContentManifest;

use super::model::{validate_identity, validate_non_empty_text, validate_text};
use super::{
    GLOSSARY_MAX_ALIAS_COUNT, GLOSSARY_MAX_CONTENT_REFERENCE_COUNT, GLOSSARY_MAX_PARAMETER_COUNT,
    GLOSSARY_MAX_RELATED_TERM_COUNT, GLOSSARY_MAX_RULE_REFERENCE_COUNT, GlossaryCatalogError,
    GlossaryDefinitionText, GlossaryEvidence, GlossaryTermInput, GlossaryTermVisibility,
};

pub(super) fn validate_input(input: &GlossaryTermInput) -> Result<(), GlossaryCatalogError> {
    validate_identity(&input.term_id, "term_id")?;
    validate_non_empty_text(&input.display_name)?;
    if input.aliases.len() > GLOSSARY_MAX_ALIAS_COUNT {
        return Err(GlossaryCatalogError::CollectionTooLarge {
            field: "aliases",
            limit: GLOSSARY_MAX_ALIAS_COUNT,
            actual: input.aliases.len(),
        });
    }
    let mut aliases = BTreeSet::new();
    for alias in &input.aliases {
        validate_non_empty_text(alias)?;
        if !aliases.insert(alias) {
            return Err(GlossaryCatalogError::DuplicateAlias);
        }
    }
    if let GlossaryDefinitionText::Available(definition) = &input.definition {
        validate_text(definition)?;
    }
    if input.parameter_placeholders.len() > GLOSSARY_MAX_PARAMETER_COUNT {
        return Err(GlossaryCatalogError::CollectionTooLarge {
            field: "parameter_placeholders",
            limit: GLOSSARY_MAX_PARAMETER_COUNT,
            actual: input.parameter_placeholders.len(),
        });
    }
    let mut placeholders = BTreeSet::new();
    for placeholder in &input.parameter_placeholders {
        validate_non_empty_text(placeholder)?;
        if !placeholders.insert(placeholder) {
            return Err(GlossaryCatalogError::DuplicateParameter);
        }
    }
    if input.related_terms.len() > GLOSSARY_MAX_RELATED_TERM_COUNT {
        return Err(GlossaryCatalogError::CollectionTooLarge {
            field: "related_terms",
            limit: GLOSSARY_MAX_RELATED_TERM_COUNT,
            actual: input.related_terms.len(),
        });
    }
    let mut related_terms = BTreeSet::new();
    for term_id in &input.related_terms {
        validate_identity(term_id, "related_term")?;
        if !related_terms.insert(term_id) {
            return Err(GlossaryCatalogError::DuplicateRelatedTerm);
        }
    }
    if input.rule_references.len() > GLOSSARY_MAX_RULE_REFERENCE_COUNT {
        return Err(GlossaryCatalogError::CollectionTooLarge {
            field: "rule_references",
            limit: GLOSSARY_MAX_RULE_REFERENCE_COUNT,
            actual: input.rule_references.len(),
        });
    }
    let mut rules = BTreeSet::new();
    for rule_reference in &input.rule_references {
        validate_identity(rule_reference, "rule_reference")?;
        if !rules.insert(rule_reference) {
            return Err(GlossaryCatalogError::DuplicateRuleReference);
        }
    }
    if input.content_references.len() > GLOSSARY_MAX_CONTENT_REFERENCE_COUNT {
        return Err(GlossaryCatalogError::CollectionTooLarge {
            field: "content_references",
            limit: GLOSSARY_MAX_CONTENT_REFERENCE_COUNT,
            actual: input.content_references.len(),
        });
    }
    let mut content_references = BTreeSet::new();
    for reference in &input.content_references {
        validate_identity(&reference.entity_kind, "entity_kind")?;
        validate_identity(&reference.namespaced_id, "namespaced_id")?;
        let key = (
            reference.entity_kind.as_str(),
            reference.namespaced_id.as_str(),
            reference.surface,
        );
        if !content_references.insert(key) {
            return Err(GlossaryCatalogError::DuplicateContentReference);
        }
    }
    match &input.evidence {
        GlossaryEvidence::NativeTooltip { source_id } => {
            validate_identity(source_id, "evidence_source_id")?;
        }
        GlossaryEvidence::OwnerDocumentation {
            document_id,
            evidence_tag,
        } => {
            validate_identity(document_id, "evidence_document_id")?;
            validate_non_empty_text(evidence_tag)?;
        }
        GlossaryEvidence::Unavailable(_) => {}
    }
    if matches!(
        input.visibility,
        GlossaryTermVisibility::Hidden | GlossaryTermVisibility::Unknown
    ) && matches!(input.evidence, GlossaryEvidence::NativeTooltip { .. })
    {
        // Hidden native entries are legal; visibility remains an explicit source decision.
    }
    Ok(())
}

pub(super) fn manifest_definitions(
    manifest: &ContentManifest,
) -> std::collections::BTreeMap<(String, String), crate::ContentCursorBinding> {
    manifest
        .definitions
        .iter()
        .map(|definition| {
            (
                (
                    definition.entity_kind.clone(),
                    definition.namespaced_id.clone(),
                ),
                manifest.cursor_binding(),
            )
        })
        .collect()
}
