// SPDX-License-Identifier: MIT

use sts2_game_mod::{
    ContentManifest, LOCALE_REFERENCE_PRODUCER_VERSION, LocaleCatalog, LocaleCatalogProducer,
    LocaleCatalogSnapshot, LocaleDirection, LocaleEntityReference, LocaleEntryInput, LocaleInput,
    LocalePlaceholder, LocalePlaceholderValue, LocalePluralCategory, LocaleRenderSource,
    LocaleSourceError, LocaleTextSegment,
};

#[path = "locale_manifest.rs"]
mod manifest_support;
pub use manifest_support::*;

/// Default locale of every synthesized catalog.
pub const EN: &str = "en-US";
/// A second left-to-right locale that falls back to [`EN`].
pub const DE: &str = "de-DE";
/// A right-to-left locale that falls back to [`EN`].
pub const AR: &str = "ar-EG";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Source {
    pub snapshot: LocaleCatalogSnapshot,
}

impl LocaleRenderSource for Source {
    fn read_catalog(&self) -> Result<LocaleCatalogSnapshot, LocaleSourceError> {
        Ok(self.snapshot.clone())
    }
}

pub fn locales() -> Vec<LocaleInput> {
    vec![
        LocaleInput {
            locale: EN.to_owned(),
            direction: LocaleDirection::LeftToRight,
            fallback: vec![EN.to_owned()],
        },
        LocaleInput {
            locale: DE.to_owned(),
            direction: LocaleDirection::LeftToRight,
            fallback: vec![DE.to_owned(), EN.to_owned()],
        },
        LocaleInput {
            locale: AR.to_owned(),
            direction: LocaleDirection::RightToLeft,
            fallback: vec![AR.to_owned(), EN.to_owned()],
        },
    ]
}

pub fn entry(
    kind: &str,
    id: &str,
    locale: &str,
    plural: LocalePluralCategory,
    segments: Vec<LocaleTextSegment>,
    requires: &[&str],
) -> LocaleEntryInput {
    LocaleEntryInput {
        entity_kind: kind.to_owned(),
        namespaced_id: id.to_owned(),
        locale: locale.to_owned(),
        plural,
        revision: format!("text-{locale}-{id}"),
        segments,
        requires: requires.iter().map(|name| (*name).to_owned()).collect(),
    }
}

pub fn text(value: &str) -> LocaleTextSegment {
    LocaleTextSegment::Text(value.to_owned())
}

pub fn placeholder(name: &str) -> LocaleTextSegment {
    LocaleTextSegment::Placeholder(name.to_owned())
}

pub fn effect(kind: &str, amount: &str) -> LocaleTextSegment {
    LocaleTextSegment::Effect {
        kind: kind.to_owned(),
        amount: amount.to_owned(),
    }
}

pub fn reference(kind: &str, id: &str) -> LocaleEntityReference {
    LocaleEntityReference {
        entity_kind: kind.to_owned(),
        namespaced_id: id.to_owned(),
    }
}

pub fn value(name: &str, value: LocalePlaceholderValue) -> LocalePlaceholder {
    LocalePlaceholder {
        name: name.to_owned(),
        value,
    }
}

pub fn snapshot(
    manifest: &ContentManifest,
    locales: Vec<LocaleInput>,
    entries: Vec<LocaleEntryInput>,
) -> LocaleCatalogSnapshot {
    LocaleCatalogSnapshot {
        manifest: manifest.cursor_binding(),
        producer_version: LOCALE_REFERENCE_PRODUCER_VERSION.to_owned(),
        default_locale: EN.to_owned(),
        locales,
        entries,
    }
}

pub fn catalog(manifest: &ContentManifest, entries: Vec<LocaleEntryInput>) -> LocaleCatalog {
    LocaleCatalogProducer::new()
        .produce(
            manifest,
            &Source {
                snapshot: snapshot(manifest, locales(), entries),
            },
        )
        .expect("catalog")
}

/// A baseline reproduction of one strike card in English and German.
pub fn baseline_entries() -> Vec<LocaleEntryInput> {
    vec![
        entry(
            "card",
            "strike",
            EN,
            LocalePluralCategory::Other,
            vec![text("Deal "), effect("damage", "6"), text(" damage.")],
            &[],
        ),
        entry(
            "card",
            "strike",
            DE,
            LocalePluralCategory::Other,
            vec![
                text("Verursache "),
                effect("damage", "6"),
                text(" Schaden."),
            ],
            &[],
        ),
    ]
}
