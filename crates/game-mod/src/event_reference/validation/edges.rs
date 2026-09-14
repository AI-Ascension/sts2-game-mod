// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::super::EventCatalogError;
use super::super::definition::{EventNarrativePage, EventOptionInput};
use super::super::model::{
    EVENT_MAX_OPTIONS, EventSemanticReference, EventSemanticReferenceKind, EventVisibility,
    validate_identity, visibility_rank,
};
use super::EventScope;

/// Collects option identities with their visibility, rejecting duplicates.
pub(super) fn collect_options(
    options: &[EventOptionInput],
) -> Result<BTreeMap<&str, EventVisibility>, EventCatalogError> {
    if options.len() > EVENT_MAX_OPTIONS {
        return Err(EventCatalogError::InvalidInput("options"));
    }
    let mut ids = BTreeMap::new();
    for option in options {
        validate_identity(&option.option_id, "option_id")?;
        if ids
            .insert(option.option_id.as_str(), option.visibility)
            .is_some()
        {
            return Err(EventCatalogError::InvalidInput("duplicate_option"));
        }
    }
    Ok(ids)
}

/// Validates the page-to-option membership relation.
///
/// Every offered option must exist and be no more restricted than its page, every option must be
/// offered by exactly one page, and no option may remain uncovered.
pub(super) fn validate_membership(
    event_id: &str,
    pages: &[EventNarrativePage],
    options: &BTreeMap<&str, EventVisibility>,
) -> Result<(), EventCatalogError> {
    let mut offered_by: BTreeMap<&str, &str> = BTreeMap::new();
    for page in pages {
        for option_id in &page.offered_options {
            let Some(option_visibility) = options.get(option_id.as_str()) else {
                return Err(EventCatalogError::UnknownOptionReference {
                    event_id: event_id.to_owned(),
                    option_id: option_id.clone(),
                });
            };
            if visibility_rank(page.visibility) > visibility_rank(*option_visibility) {
                return Err(EventCatalogError::HiddenReferenceLeak {
                    event_id: event_id.to_owned(),
                    reference_kind: EventSemanticReferenceKind::Option,
                });
            }
            if offered_by
                .insert(option_id.as_str(), page.page_id.as_str())
                .is_some()
            {
                return Err(EventCatalogError::DuplicateOptionMembership {
                    event_id: event_id.to_owned(),
                    option_id: option_id.clone(),
                });
            }
        }
    }
    for option_id in options.keys() {
        if !offered_by.contains_key(option_id) {
            return Err(EventCatalogError::UncoveredOption {
                event_id: event_id.to_owned(),
                option_id: (*option_id).to_owned(),
            });
        }
    }
    Ok(())
}

/// Validates one list of semantic reference edges against target visibility.
///
/// A page, option, or event target more restricted than the referencing record is rejected: a
/// visible record must never disclose a hidden or owner-only target identity or label.
pub(super) fn validate_reference_edges(
    scope: &EventScope<'_>,
    event_visibility: &BTreeMap<String, EventVisibility>,
    containing: EventVisibility,
    references: &[EventSemanticReference],
) -> Result<(), EventCatalogError> {
    for reference in references {
        match &reference.kind {
            EventSemanticReferenceKind::Page => {
                let Some(target) = scope.pages.get(reference.id.as_str()) else {
                    return Err(EventCatalogError::UnknownPageReference {
                        event_id: scope.event_id.to_owned(),
                        page_id: reference.id.clone(),
                    });
                };
                reject_more_visible(scope.event_id, containing, *target, &reference.kind)?;
            }
            EventSemanticReferenceKind::Option => {
                let Some(target) = scope.options.get(reference.id.as_str()) else {
                    return Err(EventCatalogError::UnknownOptionReference {
                        event_id: scope.event_id.to_owned(),
                        option_id: reference.id.clone(),
                    });
                };
                reject_more_visible(scope.event_id, containing, *target, &reference.kind)?;
            }
            EventSemanticReferenceKind::Event => {
                if let Some(target) = event_visibility.get(&reference.id) {
                    reject_more_visible(scope.event_id, containing, *target, &reference.kind)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn reject_more_visible(
    event_id: &str,
    containing: EventVisibility,
    target: EventVisibility,
    kind: &EventSemanticReferenceKind,
) -> Result<(), EventCatalogError> {
    if visibility_rank(containing) > visibility_rank(target) {
        return Err(EventCatalogError::HiddenReferenceLeak {
            event_id: event_id.to_owned(),
            reference_kind: kind.clone(),
        });
    }
    Ok(())
}
