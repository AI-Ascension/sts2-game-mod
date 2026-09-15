// SPDX-License-Identifier: MIT

use sha2::{Digest, Sha256};

use super::{
    ContentDefinition, ContentDefinitionInput, ContentFamily, ContentOriginInput, ContentPackage,
};

pub(super) fn definition_semantic_revision(definition: &ContentDefinitionInput) -> String {
    let mut canonical = String::new();
    field(&mut canonical, "kind", &definition.entity_kind);
    field(&mut canonical, "id", &definition.namespaced_id);
    field(&mut canonical, "semantic", &definition.semantic_inputs);
    origin_fields(&mut canonical, &definition.origin);
    references(&mut canonical, &definition.override_chain);
    digest(&canonical)
}

pub(super) fn definition_text_revision(
    locale: &str,
    definition: &ContentDefinitionInput,
) -> String {
    let mut canonical = String::new();
    field(&mut canonical, "locale", locale);
    field(&mut canonical, "kind", &definition.entity_kind);
    field(&mut canonical, "id", &definition.namespaced_id);
    optional_field(&mut canonical, "text", definition.localized_text.as_deref());
    digest(&canonical)
}

pub(super) fn content_set_revision(
    game_build: &str,
    packages: &[ContentPackage],
    families: &[ContentFamily],
    definitions: &[ContentDefinition],
) -> String {
    let mut canonical = String::new();
    field(&mut canonical, "game_build", game_build);
    package_fields(&mut canonical, packages);
    for family in families {
        field(&mut canonical, "family.kind", &family.entity_kind);
        field(
            &mut canonical,
            "family.count",
            &family.definition_count.to_string(),
        );
    }
    for definition in definitions {
        field(&mut canonical, "definition.kind", &definition.entity_kind);
        field(&mut canonical, "definition.id", &definition.namespaced_id);
        field(
            &mut canonical,
            "definition.semantic_revision",
            &definition.semantic_revision,
        );
    }
    digest(&canonical)
}

pub(super) fn localized_text_revision(locale: &str, definitions: &[ContentDefinition]) -> String {
    let mut canonical = String::new();
    field(&mut canonical, "locale", locale);
    for definition in definitions {
        field(&mut canonical, "definition.kind", &definition.entity_kind);
        field(&mut canonical, "definition.id", &definition.namespaced_id);
        field(
            &mut canonical,
            "definition.localized_text_revision",
            &definition.localized_text_revision,
        );
    }
    digest(&canonical)
}

pub(super) fn inventory_revision(
    game_build: &str,
    packages: &[ContentPackage],
    families: &[ContentFamily],
    definitions: &[ContentDefinition],
) -> String {
    let mut canonical = String::new();
    field(&mut canonical, "game_build", game_build);
    package_fields(&mut canonical, packages);
    family_fields(&mut canonical, families);
    for definition in definitions {
        field(&mut canonical, "definition.kind", &definition.entity_kind);
        field(&mut canonical, "definition.id", &definition.namespaced_id);
        field(
            &mut canonical,
            "definition.handled",
            if definition.handled { "true" } else { "false" },
        );
        field(
            &mut canonical,
            "definition.semantic_revision",
            &definition.semantic_revision,
        );
    }
    digest(&canonical)
}

fn package_fields(canonical: &mut String, packages: &[ContentPackage]) {
    for package in packages {
        field(canonical, "package.order", &package.order.to_string());
        field(canonical, "package.id", &package.package_id);
        optional_field(
            canonical,
            "package.version",
            package.package_version.as_deref(),
        );
    }
}

fn family_fields(canonical: &mut String, families: &[ContentFamily]) {
    for family in families {
        field(canonical, "family.kind", &family.entity_kind);
        field(
            canonical,
            "family.handled",
            if family.handled { "true" } else { "false" },
        );
        field(
            canonical,
            "family.count",
            &family.definition_count.to_string(),
        );
    }
}

fn origin_fields(canonical: &mut String, origin: &ContentOriginInput) {
    optional_field(canonical, "origin.package", origin.package_id.as_deref());
    optional_field(
        canonical,
        "origin.version",
        origin.package_version.as_deref(),
    );
}

fn references(canonical: &mut String, references: &[String]) {
    field(canonical, "override.count", &references.len().to_string());
    for reference in references {
        field(canonical, "override.reference", reference);
    }
}

fn field(canonical: &mut String, name: &str, value: &str) {
    canonical.push_str(name);
    canonical.push(':');
    canonical.push_str(&value.len().to_string());
    canonical.push(':');
    canonical.push_str(value);
    canonical.push('|');
}

fn optional_field(canonical: &mut String, name: &str, value: Option<&str>) {
    match value {
        Some(value) => {
            field(canonical, name, "present");
            let value_name = format!("{name}.value");
            field(canonical, &value_name, value);
        }
        None => field(canonical, name, "unknown"),
    }
}

fn digest(value: &str) -> String {
    let bytes = Sha256::digest(value.as_bytes());
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
