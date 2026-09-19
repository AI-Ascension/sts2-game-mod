// SPDX-License-Identifier: MIT

//! Single-document and single-screen reads over one immutable catalog.
//!
//! Every read is one-way.  Reading a blocking message or a dismissible tutorial overlay returns
//! what the screen says and what its controls are; it has no representation for a click, a
//! confirmation or a dismissal, and [`ScreenReadEffect::None`] states that the read left the
//! game exactly as it was.

use super::ReferenceTextError;
use super::catalog::{ReferenceCatalogBinding, ReferenceTextCatalog};
use super::model::{
    ReferenceDiscovery, ReferenceEntityReference, ReferenceFamily, ReferenceTextSegment,
    ReferenceUnavailableReason, validate_identity,
};
use super::screen::{PublicControlKind, PublicScreenKind};

/// How completely one read satisfied its request.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReferenceCompleteness {
    /// The reference was readable and its text was returned.
    Complete,
    /// Part of the reference could not be returned, and the withheld parts say why.
    Partial,
    /// The reference exists but this producer or policy does not project its text.
    Unsupported,
}

/// What reading a public screen changed in the game.
///
/// The type has exactly one variant on purpose: a read cannot click, confirm or dismiss, so it
/// cannot produce any other effect.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ScreenReadEffect {
    /// Reading the screen changed nothing: it was not clicked, confirmed or dismissed.
    None,
}

/// One read section of a reference document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceSectionRead {
    /// Stable section ID.
    pub section_id: String,
    /// Heading the owner renders for this section.
    pub heading: String,
    /// Ordered section text.
    pub segments: Vec<ReferenceTextSegment>,
    /// Searchable keywords the owner associated with this section.
    pub keywords: Vec<String>,
}

/// One read reference document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceDocumentRead {
    /// Binding of the catalog that produced this read.
    pub binding: ReferenceCatalogBinding,
    /// Reference family of this document.
    pub family: ReferenceFamily,
    /// Stable document identity.
    pub namespaced_id: String,
    /// Locale this document is authored in.
    pub locale: String,
    /// Discovery policy that governed this read.
    pub discovery: ReferenceDiscovery,
    /// Text revision of this document.
    pub revision: String,
    /// Ordered document title.
    pub title: Vec<ReferenceTextSegment>,
    /// Ordered sections.
    pub sections: Vec<ReferenceSectionRead>,
    /// References from this document to other definitions.
    pub related: Vec<ReferenceEntityReference>,
    /// How completely this read was satisfied.
    pub completeness: ReferenceCompleteness,
    /// Aggregate retained bytes of the text this read returned.
    pub retained_bytes: usize,
}

/// One read control of a public screen.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicControlRead {
    /// Stable control ID.
    pub control_id: String,
    /// Semantic control kind.
    pub kind: PublicControlKind,
    /// Label the owner renders for this control.
    pub label: String,
    /// Whether the owner reports the control as available.
    pub available: bool,
    /// Why the control is unavailable, when the owner reported it.
    pub unavailable_reason: Option<ReferenceUnavailableReason>,
    /// Whether this control's value was withheld rather than read.
    pub value_withheld: bool,
}

/// One read public screen.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicScreenRead {
    /// Binding of the catalog that produced this read.
    pub binding: ReferenceCatalogBinding,
    /// Semantic screen kind.
    pub screen_kind: PublicScreenKind,
    /// Stable screen ID.
    pub screen_id: String,
    /// Locale this screen's text is authored in.
    pub locale: String,
    /// Whether the screen blocks progress until acknowledged.
    pub blocking: bool,
    /// Whether the screen can be dismissed.
    pub dismissible: bool,
    /// Ordered visible text.
    pub visible_text: Vec<ReferenceTextSegment>,
    /// Described controls.
    pub controls: Vec<PublicControlRead>,
    /// Number of controls whose private value was withheld.
    pub withheld_private_inputs: usize,
    /// How completely this read was satisfied.
    pub completeness: ReferenceCompleteness,
    /// What reading this screen changed.
    pub effect: ScreenReadEffect,
}

