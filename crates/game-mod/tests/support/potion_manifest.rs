// SPDX-License-Identifier: MIT

use crate::content_fixture::ManifestSource;
use sts2_game_mod::{
    ContentDefinitionInput, ContentManifest, ContentManifestProducer, ContentOriginInput,
};

pub fn manifest() -> ContentManifest {
    let mut snapshot = crate::content_fixture::catalog_snapshot();
    snapshot.available_entity_kinds.push("potion".to_owned());
    snapshot
        .registry_definition_counts
        .insert("potion".to_owned(), 4);
    for id in [
        "mod:synthetic:healing",
        "mod:synthetic:choice",
        "mod:synthetic:conditional",
        "mod:synthetic:locked",
    ] {
        snapshot.definitions.push(ContentDefinitionInput {
            entity_kind: "potion".to_owned(),
            namespaced_id: id.to_owned(),
            semantic_inputs: format!("kind=potion;id={id}"),
            localized_text: Some(id.to_owned()),
            origin: ContentOriginInput {
                package_id: Some("mod:synthetic".to_owned()),
                package_version: None,
            },
            override_chain: Vec::new(),
        });
    }
    ContentManifestProducer::new("adapter-v1", ["card".to_owned()])
        .expect("manifest producer")
        .produce(&ManifestSource { snapshot })
        .expect("manifest")
}
