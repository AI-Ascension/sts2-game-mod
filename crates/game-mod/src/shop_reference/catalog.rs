// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::ContentManifest;

use super::{
    definition::{ShopCatalog, ShopDefinition, ShopDefinitionInput},
    encoding::definition_bytes,
    error::{ShopCatalogError, ShopSourceError, map_source_error},
    identity::validate_identity,
    model::{
        SHOP_MAX_DEFINITION_BYTES, SHOP_MAX_DEFINITIONS, SHOP_REFERENCE_ENTITY_KIND,
        SHOP_REFERENCE_PRODUCER_VERSION, ShopCatalogBinding, ShopFamilyCoverage, ShopFamilyState,
        ShopReferenceKind, ShopSemanticReference,
    },
    validation::{ShopTarget, validate_definition},
};

/// Bounded source snapshot used to construct one immutable shop catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale for localized shop values.
    pub locale: String,
    /// Exact source producer compatibility.
    pub producer_version: String,
    /// Explicit support state for the shop family.
    pub family: ShopFamilyCoverage,
    /// Typed source-owned shop records.
    pub definitions: Vec<ShopDefinitionInput>,
}

/// Owner-local source boundary for copied shop definitions.
///
/// The trait is deliberately read-only: no method here can buy, sell, restock, or spend gold, and
/// none can admit a run or observe a live shop.
pub trait ShopCatalogSource {
    /// Copies bounded shop definitions without changing any shop or run state.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<ShopCatalogSnapshot, ShopSourceError>;
}

/// Producer that binds shop definitions to one immutable content manifest and locale.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShopCatalogProducer;

impl ShopCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: ShopCatalogSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<ShopCatalog, ShopCatalogError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        validate_fences(manifest, &snapshot)?;
        if !manifest
            .families
            .iter()
            .any(|family| family.entity_kind == SHOP_REFERENCE_ENTITY_KIND)
        {
            return Err(ShopCatalogError::MissingFamily);
        }
        let manifest_ids = manifest_shop_ids(manifest);
        let family = coverage(&snapshot.family, manifest_ids.len())?;
        let binding = ShopCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            producer_version: snapshot.producer_version,
        };
        if snapshot.family.state != ShopFamilyState::Handled {
            if let Some(first) = snapshot.definitions.first() {
                return Err(ShopCatalogError::UnknownDefinition(first.shop_id.clone()));
            }
            return Ok(ShopCatalog::from_parts(binding, family, BTreeMap::new()));
        }
        let records = collect_records(snapshot.definitions)?;
        if records.len() != manifest_ids.len() {
            return Err(ShopCatalogError::FamilyCountMismatch);
        }
        if let Some(unknown) = records
            .keys()
            .find(|shop_id| !manifest_ids.contains(*shop_id))
        {
            return Err(ShopCatalogError::UnknownDefinition(unknown.clone()));
        }
        for definition in records.values() {
            validate_manifest_references(definition, manifest)?;
        }
        let definitions = bind_definitions(&binding, &manifest_ids, records)?;
        Ok(ShopCatalog::from_parts(binding, family, definitions))
    }
}

fn validate_fences(
    manifest: &ContentManifest,
    snapshot: &ShopCatalogSnapshot,
) -> Result<(), ShopCatalogError> {
    if snapshot.manifest != manifest.cursor_binding() {
        return Err(ShopCatalogError::ManifestMismatch);
    }
    if snapshot.locale != manifest.locale {
        return Err(ShopCatalogError::LocaleMismatch);
    }
    if snapshot.producer_version != SHOP_REFERENCE_PRODUCER_VERSION {
        return Err(ShopCatalogError::ProducerVersionMismatch);
    }
    if snapshot.family.entity_kind != SHOP_REFERENCE_ENTITY_KIND {
        return Err(ShopCatalogError::FamilyIdentityMismatch);
    }
    if snapshot.definitions.len() > SHOP_MAX_DEFINITIONS {
        return Err(ShopCatalogError::InvalidInput("definitions"));
    }
    Ok(())
}

fn manifest_shop_ids(manifest: &ContentManifest) -> BTreeSet<String> {
    manifest
        .definitions
        .iter()
        .filter(|definition| definition.entity_kind == SHOP_REFERENCE_ENTITY_KIND)
        .map(|definition| definition.namespaced_id.clone())
        .collect()
}

fn coverage(
    family: &ShopFamilyCoverage,
    count: usize,
) -> Result<ShopFamilyCoverage, ShopCatalogError> {
    if family.definition_count != count {
        return Err(ShopCatalogError::FamilyCountMismatch);
    }
    Ok(ShopFamilyCoverage {
        entity_kind: SHOP_REFERENCE_ENTITY_KIND.to_owned(),
        state: family.state,
        definition_count: count,
    })
}

