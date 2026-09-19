// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::hygiene::{validate_inert_text, validate_keyword};
use super::model::{
    ReferenceDocumentInput, ReferenceEntityReference, ReferenceTextSegment, validate_identity,
};
use super::screen::{PublicControlInput, PublicScreenInput};
use super::{
    REFERENCE_TEXT_MAX_CONTROLS, REFERENCE_TEXT_MAX_DOCUMENTS, REFERENCE_TEXT_MAX_KEYWORDS,
    REFERENCE_TEXT_MAX_RELATED, REFERENCE_TEXT_MAX_SCREENS, REFERENCE_TEXT_MAX_SECTIONS,
    REFERENCE_TEXT_MAX_SEGMENTS, REFERENCE_TEXT_MAX_TEXT_BYTES, REFERENCE_TEXT_MAX_VISIBLE_BYTES,
    ReferenceTextError,
};

type Known = BTreeSet<(String, String)>;

fn validate_reference_shape(
    reference: &ReferenceEntityReference,
) -> Result<(), ReferenceTextError> {
    validate_identity(&reference.entity_kind, "entity_kind")?;
    validate_identity(&reference.namespaced_id, "namespaced_id")
}

fn validate_known(
    reference: &ReferenceEntityReference,
    known: &Known,
) -> Result<(), ReferenceTextError> {
    validate_reference_shape(reference)?;
    let target = (
        reference.entity_kind.clone(),
        reference.namespaced_id.clone(),
    );
    if !known.contains(&target) {
        return Err(ReferenceTextError::UnknownManifestReference {
            entity_kind: reference.entity_kind.clone(),
            namespaced_id: reference.namespaced_id.clone(),
        });
    }
    Ok(())
}

fn validate_segments(
    segments: &[ReferenceTextSegment],
    known: &Known,
    field: &'static str,
) -> Result<usize, ReferenceTextError> {
    if segments.len() > REFERENCE_TEXT_MAX_SEGMENTS {
        return Err(ReferenceTextError::InvalidInput(field));
    }
    let mut retained = 0usize;
    for segment in segments {
        match segment {
            ReferenceTextSegment::Text(value) | ReferenceTextSegment::Emphasis(value) => {
                validate_inert_text(value, field)?;
            }
            ReferenceTextSegment::LineBreak | ReferenceTextSegment::Unavailable(_) => {}
            ReferenceTextSegment::Reference(reference) => validate_known(reference, known)?,
        }
        retained = retained.saturating_add(segment.retained_len());
    }
    Ok(retained)
}

fn validate_keywords(keywords: &[String]) -> Result<(), ReferenceTextError> {
    if keywords.len() > REFERENCE_TEXT_MAX_KEYWORDS {
        return Err(ReferenceTextError::InvalidInput("keywords"));
    }
    for keyword in keywords {
        validate_keyword(keyword)?;
    }
    Ok(())
}

/// Validates every document before any of them enters the immutable catalog.
pub(super) fn validate_documents(
    documents: &[ReferenceDocumentInput],
    known: &Known,
) -> Result<(), ReferenceTextError> {
    if documents.len() > REFERENCE_TEXT_MAX_DOCUMENTS {
        return Err(ReferenceTextError::InvalidInput("documents"));
    }
    for document in documents {
        validate_document(document, known)?;
    }
    Ok(())
}

fn validate_document(
    document: &ReferenceDocumentInput,
    known: &Known,
) -> Result<(), ReferenceTextError> {
    validate_identity(&document.namespaced_id, "namespaced_id")?;
    validate_identity(&document.locale, "locale")?;
    validate_identity(&document.revision, "revision")?;
    validate_keywords(&document.keywords)?;
    if document.sections.len() > REFERENCE_TEXT_MAX_SECTIONS {
        return Err(ReferenceTextError::InvalidInput("sections"));
    }
    if document.related.len() > REFERENCE_TEXT_MAX_RELATED {
        return Err(ReferenceTextError::InvalidInput("related"));
    }
    let mut retained = validate_segments(&document.title, known, "title")?;
    let mut section_ids = BTreeSet::new();
    for section in &document.sections {
        validate_identity(&section.section_id, "section_id")?;
        if !section_ids.insert(section.section_id.as_str()) {
            return Err(ReferenceTextError::DuplicateReference {
                family: document.family,
                namespaced_id: section.section_id.clone(),
            });
        }
        validate_inert_text(&section.heading, "heading")?;
        validate_keywords(&section.keywords)?;
        retained = retained.saturating_add(section.heading.len());
        retained = retained.saturating_add(validate_segments(
            &section.segments,
            known,
            "section_segments",
        )?);
    }
    for reference in &document.related {
        validate_known(reference, known)?;
    }
    if retained > REFERENCE_TEXT_MAX_TEXT_BYTES {
        return Err(ReferenceTextError::TextTooLarge {
            limit: REFERENCE_TEXT_MAX_TEXT_BYTES,
            actual: retained,
        });
    }
    Ok(())
}

fn validate_control(
    control: &PublicControlInput,
    seen: &mut BTreeSet<String>,
) -> Result<(), ReferenceTextError> {
    validate_identity(&control.control_id, "control_id")?;
    if !seen.insert(control.control_id.clone()) {
        return Err(ReferenceTextError::DuplicateReference {
            family: super::ReferenceFamily::UiText,
            namespaced_id: control.control_id.clone(),
        });
    }
    validate_inert_text(&control.label, "control_label")?;
    // An unavailable control must say why.  `None` here would be indistinguishable from a
    // control the owner simply did not describe, which is the failure this slice refuses.
    if !control.available && control.unavailable_reason.is_none() {
        return Err(ReferenceTextError::MissingUnavailableReason {
            control_id: control.control_id.clone(),
        });
    }
    Ok(())
}

/// Validates every screen before any of them enters the immutable catalog.
pub(super) fn validate_screens(
    screens: &[PublicScreenInput],
    known: &Known,
) -> Result<(), ReferenceTextError> {
    if screens.len() > REFERENCE_TEXT_MAX_SCREENS {
        return Err(ReferenceTextError::InvalidInput("screens"));
    }
    for screen in screens {
        validate_identity(&screen.screen_id, "screen_id")?;
        validate_identity(&screen.locale, "locale")?;
        if screen.controls.len() > REFERENCE_TEXT_MAX_CONTROLS {
            return Err(ReferenceTextError::InvalidInput("controls"));
        }
        let retained = validate_segments(&screen.visible_text, known, "visible_text")?;
        if retained > REFERENCE_TEXT_MAX_VISIBLE_BYTES {
            return Err(ReferenceTextError::TextTooLarge {
                limit: REFERENCE_TEXT_MAX_VISIBLE_BYTES,
                actual: retained,
            });
        }
        let mut seen = BTreeSet::new();
        for control in &screen.controls {
            validate_control(control, &mut seen)?;
        }
    }
    Ok(())
}
