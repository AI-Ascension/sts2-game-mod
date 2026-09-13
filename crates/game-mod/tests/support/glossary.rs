// SPDX-License-Identifier: MIT

#![allow(dead_code)]

use sts2_game_mod::{
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifest,
    ContentManifestProducer, ContentOriginInput, ContentPackageInput, GLOSSARY_PRODUCER_VERSION,
    GlossaryContentReferenceInput, GlossaryContentReferenceSurface, GlossaryDefinitionText,
    GlossaryEvidence, GlossaryProducer, GlossaryQueryScope, GlossaryReferenceVisibilityPolicy,
    GlossarySnapshot, GlossarySource, GlossaryTermInput, GlossaryTermVisibility,
    GlossaryUnresolvedReason,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestSource {
    pub snapshot: ContentCatalogSnapshot,
}

impl ContentCatalogSource for ManifestSource {
    fn read_catalog(&self) -> Result<ContentCatalogSnapshot, sts2_game_mod::ContentSourceError> {
        Ok(self.snapshot.clone())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossaryFixtureSource {
    pub snapshot: Result<GlossarySnapshot, sts2_game_mod::GlossarySourceError>,
}

impl GlossarySource for GlossaryFixtureSource {
    fn read_glossary(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<GlossarySnapshot, sts2_game_mod::GlossarySourceError> {
        self.snapshot.clone()
    }
}

fn package(package_id: &str, version: Option<&str>, order: u32) -> ContentPackageInput {
    ContentPackageInput {
        package_id: package_id.to_owned(),
        package_version: version.map(str::to_owned),
        order,
    }
}

fn definition(kind: &str, id: &str, text: &str) -> ContentDefinitionInput {
    ContentDefinitionInput {
        entity_kind: kind.to_owned(),
        namespaced_id: id.to_owned(),
        semantic_inputs: format!("kind={kind};id={id}"),
        localized_text: Some(text.to_owned()),
        origin: ContentOriginInput {
            package_id: Some("base:game".to_owned()),
            package_version: Some("0.107.1".to_owned()),
        },
        override_chain: Vec::new(),
    }
}

pub fn manifest() -> ContentManifest {
    ContentManifestProducer::new("adapter-v1", ["card".to_owned(), "status".to_owned()])
        .expect("valid manifest producer")
        .produce(&ManifestSource {
            snapshot: ContentCatalogSnapshot {
                generation_before: 11,
                generation_after: 11,
                game_build: "sts2-build:0.107.1".to_owned(),
                locale: "en-US".to_owned(),
                packages: vec![package("base:game", Some("0.107.1"), 0)],
                available_entity_kinds: vec!["card".to_owned(), "status".to_owned()],
                definitions: vec![
                    definition("card", "base:ironclad:strike", "Strike"),
                    definition("status", "base:status:strength", "Strength"),
                ],
            },
        })
        .expect("valid manifest")
}

pub fn snapshot(manifest: &ContentManifest) -> GlossarySnapshot {
    let mut strength = GlossaryTermInput::new("status:strength", "Strength");
    strength.aliases = vec!["Might".to_owned()];
    strength.definition = GlossaryDefinitionText::Available(
        "Gain {amount} Strength; related effects can stack.".to_owned(),
    );
    strength.parameter_placeholders = vec!["{amount}".to_owned()];
    strength.related_terms = vec!["status:weak".to_owned(), "status:missing".to_owned()];
    strength.rule_references = vec!["rule:strength-stack".to_owned()];
    strength.content_references = vec![
        GlossaryContentReferenceInput {
            entity_kind: "card".to_owned(),
            namespaced_id: "base:ironclad:strike".to_owned(),
            surface: GlossaryContentReferenceSurface::RenderedText,
        },
        GlossaryContentReferenceInput {
            entity_kind: "card".to_owned(),
            namespaced_id: "card:missing".to_owned(),
            surface: GlossaryContentReferenceSurface::Definition,
        },
    ];
    strength.evidence = GlossaryEvidence::NativeTooltip {
        source_id: "tooltip:strength".to_owned(),
    };

    let mut weak = GlossaryTermInput::new("status:weak", "Weak");
    weak.aliases = vec!["Frailty".to_owned()];
    weak.definition = GlossaryDefinitionText::Available("Reduce outgoing damage.".to_owned());
    weak.related_terms = vec!["status:strength".to_owned()];
    weak.rule_references = vec!["rule:damage-modifier".to_owned()];
    weak.evidence = GlossaryEvidence::OwnerDocumentation {
        document_id: "docs:glossary:weak".to_owned(),
        evidence_tag: "source-derived".to_owned(),
    };

    let mut eclair = GlossaryTermInput::new("keyword:eclair", "Éclair");
    eclair.aliases = vec!["Pastry".to_owned()];
    eclair.definition = GlossaryDefinitionText::Unavailable(GlossaryUnresolvedReason::Unsupported);
    eclair.visibility = GlossaryTermVisibility::ReferenceOnly;
    eclair.evidence = GlossaryEvidence::OwnerDocumentation {
        document_id: "docs:glossary:eclair".to_owned(),
        evidence_tag: "unverified".to_owned(),
    };

    let mut duplicate = GlossaryTermInput::new("keyword:strength", "Strength");
    duplicate.definition =
        GlossaryDefinitionText::Available("A duplicate display name fixture.".to_owned());
    duplicate.evidence = GlossaryEvidence::OwnerDocumentation {
        document_id: "docs:glossary:duplicate".to_owned(),
        evidence_tag: "source-derived".to_owned(),
    };

    let mut hidden = GlossaryTermInput::new("keyword:hidden", "Hidden");
    hidden.definition = GlossaryDefinitionText::Available("Never exposed.".to_owned());
    hidden.visibility = GlossaryTermVisibility::Hidden;
    hidden.evidence = GlossaryEvidence::Unavailable(GlossaryUnresolvedReason::Denied);

    GlossarySnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        producer_version: GLOSSARY_PRODUCER_VERSION.to_owned(),
        terms: vec![strength, weak, eclair, duplicate, hidden],
    }
}

pub fn catalog() -> sts2_game_mod::GlossaryCatalog {
    let manifest = manifest();
    GlossaryProducer::new(GlossaryReferenceVisibilityPolicy::AllowReferenceTerms)
        .produce(
            &manifest,
            &GlossaryFixtureSource {
                snapshot: Ok(snapshot(&manifest)),
            },
        )
        .expect("valid glossary")
}

pub fn public_scope() -> GlossaryQueryScope {
    GlossaryQueryScope::Public
}

pub fn reference_scope() -> GlossaryQueryScope {
    GlossaryQueryScope::Reference
}
