// SPDX-License-Identifier: MIT

use super::fixture::{CatalogSource, manifest, snapshot};
use sts2_game_mod::{
    CharacterCatalogError, CharacterCatalogProducer, CharacterListQuery,
    CharacterLoadoutAvailability, CharacterText, CharacterUnavailableReason,
    CharacterVisibilityScope, ContentUnlockState,
};

fn query(locale: &str, scope: CharacterVisibilityScope) -> CharacterListQuery {
    CharacterListQuery {
        locale: locale.to_owned(),
        scope,
        limit: 8,
        continuation: None,
    }
}

fn localized_source(
    manifest: &sts2_game_mod::ContentManifest,
    ember_name: &str,
    ember_description: &str,
    locked_name: &str,
    locked_description: &str,
) -> CatalogSource {
    let mut source = snapshot(manifest);
    let ember = source
        .definitions
        .iter_mut()
        .find(|definition| definition.character_id == "base:character:ember")
        .expect("synthetic ember");
    ember.name = CharacterText::Available(ember_name.to_owned());
    ember.description = CharacterText::Available(ember_description.to_owned());

    let locked = source
        .definitions
        .iter_mut()
        .find(|definition| definition.character_id == "mod:character:locked")
        .expect("synthetic locked character");
    locked.name = CharacterText::Available(locked_name.to_owned());
    locked.description = CharacterText::Available(locked_description.to_owned());

    CatalogSource {
        snapshot: Ok(source),
    }
}