fn withheld_text(reason: ReferenceUnavailableReason) -> Vec<ReferenceTextSegment> {
    vec![ReferenceTextSegment::Unavailable(reason)]
}

impl ReferenceTextCatalog {
    /// Reads one reference document, or reports why its text is withheld.
    pub fn read_document(
        &self,
        family: ReferenceFamily,
        namespaced_id: &str,
    ) -> Result<ReferenceDocumentRead, ReferenceTextError> {
        validate_identity(namespaced_id, "namespaced_id")?;
        let key = (family, namespaced_id.to_owned());
        let Some(stored) = self.documents.get(&key) else {
            return Err(ReferenceTextError::NotFound);
        };
        // A family this producer does not project, or a policy that withholds the document, is
        // answered with an explicit reason instead of empty text that would read as "authored".
        let withheld = if !family.is_inventoried() {
            Some(ReferenceUnavailableReason::UnsupportedScope)
        } else if !stored.discovery.is_readable() {
            Some(ReferenceUnavailableReason::Locked)
        } else {
            None
        };
        let (title, sections, completeness, retained_bytes) = match withheld {
            Some(reason) => (
                withheld_text(reason),
                stored
                    .sections
                    .iter()
                    .map(|section| ReferenceSectionRead {
                        section_id: section.section_id.clone(),
                        heading: section.heading.clone(),
                        segments: withheld_text(reason),
                        keywords: section.keywords.clone(),
                    })
                    .collect(),
                if family.is_inventoried() {
                    ReferenceCompleteness::Partial
                } else {
                    ReferenceCompleteness::Unsupported
                },
                0,
            ),
            None => (
                stored.title.clone(),
                stored
                    .sections
                    .iter()
                    .map(|section| ReferenceSectionRead {
                        section_id: section.section_id.clone(),
                        heading: section.heading.clone(),
                        segments: section.segments.clone(),
                        keywords: section.keywords.clone(),
                    })
                    .collect(),
                ReferenceCompleteness::Complete,
                stored.retained_bytes,
            ),
        };
        Ok(ReferenceDocumentRead {
            binding: self.binding.clone(),
            family,
            namespaced_id: namespaced_id.to_owned(),
            locale: stored.locale.clone(),
            discovery: stored.discovery,
            revision: stored.revision.clone(),
            title,
            sections,
            related: stored.related.clone(),
            completeness,
            retained_bytes,
        })
    }

    /// Reads one supported public screen without interacting with it.
    pub fn read_screen(&self, screen_id: &str) -> Result<PublicScreenRead, ReferenceTextError> {
        validate_identity(screen_id, "screen_id")?;
        let Some(stored) = self.screens.get(screen_id) else {
            return Err(ReferenceTextError::NotFound);
        };
        let completeness = if stored.screen_kind.is_classified() {
            ReferenceCompleteness::Complete
        } else {
            // An unclassified screen is reported as unsupported scope rather than presented as a
            // screen whose description looks complete.
            ReferenceCompleteness::Unsupported
        };
        let controls = stored
            .controls
            .iter()
            .map(|control| PublicControlRead {
                control_id: control.control_id.clone(),
                kind: control.kind,
                label: control.label.clone(),
                available: control.available,
                unavailable_reason: control.unavailable_reason,
                value_withheld: control.kind.carries_private_input(),
            })
            .collect();
        Ok(PublicScreenRead {
            binding: self.binding.clone(),
            screen_kind: stored.screen_kind,
            screen_id: screen_id.to_owned(),
            locale: stored.locale.clone(),
            blocking: stored.blocking,
            dismissible: stored.dismissible,
            visible_text: stored.visible_text.clone(),
            controls,
            withheld_private_inputs: stored.withheld_private_inputs,
            completeness,
            effect: ScreenReadEffect::None,
        })
    }
}
