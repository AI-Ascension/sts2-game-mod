// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::{ContentCursorBinding, ContentManifest};

use super::error::map_source_error;
use super::model::{
    LocaleCompleteness, LocaleDirection, LocaleEntityReference, LocaleEntryInput, LocaleInput,
    LocalePlaceholder, LocalePluralCategory, LocaleRenderedSegment, LocaleTextSegment,
};
use super::validation::{
    validate_entry, validate_entry_count, validate_locales, validate_reference_shape,
};
use super::{
    LOCALE_REFERENCE_DEFAULT_PAGE_ITEMS, LOCALE_REFERENCE_MAX_VARIANTS,
    LOCALE_REFERENCE_PRODUCER_VERSION, LocaleCatalogError, LocaleSourceError,
};

/// Bounded source snapshot used to construct one immutable locale catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocaleCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Exact source producer compatibility.
    pub producer_version: String,
    /// Locale used when no fallback step carries text.
    pub default_locale: String,
    /// Ordered supported locales with explicit fallback chains.
    pub locales: Vec<LocaleInput>,
    /// Typed source-owned rendered text records.
    pub entries: Vec<LocaleEntryInput>,
}

/// Owner-local source boundary for copied rendered text.
///
/// Implementations return owned values and never expose install paths, assemblies, account
/// identifiers, save data or raw host exceptions through this seam.
pub trait LocaleRenderSource {
    /// Copies the owner locale registry without switching the active game language.
    fn read_catalog(&self) -> Result<LocaleCatalogSnapshot, LocaleSourceError>;
}

/// Binding that fences one catalog and every reference it produced.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocaleCatalogBinding {
    /// Content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Producer identity that built the catalog.
    pub producer_version: String,
}

/// One request for a reference's text in one locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocaleRenderRequest {
    /// Requested locale tag.
    pub locale: String,
    /// Stable entity family of the requested reference.
    pub entity_kind: String,
    /// Stable namespaced ID of the requested reference.
    pub namespaced_id: String,
    /// Plural category the caller was given; `None` requests the non-plural form.
    pub plural: Option<LocalePluralCategory>,
    /// Typed placeholder values.
    pub placeholders: Vec<LocalePlaceholder>,
}

/// One rendered response with its locale, fallback and completeness metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocaleRenderedText {
    /// Stable entity family, identical in every language.
    pub entity_kind: String,
    /// Stable namespaced ID, identical in every language.
    pub namespaced_id: String,
    /// Locale the caller requested.
    pub requested_locale: String,
    /// Locale that actually supplied the text.
    pub effective_locale: String,
    /// Fallback chain consulted, ending with the effective locale.
    pub fallback_chain: Vec<String>,
    /// Text direction of the effective locale.
    pub direction: LocaleDirection,
    /// Revision of the text that supplied this render.
    pub text_revision: String,
    /// Plural category of the entry that actually supplied this render.
    pub effective_plural: LocalePluralCategory,
    /// How completely the request was satisfied.
    pub completeness: LocaleCompleteness,
    /// Ordered rendered segments.
    pub segments: Vec<LocaleRenderedSegment>,
    /// Declared placeholder names that had no usable supplied value.
    pub unresolved_placeholders: Vec<String>,
}

/// A reference to one definition's text, fenced to the catalog binding that produced it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocaleTextReference {
    /// Binding of the catalog that produced this reference.
    pub binding: LocaleCatalogBinding,
    /// Locale this reference was produced for.
    pub locale: String,
    /// Language-independent reference.
    pub reference: LocaleEntityReference,
}

/// Query for a locale-partitioned entry listing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocaleEntryListQuery {
    /// Restrict to one locale; `None` lists every locale.
    pub locale: Option<String>,
    /// Restrict to one entity family; `None` lists every family.
    pub entity_kind: Option<String>,
    /// Requested page size.
    pub page_size: usize,
}

impl Default for LocaleEntryListQuery {
    fn default() -> Self {
        Self {
            locale: None,
            entity_kind: None,
            page_size: LOCALE_REFERENCE_DEFAULT_PAGE_ITEMS,
        }
    }
}

/// Continuation bound to one locale, one catalog revision and one query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocaleContinuation {
    /// Locale signature the page was produced for.
    pub locale_signature: String,
    /// Catalog revision signature the page was produced under.
    pub revision_signature: String,
    /// Query signature the page was produced for.
    pub query_signature: String,
    /// Next offset into the filtered listing.
    pub offset: usize,
}