fn collect_records(
    inputs: Vec<ShopDefinitionInput>,
) -> Result<BTreeMap<String, ShopDefinitionInput>, ShopCatalogError> {
    let targets = inputs
        .iter()
        .map(|input| {
            (
                input.shop_id.clone(),
                ShopTarget {
                    visibility: input.visibility,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut records = BTreeMap::new();
    for input in inputs {
        validate_identity(&input.shop_id, "shop_id")?;
        validate_definition(&input, &targets)?;
        let bytes = definition_bytes(&input);
        if bytes > SHOP_MAX_DEFINITION_BYTES {
            return Err(ShopCatalogError::DefinitionTooLarge {
                limit: SHOP_MAX_DEFINITION_BYTES,
                actual: bytes,
            });
        }
        let shop_id = input.shop_id.clone();
        if records.insert(shop_id.clone(), input).is_some() {
            return Err(ShopCatalogError::DuplicateDefinition(shop_id));
        }
    }
    Ok(records)
}

fn bind_definitions(
    binding: &ShopCatalogBinding,
    manifest_ids: &BTreeSet<String>,
    records: BTreeMap<String, ShopDefinitionInput>,
) -> Result<BTreeMap<String, ShopDefinition>, ShopCatalogError> {
    let mut definitions = BTreeMap::new();
    for shop_id in manifest_ids {
        let Some(input) = records.get(shop_id) else {
            return Err(ShopCatalogError::MissingDefinition(shop_id.clone()));
        };
        let definition = ShopDefinition::from_input(binding, input.clone());
        definitions.insert(shop_id.clone(), definition);
    }
    Ok(definitions)
}

fn validate_manifest_references(
    input: &ShopDefinitionInput,
    manifest: &ContentManifest,
) -> Result<(), ShopCatalogError> {
    validate_reference_list(&input.references, manifest)?;
    for entry in &input.entries {
        validate_reference_list(std::slice::from_ref(&entry.definition), manifest)?;
        validate_reference_list(&entry.references, manifest)?;
        for restriction in &entry.restrictions {
            for requirement in &restriction.requirements {
                validate_reference_list(&requirement.references, manifest)?;
            }
            validate_reference_list(&restriction.references, manifest)?;
        }
    }
    for service in &input.services {
        validate_reference_list(&service.selection_candidates, manifest)?;
        for limit in &service.limits {
            validate_reference_list(&limit.references, manifest)?;
        }
        for requirement in &service.eligibility {
            validate_reference_list(&requirement.references, manifest)?;
        }
        validate_reference_list(&service.references, manifest)?;
    }
    for rule in &input.restock {
        validate_reference_list(&rule.restocks_entries, manifest)?;
        validate_reference_list(&rule.references, manifest)?;
    }
    Ok(())
}

fn validate_reference_list(
    references: &[ShopSemanticReference],
    manifest: &ContentManifest,
) -> Result<(), ShopCatalogError> {
    for reference in references {
        let Some(entity_kind) = reference_kind_token(&reference.kind) else {
            continue;
        };
        ensure_manifest_reference(manifest, entity_kind, &reference.id)?;
    }
    Ok(())
}

fn ensure_manifest_reference(
    manifest: &ContentManifest,
    entity_kind: &str,
    namespaced_id: &str,
) -> Result<(), ShopCatalogError> {
    if manifest.definitions.iter().any(|definition| {
        definition.entity_kind == entity_kind && definition.namespaced_id == namespaced_id
    }) {
        Ok(())
    } else {
        Err(ShopCatalogError::UnknownManifestReference {
            entity_kind: entity_kind.to_owned(),
            namespaced_id: namespaced_id.to_owned(),
        })
    }
}

/// Returns the manifest entity family a typed reference resolves to, when it names one.
///
/// Entry and restock-rule references are local to their owning shop definition and therefore name
/// no manifest family; an unclassified reference is deliberately `None` rather than assumed.
#[must_use]
pub(super) fn reference_kind_token(kind: &ShopReferenceKind) -> Option<&str> {
    match kind {
        ShopReferenceKind::Shop => Some(super::model::SHOP_REFERENCE_ENTITY_KIND),
        ShopReferenceKind::Card => Some(super::model::SHOP_REFERENCE_CARD_KIND),
        ShopReferenceKind::Relic => Some(super::model::SHOP_REFERENCE_RELIC_KIND),
        ShopReferenceKind::Potion => Some(super::model::SHOP_REFERENCE_POTION_KIND),
        ShopReferenceKind::Service => Some(super::model::SHOP_REFERENCE_SERVICE_KIND),
        ShopReferenceKind::Currency => Some(super::model::SHOP_REFERENCE_CURRENCY_KIND),
        ShopReferenceKind::Content { entity_kind } => Some(entity_kind),
        ShopReferenceKind::Entry | ShopReferenceKind::RestockRule | ShopReferenceKind::Unknown => {
            None
        }
    }
}
