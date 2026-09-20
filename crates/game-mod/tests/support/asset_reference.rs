// SPDX-License-Identifier: MIT

//! The fixture assets and the declared class coverage every asset record shares.

use sts2_game_mod::{
    ASSET_REFERENCE_PRODUCER_VERSION, AssetByteSize, AssetCatalogSnapshot, AssetClassCoverage,
    AssetClassState, AssetEntryInput, AssetField, AssetFieldAvailability, AssetFieldStatus,
    AssetFieldValue, AssetMediaClass, AssetMediaKind, AssetMediaProperties, AssetOrigin,
    AssetRenditionPayload, AssetRetrievalState, AssetVisibility, ContentManifest,
};

/// Every handle the shared fixture publishes, in stable order.
pub const FIXTURE_HANDLES: &[&str] = &[
    "asset.art.bash",
    "asset.audio.blood",
    "asset.hidden.marker",
    "asset.icon.metadata",
    "asset.icon.strike",
    "asset.icon.unavailable",
    "asset.missing.spare",
    "asset.owner.ambience",
];

pub fn definition(
    entity_kind: &str,
    namespaced_id: &str,
) -> sts2_game_mod::AssetDefinitionReference {
    sts2_game_mod::AssetDefinitionReference::new(entity_kind, namespaced_id).expect("definition")
}

pub fn origin(manifest: &ContentManifest) -> AssetOrigin {
    AssetOrigin::new("base:synthetic", Some("1"), &manifest.content_set_revision).expect("origin")
}

pub fn image_properties(width: u32, height: u32) -> AssetMediaProperties {
    AssetMediaProperties::new("image/png", Some(width), Some(height), None).expect("properties")
}

pub fn audio_properties(duration_ms: u64) -> AssetMediaProperties {
    AssetMediaProperties::new("audio/ogg", None, None, Some(duration_ms)).expect("properties")
}

pub fn binary(media_type: &str, len: u64) -> AssetRenditionPayload {
    AssetRenditionPayload::Binary {
        media_type: media_type.to_owned(),
        bytes: vec![0xA5; usize::try_from(len).expect("len")],
    }
}

/// States one field-row inventory with the three carried statuses named explicitly.
pub fn field_rows(
    media_properties: AssetFieldStatus,
    byte_size: AssetFieldStatus,
    rendition: AssetFieldStatus,
) -> Vec<AssetFieldAvailability> {
    let available = AssetFieldStatus::Available;
    vec![
        AssetFieldAvailability {
            field: AssetField::Handle,
            status: available,
        },
        AssetFieldAvailability {
            field: AssetField::Definition,
            status: available,
        },
        AssetFieldAvailability {
            field: AssetField::MediaKind,
            status: available,
        },
        AssetFieldAvailability {
            field: AssetField::RetrievalState,
            status: available,
        },
        AssetFieldAvailability {
            field: AssetField::MediaProperties,
            status: media_properties,
        },
        AssetFieldAvailability {
            field: AssetField::ByteSize,
            status: byte_size,
        },
        AssetFieldAvailability {
            field: AssetField::Origin,
            status: available,
        },
        AssetFieldAvailability {
            field: AssetField::Rendition,
            status: rendition,
        },
    ]
}

fn expiry(manifest: &ContentManifest) -> u64 {
    manifest.catalog_generation + 500
}

/// The identity, media and byte-size declaration one fixture asset states.
pub struct AssetSpec {
    pub handle: &'static str,
    pub definition: sts2_game_mod::AssetDefinitionReference,
    pub media_kind: AssetMediaKind,
    pub properties: AssetMediaProperties,
    pub stored: u64,
    pub decoded: u64,
}

impl AssetSpec {
    pub fn new(
        handle: &'static str,
        entity_kind: &str,
        namespaced_id: &str,
        media_kind: AssetMediaKind,
        properties: AssetMediaProperties,
        stored: u64,
        decoded: u64,
    ) -> Self {
        Self {
            handle,
            definition: definition(entity_kind, namespaced_id),
            media_kind,
            properties,
            stored,
            decoded,
        }
    }
}

fn retrievable(
    manifest: &ContentManifest,
    spec: AssetSpec,
    visibility: AssetVisibility,
) -> AssetEntryInput {
    let size = AssetByteSize::new(spec.stored, spec.decoded).expect("size");
    AssetEntryInput {
        handle: spec.handle.to_owned(),
        definition: spec.definition,
        media_kind: spec.media_kind,
        retrieval_state: AssetRetrievalState::Retrievable,
        media_properties: AssetFieldValue::available(spec.properties.clone()),
        byte_size: AssetFieldValue::available(size),
        origin: origin(manifest),
        rendition: AssetFieldValue::available(binary(spec.properties.media_type(), spec.stored)),
        visibility,
        expires_at_generation: expiry(manifest),
        fields: field_rows(
            AssetFieldStatus::Available,
            AssetFieldStatus::Available,
            AssetFieldStatus::Available,
        ),
    }
}

fn metadata_only(
    manifest: &ContentManifest,
    spec: AssetSpec,
    state: AssetRetrievalState,
    visibility: AssetVisibility,
) -> AssetEntryInput {
    let size = AssetByteSize::new(spec.stored, spec.decoded).expect("size");
    AssetEntryInput {
        handle: spec.handle.to_owned(),
        definition: spec.definition,
        media_kind: spec.media_kind,
        retrieval_state: state,
        media_properties: AssetFieldValue::available(spec.properties),
        byte_size: AssetFieldValue::available(size),
        origin: origin(manifest),
        rendition: AssetFieldValue::available(AssetRenditionPayload::MetadataOnly),
        visibility,
        expires_at_generation: expiry(manifest),
        fields: field_rows(
            AssetFieldStatus::Available,
            AssetFieldStatus::Available,
            AssetFieldStatus::Available,
        ),
    }
}

