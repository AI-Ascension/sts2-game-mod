// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::{ContentCursorBinding, ContentManifest};

use super::error::map_source_error;
use super::model::{
    ReferenceDiscovery, ReferenceDocumentInput, ReferenceEntityReference, ReferenceFamily,
    ReferenceSectionInput, ReferenceTextSegment,
};
use super::screen::{PublicControlInput, PublicScreenInput, PublicScreenKind};
use super::validation::{validate_documents, validate_screens};
use super::{REFERENCE_TEXT_PRODUCER_VERSION, ReferenceTextError, ReferenceTextSourceError};

/// Bounded source snapshot used to construct one immutable reference-text catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceTextSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Exact source producer compatibility.
    pub producer_version: String,
    /// Locale the snapshot's text is authored in.
    pub locale: String,
    /// Typed source-owned reference documents.
    pub documents: Vec<ReferenceDocumentInput>,
    /// Typed source-owned supported public screens.
    pub screens: Vec<PublicScreenInput>,
}

/// Owner-local source boundary for copied reference text and public screen text.
///
/// Implementations return owned values and never expose install paths, assemblies, account
/// identifiers, save data, raw scene nodes or raw host exceptions through this seam.  The seam is
/// read-only: it cannot click, confirm or dismiss the screen it describes.
pub trait ReferenceTextSource {
    /// Copies the owner's reference inventory and current public screen text.
    fn read_reference(&self) -> Result<ReferenceTextSnapshot, ReferenceTextSourceError>;
}

/// Binding that fences one catalog and every read it produced.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ReferenceCatalogBinding {
    /// Content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Producer identity that built the catalog.
    pub producer_version: String,
}

pub(super) type DocumentKey = (ReferenceFamily, String);

#[derive(Clone, Debug)]
pub(super) struct StoredDocument {
    pub(super) locale: String,
    pub(super) discovery: ReferenceDiscovery,
    pub(super) revision: String,
    pub(super) title: Vec<ReferenceTextSegment>,
    pub(super) sections: Vec<ReferenceSectionInput>,
    pub(super) related: Vec<ReferenceEntityReference>,
    pub(super) retained_bytes: usize,
}

#[derive(Clone, Debug)]
pub(super) struct StoredScreen {
    pub(super) screen_kind: PublicScreenKind,
    pub(super) locale: String,
    pub(super) blocking: bool,
    pub(super) dismissible: bool,
    pub(super) visible_text: Vec<ReferenceTextSegment>,
    pub(super) controls: Vec<PublicControlInput>,
    pub(super) withheld_private_inputs: usize,
}

/// An immutable, source-only reference-document and public-screen text catalog.
#[derive(Clone, Debug)]
pub struct ReferenceTextCatalog {
    pub(super) binding: ReferenceCatalogBinding,
    pub(super) locale: String,
    pub(super) documents: BTreeMap<DocumentKey, StoredDocument>,
    pub(super) ordered: Vec<DocumentKey>,
    pub(super) keywords: BTreeMap<DocumentKey, BTreeSet<String>>,
    pub(super) screens: BTreeMap<String, StoredScreen>,
    pub(super) screen_order: Vec<String>,
    pub(super) unsupported_families: BTreeSet<String>,
}

impl ReferenceTextCatalog {
    /// Returns the binding that fences this catalog and every read it produced.
    #[must_use]
    pub fn binding(&self) -> &ReferenceCatalogBinding {
        &self.binding
    }

    /// Returns the locale this catalog's text is authored in.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// Returns the number of reference documents carried by this catalog.
    #[must_use]
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// Returns the number of supported public screens carried by this catalog.
    #[must_use]
    pub fn screen_count(&self) -> usize {
        self.screens.len()
    }

    /// Returns the identities of the supported public screens, in deterministic order.
    #[must_use]
    pub fn screen_ids(&self) -> &[String] {
        &self.screen_order
    }
}

/// Producer that binds reference text to one immutable content manifest.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReferenceTextCatalogProducer;

impl ReferenceTextCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: ReferenceTextSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<ReferenceTextCatalog, ReferenceTextError> {
        let snapshot = source.read_reference().map_err(map_source_error)?;
        let manifest_binding = manifest.cursor_binding();
        if snapshot.manifest != manifest_binding {
            return Err(ReferenceTextError::ManifestMismatch);
        }
        if snapshot.producer_version != REFERENCE_TEXT_PRODUCER_VERSION {
            return Err(ReferenceTextError::ProducerVersionMismatch);
        }
        let known: BTreeSet<(String, String)> = manifest
            .definitions
            .iter()
            .map(|definition| {
                (
                    definition.entity_kind.clone(),
                    definition.namespaced_id.clone(),
                )
            })
            .collect();
        validate_documents(&snapshot.documents, &known)?;
        validate_screens(&snapshot.screens, &known)?;

        let mut documents = BTreeMap::new();
        let mut keywords = BTreeMap::new();
        let mut unsupported_families = BTreeSet::new();
        for input in &snapshot.documents {
            let key = (input.family, input.namespaced_id.clone());
            if !input.family.is_inventoried() {
                unsupported_families.insert(input.namespaced_id.clone());
            }
            let mut terms: BTreeSet<String> = input.keywords.iter().cloned().collect();
            for section in &input.sections {
                terms.extend(section.keywords.iter().cloned());
            }
            let retained_bytes = input
                .title
                .iter()
                .chain(
                    input
                        .sections
                        .iter()
                        .flat_map(|section| section.segments.iter()),
                )
                .map(ReferenceTextSegment::retained_len)
                .sum();
            let stored = StoredDocument {
                locale: input.locale.clone(),
                discovery: input.discovery,
                revision: input.revision.clone(),
                title: input.title.clone(),
                sections: input.sections.clone(),
                related: input.related.clone(),
                retained_bytes,
            };
            if documents.insert(key.clone(), stored).is_some() {
                return Err(ReferenceTextError::DuplicateReference {
                    family: input.family,
                    namespaced_id: input.namespaced_id.clone(),
                });
            }
            keywords.insert(key, terms);
        }
        let ordered: Vec<DocumentKey> = documents.keys().cloned().collect();

        let mut screens = BTreeMap::new();
        for input in &snapshot.screens {
            let withheld_private_inputs = input
                .controls
                .iter()
                .filter(|control| control.kind.carries_private_input())
                .count();
            let stored = StoredScreen {
                screen_kind: input.screen_kind,
                locale: input.locale.clone(),
                blocking: input.blocking,
                dismissible: input.dismissible,
                visible_text: input.visible_text.clone(),
                controls: input.controls.clone(),
                withheld_private_inputs,
            };
            if screens.insert(input.screen_id.clone(), stored).is_some() {
                return Err(ReferenceTextError::DuplicateReference {
                    family: ReferenceFamily::UiText,
                    namespaced_id: input.screen_id.clone(),
                });
            }
        }
        let screen_order: Vec<String> = screens.keys().cloned().collect();
        Ok(ReferenceTextCatalog {
            binding: ReferenceCatalogBinding {
                manifest: manifest_binding,
                producer_version: snapshot.producer_version,
            },
            locale: snapshot.locale,
            documents,
            ordered,
            keywords,
            screens,
            screen_order,
            unsupported_families,
        })
    }
}