/// One summary row of a locale-partitioned listing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocaleEntrySummary {
    /// Stable entity family.
    pub entity_kind: String,
    /// Stable namespaced ID.
    pub namespaced_id: String,
    /// Locale this entry is rendered in.
    pub locale: String,
    /// Plural category of this entry.
    pub plural: LocalePluralCategory,
    /// Text revision of this entry.
    pub revision: String,
    /// Number of ordered segments.
    pub segment_count: usize,
}

/// One page of a locale-partitioned listing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocaleEntryPage {
    /// Summary rows in deterministic order.
    pub items: Vec<LocaleEntrySummary>,
    /// Total matching entries across every page.
    pub total: usize,
    /// Continuation for the next page, when another page exists.
    pub next: Option<LocaleContinuation>,
}

pub(super) type EntryKey = (String, String, String, LocalePluralCategory);

#[derive(Clone, Debug)]
pub(super) struct StoredEntry {
    pub(super) revision: String,
    pub(super) segments: Vec<LocaleTextSegment>,
    pub(super) requires: Vec<String>,
}

/// An immutable, source-only, locale-qualified rendered-text catalog.
#[derive(Clone, Debug)]
pub struct LocaleCatalog {
    pub(super) binding: LocaleCatalogBinding,
    pub(super) default_locale: String,
    pub(super) locale_order: Vec<String>,
    pub(super) locales: BTreeMap<String, LocaleInput>,
    pub(super) entries: BTreeMap<EntryKey, StoredEntry>,
    pub(super) ordered_keys: Vec<EntryKey>,
    pub(super) variants: BTreeMap<(String, String), usize>,
}

/// Producer that binds rendered text to one immutable content manifest and locale set.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LocaleCatalogProducer;

impl LocaleCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: LocaleRenderSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<LocaleCatalog, LocaleCatalogError> {
        let snapshot = source.read_catalog().map_err(map_source_error)?;
        let manifest_binding = manifest.cursor_binding();
        if snapshot.manifest != manifest_binding {
            return Err(LocaleCatalogError::ManifestMismatch);
        }
        if snapshot.producer_version != LOCALE_REFERENCE_PRODUCER_VERSION {
            return Err(LocaleCatalogError::ProducerVersionMismatch);
        }
        validate_entry_count(snapshot.entries.len())?;
        let locales = validate_locales(&snapshot.locales, &snapshot.default_locale)?;
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
        let mut entries: BTreeMap<EntryKey, StoredEntry> = BTreeMap::new();
        let mut variants: BTreeMap<(String, String), usize> = BTreeMap::new();
        for input in &snapshot.entries {
            validate_entry(input, &locales)?;
            let identity = (input.entity_kind.clone(), input.namespaced_id.clone());
            if !known.contains(&identity) {
                return Err(LocaleCatalogError::UnknownManifestReference {
                    entity_kind: input.entity_kind.clone(),
                    namespaced_id: input.namespaced_id.clone(),
                });
            }
            for segment in &input.segments {
                if let LocaleTextSegment::Reference(reference) = segment {
                    validate_reference_shape(reference)?;
                    let target = (
                        reference.entity_kind.clone(),
                        reference.namespaced_id.clone(),
                    );
                    if !known.contains(&target) {
                        return Err(LocaleCatalogError::UnknownManifestReference {
                            entity_kind: reference.entity_kind.clone(),
                            namespaced_id: reference.namespaced_id.clone(),
                        });
                    }
                }
            }
            let count = variants.entry(identity).or_insert(0);
            *count += 1;
            if *count > LOCALE_REFERENCE_MAX_VARIANTS {
                return Err(LocaleCatalogError::InvalidInput("variants"));
            }
            let key = (
                input.entity_kind.clone(),
                input.namespaced_id.clone(),
                input.locale.clone(),
                input.plural,
            );
            let stored = StoredEntry {
                revision: input.revision.clone(),
                segments: input.segments.clone(),
                requires: input.requires.clone(),
            };
            if entries.insert(key.clone(), stored).is_some() {
                return Err(LocaleCatalogError::DuplicateEntry {
                    entity_kind: input.entity_kind.clone(),
                    namespaced_id: input.namespaced_id.clone(),
                    locale: input.locale.clone(),
                    plural: input.plural,
                });
            }
        }
        let ordered_keys: Vec<EntryKey> = entries.keys().cloned().collect();
        let locale_order = snapshot
            .locales
            .iter()
            .map(|input| input.locale.clone())
            .collect();
        Ok(LocaleCatalog {
            binding: LocaleCatalogBinding {
                manifest: manifest_binding,
                producer_version: snapshot.producer_version,
            },
            default_locale: snapshot.default_locale,
            locale_order,
            locales,
            entries,
            ordered_keys,
            variants,
        })
    }
}