/// A retrievable icon whose declared bytes are served.
pub fn icon_entry(manifest: &ContentManifest) -> AssetEntryInput {
    retrievable(
        manifest,
        AssetSpec::new(
            "asset.icon.strike",
            "card",
            "card.strike",
            AssetMediaKind::Icon,
            image_properties(64, 64),
            512,
            1_024,
        ),
        AssetVisibility::Visible,
    )
}

/// A retrievable piece of card art whose declared bytes are served.
pub fn art_entry(manifest: &ContentManifest) -> AssetEntryInput {
    retrievable(
        manifest,
        AssetSpec::new(
            "asset.art.bash",
            "card",
            "card.bash",
            AssetMediaKind::Art,
            image_properties(512, 256),
            1_280,
            2_048,
        ),
        AssetVisibility::Visible,
    )
}

/// A retrievable audio asset with a duration and no dimensions.
pub fn audio_entry(manifest: &ContentManifest) -> AssetEntryInput {
    retrievable(
        manifest,
        AssetSpec::new(
            "asset.audio.blood",
            "relic",
            "relic.burning_blood",
            AssetMediaKind::Audio,
            audio_properties(12_000),
            4_096,
            8_192,
        ),
        AssetVisibility::Visible,
    )
}

/// An icon whose metadata is stated and whose bytes are withheld at this scope.
pub fn metadata_only_entry(manifest: &ContentManifest) -> AssetEntryInput {
    metadata_only(
        manifest,
        AssetSpec::new(
            "asset.icon.metadata",
            "card",
            "card.strike",
            AssetMediaKind::Icon,
            image_properties(32, 32),
            256,
            512,
        ),
        AssetRetrievalState::MetadataOnly,
        AssetVisibility::Visible,
    )
}

/// Art this boundary exists for but cannot serve on this host.
pub fn unavailable_entry(manifest: &ContentManifest) -> AssetEntryInput {
    metadata_only(
        manifest,
        AssetSpec::new(
            "asset.icon.unavailable",
            "card",
            "card.bash",
            AssetMediaKind::Art,
            image_properties(128, 128),
            512,
            512,
        ),
        AssetRetrievalState::RetrievalUnavailable,
        AssetVisibility::Visible,
    )
}

/// An asset content links but the installed build does not contain.
pub fn missing_entry(manifest: &ContentManifest) -> AssetEntryInput {
    AssetEntryInput {
        handle: "asset.missing.spare".to_owned(),
        definition: definition("card", "card.strike"),
        media_kind: AssetMediaKind::Art,
        retrieval_state: AssetRetrievalState::AssetMissing,
        media_properties: AssetFieldValue::not_applicable(),
        byte_size: AssetFieldValue::not_observed(),
        origin: origin(manifest),
        rendition: AssetFieldValue::not_observed(),
        visibility: AssetVisibility::Visible,
        expires_at_generation: expiry(manifest),
        fields: field_rows(
            AssetFieldStatus::NotApplicable,
            AssetFieldStatus::NotObserved,
            AssetFieldStatus::NotObserved,
        ),
    }
}

/// Audio only an owner-authorized scope may observe.
pub fn owner_only_entry(manifest: &ContentManifest) -> AssetEntryInput {
    metadata_only(
        manifest,
        AssetSpec::new(
            "asset.owner.ambience",
            "relic",
            "relic.burning_blood",
            AssetMediaKind::Audio,
            audio_properties(8_000),
            2_048,
            4_096,
        ),
        AssetRetrievalState::MetadataOnly,
        AssetVisibility::OwnerOnly,
    )
}

/// A marker no visibility scope may observe.
pub fn hidden_entry(manifest: &ContentManifest) -> AssetEntryInput {
    metadata_only(
        manifest,
        AssetSpec::new(
            "asset.hidden.marker",
            "card",
            "card.strike",
            AssetMediaKind::Icon,
            image_properties(16, 16),
            128,
            256,
        ),
        AssetRetrievalState::MetadataOnly,
        AssetVisibility::Hidden,
    )
}

/// An otherwise valid icon whose reference has already passed its generation.
pub fn expired_entry(manifest: &ContentManifest) -> AssetEntryInput {
    let mut entry = icon_entry(manifest);
    entry.handle = "asset.icon.expired".to_owned();
    entry.expires_at_generation = manifest.catalog_generation;
    entry
}

/// The shared fixture: two audio assets and six image assets.
pub fn fixture_assets(manifest: &ContentManifest) -> Vec<AssetEntryInput> {
    vec![
        icon_entry(manifest),
        art_entry(manifest),
        audio_entry(manifest),
        metadata_only_entry(manifest),
        unavailable_entry(manifest),
        missing_entry(manifest),
        owner_only_entry(manifest),
        hidden_entry(manifest),
    ]
}

/// States per-class coverage with the count the assets actually declare.
pub fn coverage_for(assets: &[AssetEntryInput]) -> Vec<AssetClassCoverage> {
    AssetMediaClass::all()
        .iter()
        .map(|class| AssetClassCoverage {
            class: *class,
            state: AssetClassState::Projected,
            asset_count: assets
                .iter()
                .filter(|entry| entry.media_kind.media_class() == *class)
                .count(),
            unsupported_fields: Vec::new(),
        })
        .collect()
}

pub fn snapshot(manifest: &ContentManifest, assets: Vec<AssetEntryInput>) -> AssetCatalogSnapshot {
    AssetCatalogSnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        producer_version: ASSET_REFERENCE_PRODUCER_VERSION.to_owned(),
        content_revision: manifest.content_set_revision.clone(),
        generation: manifest.catalog_generation,
        classes: coverage_for(&assets),
        assets,
    }
}
