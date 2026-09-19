// SPDX-License-Identifier: MIT

use std::cell::Cell;

use sts2_game_mod::{
    ContentManifest, PublicControlInput, PublicControlKind, PublicScreenInput, PublicScreenKind,
    REFERENCE_TEXT_PRODUCER_VERSION, ReferenceDiscovery, ReferenceDocumentInput,
    ReferenceEntityReference, ReferenceFamily, ReferenceSectionInput, ReferenceTextCatalog,
    ReferenceTextCatalogProducer, ReferenceTextSegment, ReferenceTextSnapshot, ReferenceTextSource,
    ReferenceTextSourceError, ReferenceUnavailableReason,
};

#[path = "locale_manifest.rs"]
mod manifest_support;
pub use manifest_support::manifest;

/// Locale every synthesized document and screen is authored in.
pub const EN: &str = "en-US";
/// A second locale used to prove that a continuation is bound to one catalog revision.
pub const DE: &str = "de-DE";

/// Source that returns one owned snapshot and counts how often it was consulted.
#[derive(Debug, Default)]
pub struct Source {
    pub snapshot: Option<ReferenceTextSnapshot>,
    pub reads: Cell<usize>,
    pub failure: Option<ReferenceTextSourceError>,
}

impl ReferenceTextSource for Source {
    fn read_reference(&self) -> Result<ReferenceTextSnapshot, ReferenceTextSourceError> {
        self.reads.set(self.reads.get() + 1);
        match (&self.failure, &self.snapshot) {
            (Some(error), _) => Err(*error),
            (None, Some(snapshot)) => Ok(snapshot.clone()),
            (None, None) => Err(ReferenceTextSourceError::NoActiveSource),
        }
    }
}

/// Manifest that knows every definition the baseline fixtures reference.
pub fn base_manifest() -> ContentManifest {
    manifest(&[
        ("card", "strike"),
        ("card", "defend"),
        ("text", "tutorial_combat"),
        ("text", "help_keywords"),
        ("text", "lore_spire"),
        ("text", "credits_team"),
        ("text", "ui_hud"),
        ("text", "legacy_secret"),
        ("screen", "tutorial_overlay"),
        ("screen", "blocking_message"),
        ("screen", "settings_screen"),
        ("screen", "unknown_screen"),
    ])
}

/// A typed reference to a definition the manifest knows.
pub fn entity(kind: &str, id: &str) -> ReferenceEntityReference {
    ReferenceEntityReference {
        entity_kind: kind.to_owned(),
        namespaced_id: id.to_owned(),
    }
}

/// Literal reference text.
pub fn text(value: &str) -> ReferenceTextSegment {
    ReferenceTextSegment::Text(value.to_owned())
}

/// Emphasis run the owner rendered.
pub fn emphasis(value: &str) -> ReferenceTextSegment {
    ReferenceTextSegment::Emphasis(value.to_owned())
}

/// Typed reference segment.
pub fn link(kind: &str, id: &str) -> ReferenceTextSegment {
    ReferenceTextSegment::Reference(entity(kind, id))
}