#[test]
fn locale_change_preserves_character_and_loadout_identity() {
    let manifest_en = manifest();
    let mut manifest_fr = manifest_en.clone();
    manifest_fr.locale = "fr-FR".to_owned();

    // Locale is separate from the manifest cursor identity in this owner-local API.
    assert_eq!(manifest_en.locale, "en-US");
    assert_eq!(manifest_fr.locale, "fr-FR");
    assert_eq!(manifest_en.cursor_binding(), manifest_fr.cursor_binding());
    assert_eq!(
        manifest_en.content_set_revision,
        manifest_fr.content_set_revision
    );

    let source_en = localized_source(
        &manifest_en,
        "Ember (synthetic en-US)",
        "Synthetic en-US character description.",
        "Locked (synthetic en-US)",
        "Synthetic en-US progression requirement.",
    );
    let source_fr = localized_source(
        &manifest_fr,
        "Braise (synthetic fr-FR)",
        "Description synthétique fr-FR.",
        "Verrouillé (synthetic fr-FR)",
        "Condition de progression synthétique fr-FR.",
    );
    let source_en_before = source_en.clone();
    let source_fr_before = source_fr.clone();

    assert_eq!(
        source_en
            .snapshot
            .as_ref()
            .expect("English fixture")
            .manifest,
        source_fr
            .snapshot
            .as_ref()
            .expect("French fixture")
            .manifest
    );
    assert_eq!(
        source_en
            .snapshot
            .as_ref()
            .expect("English fixture")
            .producer_version,
        source_fr
            .snapshot
            .as_ref()
            .expect("French fixture")
            .producer_version
    );
    assert_eq!(
        source_en.snapshot.as_ref().expect("English fixture").locale,
        "en-US"
    );
    assert_eq!(
        source_fr.snapshot.as_ref().expect("French fixture").locale,
        "fr-FR"
    );

    let producer = CharacterCatalogProducer::new();
    let catalog_en = producer
        .produce(&manifest_en, &source_en)
        .expect("English catalog");
    let catalog_fr = producer
        .produce(&manifest_fr, &source_fr)
        .expect("French catalog");

    // This source equality covers only immutable fixture inputs, not profile progress.
    assert_eq!(source_en, source_en_before);
    assert_eq!(source_fr, source_fr_before);
    assert_eq!(catalog_en.binding().manifest, catalog_fr.binding().manifest);
    assert_eq!(
        catalog_en.binding().producer_version,
        catalog_fr.binding().producer_version
    );
    assert_eq!(catalog_en.locale(), "en-US");
    assert_eq!(catalog_fr.locale(), "fr-FR");

    let en_page = catalog_en
        .reader()
        .list(&query("en-US", CharacterVisibilityScope::Public))
        .expect("English page");
    let fr_page = catalog_fr
        .reader()
        .list(&query("fr-FR", CharacterVisibilityScope::Public))
        .expect("French page");
    let en_summary = en_page
        .entries
        .iter()
        .find(|entry| entry.reference.character_id == "base:character:ember")
        .expect("English Ember summary");
    let fr_summary = fr_page
        .entries
        .iter()
        .find(|entry| entry.reference.character_id == "base:character:ember")
        .expect("French Ember summary");

    assert_eq!(
        en_summary.reference.character_id,
        fr_summary.reference.character_id
    );
    assert_ne!(en_summary.reference, fr_summary.reference);
    assert_eq!(en_summary.reference.catalog.locale, "en-US");
    assert_eq!(fr_summary.reference.catalog.locale, "fr-FR");
    assert_eq!(
        en_summary.name,
        CharacterText::Available("Ember (synthetic en-US)".to_owned())
    );
    assert_eq!(
        fr_summary.name,
        CharacterText::Available("Braise (synthetic fr-FR)".to_owned())
    );

    let ember_en = catalog_en
        .get(&en_summary.reference, CharacterVisibilityScope::Public)
        .expect("English detail");
    let ember_fr = catalog_fr
        .get(&fr_summary.reference, CharacterVisibilityScope::Public)
        .expect("French detail");
    assert_eq!(
        ember_en.reference.character_id,
        ember_fr.reference.character_id
    );
    assert_eq!(
        ember_en.description,
        CharacterText::Available("Synthetic en-US character description.".to_owned())
    );
    assert_eq!(
        ember_fr.description,
        CharacterText::Available("Description synthétique fr-FR.".to_owned())
    );
    assert_eq!(
        catalog_fr.get(&en_summary.reference, CharacterVisibilityScope::Public),
        Err(CharacterCatalogError::StaleReference)
    );

    let en_loadout_ids = ember_en
        .loadouts
        .iter()
        .map(|loadout| loadout.loadout_id.as_str())
        .collect::<Vec<_>>();
    let fr_loadout_ids = ember_fr
        .loadouts
        .iter()
        .map(|loadout| loadout.loadout_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(en_loadout_ids, fr_loadout_ids);
    assert_eq!(
        ember_en.loadouts[0].starting_deck,
        ember_fr.loadouts[0].starting_deck
    );
    assert_eq!(
        ember_en.loadouts[0].starting_relics,
        ember_fr.loadouts[0].starting_relics
    );
    assert_eq!(
        ember_en.loadouts[0]
            .starting_deck
            .value()
            .expect("English deck")[0]
            .namespaced_id,
        ember_fr.loadouts[0]
            .starting_deck
            .value()
            .expect("French deck")[0]
            .namespaced_id
    );
    assert_eq!(
        ember_en.loadouts[0]
            .starting_relics
            .value()
            .expect("English relics")[0]
            .namespaced_id,
        ember_fr.loadouts[0]
            .starting_relics
            .value()
            .expect("French relics")[0]
            .namespaced_id
    );

    let locked_en_page = catalog_en
        .reader()
        .list(&query("en-US", CharacterVisibilityScope::Reference))
        .expect("English reference page");
    let locked_fr_page = catalog_fr
        .reader()
        .list(&query("fr-FR", CharacterVisibilityScope::Reference))
        .expect("French reference page");
    let locked_en_summary = locked_en_page
        .entries
        .iter()
        .find(|entry| entry.reference.character_id == "mod:character:locked")
        .expect("English locked summary");
    let locked_fr_summary = locked_fr_page
        .entries
        .iter()
        .find(|entry| entry.reference.character_id == "mod:character:locked")
        .expect("French locked summary");
    assert_eq!(locked_en_summary.unlock_state, ContentUnlockState::Locked);
    assert_eq!(locked_fr_summary.unlock_state, ContentUnlockState::Locked);
    assert_eq!(locked_en_summary.available_loadout_count, 0);
    assert_eq!(locked_fr_summary.available_loadout_count, 0);

    let locked_en = catalog_en
        .get(
            &locked_en_summary.reference,
            CharacterVisibilityScope::Reference,
        )
        .expect("English locked detail");
    let locked_fr = catalog_fr
        .get(
            &locked_fr_summary.reference,
            CharacterVisibilityScope::Reference,
        )
        .expect("French locked detail");
    for locked in [&locked_en, &locked_fr] {
        assert_eq!(
            locked.unlock.value().expect("unlock").state,
            ContentUnlockState::Locked
        );
        assert_eq!(
            locked.loadouts[0].availability,
            CharacterLoadoutAvailability::Unavailable(CharacterUnavailableReason::Unsupported)
        );
        assert_eq!(
            locked
                .unlock
                .value()
                .expect("unlock")
                .requirements
                .value()
                .expect("requirements")[0]
                .progression_id,
            "progression:unlock-locked"
        );
    }
    assert_eq!(
        locked_en.loadouts[0].availability,
        locked_fr.loadouts[0].availability
    );
}
