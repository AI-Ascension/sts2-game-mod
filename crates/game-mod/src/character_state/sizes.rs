// SPDX-License-Identifier: MIT

use super::{
    definition::{
        CharacterResourceDefinition, CharacterResourceKind, CharacterResourceValueDefinition,
        SecondaryEntityDefinition, SecondaryEntityKind,
    },
    model::CharacterStateCatalogBinding,
};

/// Measures the full binding retained by a static or live detail.
pub(super) fn catalog_binding_bytes(binding: &CharacterStateCatalogBinding) -> usize {
    binding.manifest.adapter_compatibility.len()
        + binding.manifest.content_set_revision.len()
        + binding.manifest.localized_text_revision.len()
        + binding.manifest.inventory_revision.len()
        + binding.locale.len()
        + binding.producer_version.len()
        + 8
}

/// Measures the complete static resource detail, including its catalog witness.
pub(super) fn resource_definition_bytes(definition: &CharacterResourceDefinition) -> usize {
    catalog_binding_bytes(&definition.reference.catalog)
        + definition.reference.definition_id.len()
        + definition.character_id.len()
        + definition.mode_id.len()
        + definition.label.len()
        + definition.rule_reference.len()
        + resource_kind_bytes(&definition.kind)
        + resource_value_definition_bytes(&definition.value)
        + 8
}

/// Measures the complete static secondary-entity detail, including its catalog witness.
pub(super) fn secondary_definition_bytes(definition: &SecondaryEntityDefinition) -> usize {
    catalog_binding_bytes(&definition.reference.catalog)
        + definition.reference.definition_id.len()
        + definition.character_id.len()
        + definition.mode_id.len()
        + definition.label.len()
        + definition.rule_reference.len()
        + secondary_kind_bytes(&definition.kind)
        + 8
}

fn resource_kind_bytes(kind: &CharacterResourceKind) -> usize {
    match kind {
        CharacterResourceKind::Custom(value) => value.len(),
        _ => 1,
    }
}

fn secondary_kind_bytes(kind: &SecondaryEntityKind) -> usize {
    match kind {
        SecondaryEntityKind::Custom(value) => value.len(),
        _ => 1,
    }
}

fn resource_value_definition_bytes(definition: &CharacterResourceValueDefinition) -> usize {
    match definition {
        CharacterResourceValueDefinition::Integer { unit }
        | CharacterResourceValueDefinition::Boolean { unit } => unit.as_str().len() + 1,
        CharacterResourceValueDefinition::Decimal { unit, .. } => unit.as_str().len() + 2,
        CharacterResourceValueDefinition::Text => 1,
        CharacterResourceValueDefinition::Custom { kind, unit } => {
            kind.len() + unit.as_ref().map_or(0, |unit| unit.as_str().len()) + 1
        }
    }
}