fn keywords(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// One section input.
pub fn section(
    id: &str,
    heading: &str,
    segments: Vec<ReferenceTextSegment>,
    terms: &[&str],
) -> ReferenceSectionInput {
    ReferenceSectionInput {
        section_id: id.to_owned(),
        heading: heading.to_owned(),
        segments,
        keywords: keywords(terms),
    }
}

/// One document input authored in [`EN`].
pub fn document(
    family: ReferenceFamily,
    id: &str,
    discovery: ReferenceDiscovery,
    title: Vec<ReferenceTextSegment>,
    sections: Vec<ReferenceSectionInput>,
    terms: &[&str],
) -> ReferenceDocumentInput {
    ReferenceDocumentInput {
        family,
        namespaced_id: id.to_owned(),
        locale: EN.to_owned(),
        discovery,
        revision: format!("text-{id}"),
        title,
        sections,
        keywords: keywords(terms),
        related: Vec::new(),
    }
}

/// One described control.
pub fn control(
    id: &str,
    kind: PublicControlKind,
    label: &str,
    available: bool,
    reason: Option<ReferenceUnavailableReason>,
) -> PublicControlInput {
    PublicControlInput {
        control_id: id.to_owned(),
        kind,
        label: label.to_owned(),
        available,
        unavailable_reason: reason,
    }
}

/// One public screen input authored in [`EN`].
pub fn screen(
    kind: PublicScreenKind,
    id: &str,
    blocking: bool,
    dismissible: bool,
    visible: Vec<ReferenceTextSegment>,
    controls: Vec<PublicControlInput>,
) -> PublicScreenInput {
    PublicScreenInput {
        screen_kind: kind,
        screen_id: id.to_owned(),
        locale: EN.to_owned(),
        blocking,
        dismissible,
        visible_text: visible,
        controls,
    }
}

/// Baseline documents: two tutorial/help families, a locked lore document, credits, UI text, and
/// one document whose family this producer does not project.
pub fn base_documents() -> Vec<ReferenceDocumentInput> {
    vec![
        document(
            ReferenceFamily::Tutorial,
            "tutorial_combat",
            ReferenceDiscovery::Open,
            vec![text("Combat basics")],
            vec![section(
                "steps",
                "Steps",
                vec![
                    text("Play a card."),
                    ReferenceTextSegment::LineBreak,
                    emphasis("End turn."),
                ],
                &["combat", "turn"],
            )],
            &["tutorial", "combat"],
        ),
        document(
            ReferenceFamily::Help,
            "help_keywords",
            ReferenceDiscovery::Open,
            vec![text("Keyword reference")],
            vec![section(
                "keywords",
                "Keywords",
                vec![text("Exhaust leaves play.")],
                &[],
            )],
            &["help", "keywords"],
        ),
        document(
            ReferenceFamily::Lore,
            "lore_spire",
            ReferenceDiscovery::Locked,
            vec![text("The Spire")],
            Vec::new(),
            &["lore"],
        ),
        document(
            ReferenceFamily::Credits,
            "credits_team",
            ReferenceDiscovery::Discovered,
            vec![text("Credits")],
            Vec::new(),
            &["credits"],
        ),
        document(
            ReferenceFamily::UiText,
            "ui_hud",
            ReferenceDiscovery::Open,
            vec![text("HUD text")],
            Vec::new(),
            &["hud"],
        ),
        document(
            ReferenceFamily::Unknown,
            "legacy_secret",
            ReferenceDiscovery::Open,
            vec![text("Legacy")],
            Vec::new(),
            &["legacy"],
        ),
    ]
}

/// Baseline screens: a dismissible tutorial overlay, a blocking message, a modal carrying a
/// private text input, and one screen kind this producer does not classify.
pub fn base_screens() -> Vec<PublicScreenInput> {
    vec![
        screen(
            PublicScreenKind::TutorialOverlay,
            "tutorial_overlay",
            false,
            true,
            vec![text("Move with the arrow keys.")],
            vec![control(
                "dismiss",
                PublicControlKind::Button,
                "Dismiss",
                true,
                None,
            )],
        ),
        screen(
            PublicScreenKind::BlockingMessage,
            "blocking_message",
            true,
            false,
            vec![text("The Spire calls."), link("card", "strike")],
            vec![control(
                "acknowledge",
                PublicControlKind::Button,
                "Acknowledge",
                true,
                None,
            )],
        ),
        screen(
            PublicScreenKind::Modal,
            "settings_screen",
            true,
            true,
            vec![text("Enter a profile name.")],
            vec![
                control(
                    "name_field",
                    PublicControlKind::TextInput,
                    "Profile name",
                    true,
                    None,
                ),
                control(
                    "confirm",
                    PublicControlKind::Button,
                    "Confirm",
                    false,
                    Some(ReferenceUnavailableReason::PrivateInput),
                ),
            ],
        ),
        screen(
            PublicScreenKind::Unknown,
            "unknown_screen",
            false,
            false,
            vec![text("Unclassified surface")],
            Vec::new(),
        ),
    ]
}

/// Snapshot that binds the baseline fixtures to one manifest.
pub fn snapshot(
    manifest: &ContentManifest,
    documents: Vec<ReferenceDocumentInput>,
    screens: Vec<PublicScreenInput>,
) -> ReferenceTextSnapshot {
    snapshot_in(manifest, EN, documents, screens)
}

/// Snapshot authored in one explicit locale.
pub fn snapshot_in(
    manifest: &ContentManifest,
    locale: &str,
    documents: Vec<ReferenceDocumentInput>,
    screens: Vec<PublicScreenInput>,
) -> ReferenceTextSnapshot {
    ReferenceTextSnapshot {
        manifest: manifest.cursor_binding(),
        producer_version: REFERENCE_TEXT_PRODUCER_VERSION.to_owned(),
        locale: locale.to_owned(),
        documents,
        screens,
    }
}

/// Produces the catalog for one snapshot.
pub fn catalog_for(
    manifest: &ContentManifest,
    snapshot: ReferenceTextSnapshot,
) -> Result<ReferenceTextCatalog, sts2_game_mod::ReferenceTextError> {
    let source = Source {
        snapshot: Some(snapshot),
        ..Source::default()
    };
    ReferenceTextCatalogProducer::new().produce(manifest, &source)
}

/// Produces the baseline catalog.
pub fn catalog(manifest: &ContentManifest) -> ReferenceTextCatalog {
    catalog_for(
        manifest,
        snapshot(manifest, base_documents(), base_screens()),
    )
    .expect("catalog")
}
