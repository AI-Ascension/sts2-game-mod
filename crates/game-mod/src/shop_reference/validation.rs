// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use super::{
    catalog::reference_kind_token,
    definition::ShopDefinitionInput,
    entry::{ShopEntryInput, ShopRestriction},
    error::ShopCatalogError,
    field::ShopField,
    identity::{validate_identity, validate_text_value},
    model::{
        SHOP_MAX_ENTRIES, SHOP_MAX_REFERENCES, SHOP_MAX_REQUIREMENTS, SHOP_MAX_RESTOCK_RULES,
        SHOP_MAX_RESTRICTIONS, SHOP_MAX_SELECTION_CANDIDATES, SHOP_MAX_SERVICES, ShopItemKind,
        ShopReferenceKind, ShopSemanticReference, ShopVisibility, visibility_rank,
    },
    pricing::{validate_price, validate_sale},
    restock::within_restock_bounds,
    rules::{
        reject_exposed_value, reject_purchase_action, validate_item_kind, validate_reference_kind,
        validate_requirement, validate_restriction_kind, validate_service_declaration,
        validate_stock, validate_trigger_kind,
    },
    service::ShopServiceInput,
};

/// Visibility and identity of one shop definition already collected from the same snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ShopTarget {
    /// Visibility of the referenced shop definition.
    pub(super) visibility: ShopVisibility,
}

/// Validates one source-owned shop definition before it enters an immutable catalog.
pub(super) fn validate_definition(
    input: &ShopDefinitionInput,
    shop_targets: &BTreeMap<String, ShopTarget>,
) -> Result<(), ShopCatalogError> {
    validate_identity(&input.shop_id, "shop_id")?;
    validate_text_value(&input.label, "shop_label")?;
    if input.generation == 0 {
        return Err(ShopCatalogError::InvalidInput("generation"));
    }
    if input.entries.len() > SHOP_MAX_ENTRIES {
        return Err(ShopCatalogError::InvalidInput("entries"));
    }
    if input.services.len() > SHOP_MAX_SERVICES {
        return Err(ShopCatalogError::InvalidInput("services"));
    }
    if input.restock.len() > SHOP_MAX_RESTOCK_RULES {
        return Err(ShopCatalogError::InvalidInput("restock"));
    }
    if input.references.len() > SHOP_MAX_REFERENCES {
        return Err(ShopCatalogError::InvalidInput("references"));
    }
    reject_exposed_value(input.visibility, &input.label)?;
    let scope = ShopScope::new(input, shop_targets)?;
    for entry in &input.entries {
        scope.validate_entry(entry)?;
    }
    for service in &input.services {
        scope.validate_service(service)?;
    }
    scope.validate_restock()?;
    scope.validate_references(&input.references, input.visibility)
}

/// One shop definition's local identity sets and the snapshot-wide visibility targets.
struct ShopScope<'a> {
    input: &'a ShopDefinitionInput,
    entry_ids: BTreeSet<&'a str>,
    rule_ids: BTreeSet<&'a str>,
    targets: &'a BTreeMap<String, ShopTarget>,
}

impl<'a> ShopScope<'a> {
    fn new(
        input: &'a ShopDefinitionInput,
        targets: &'a BTreeMap<String, ShopTarget>,
    ) -> Result<Self, ShopCatalogError> {
        let entry_ids = collect_entry_ids(input)?;
        let rule_ids = collect_rule_ids(input)?;
        collect_service_ids(input)?;
        Ok(Self {
            input,
            entry_ids,
            rule_ids,
            targets,
        })
    }

    fn validate_entry(&self, entry: &ShopEntryInput) -> Result<(), ShopCatalogError> {
        validate_identity(&entry.entry_id, "entry_id")?;
        validate_text_value(&entry.label, "entry_label")?;
        validate_item_kind(&entry.item_kind)?;
        self.validate_kind_agreement(entry)?;
        validate_price(&entry.price, &self.input.shop_id, &entry.entry_id)?;
        validate_sale(&entry.sale, &self.input.shop_id, &entry.entry_id)?;
        validate_stock(&entry.stock)?;
        reject_exposed_value(entry.visibility, &entry.label)?;
        reject_purchase_action(&entry.purchase_action)?;
        if entry.restrictions.len() > SHOP_MAX_RESTRICTIONS {
            return Err(ShopCatalogError::InvalidInput("restrictions"));
        }
        for restriction in &entry.restrictions {
            self.validate_restriction(restriction, entry.visibility)?;
        }
        self.validate_references(&entry.references, entry.visibility)?;
        self.validate_references(std::slice::from_ref(&entry.definition), entry.visibility)
    }

    /// Rejects an entry whose stated kind contradicts the family it resolves to.
    ///
    /// A definition that resolves to a card, relic, potion, or service must be reported as exactly
    /// that kind, so no supported item can stay hard-coded `Unknown`.
    fn validate_kind_agreement(&self, entry: &ShopEntryInput) -> Result<(), ShopCatalogError> {
        let expected = match entry.definition.kind {
            ShopReferenceKind::Card => ShopItemKind::Card,
            ShopReferenceKind::Relic => ShopItemKind::Relic,
            ShopReferenceKind::Potion => ShopItemKind::Potion,
            ShopReferenceKind::Service => ShopItemKind::Service,
            _ => return Ok(()),
        };
        if entry.item_kind == expected {
            return Ok(());
        }
        Err(ShopCatalogError::KindDisagreesWithDefinition {
            shop_id: self.input.shop_id.clone(),
            entry_id: entry.entry_id.clone(),
            reported: entry.item_kind.clone(),
            family: reference_kind_token(&entry.definition.kind)
                .unwrap_or("unknown")
                .to_owned(),
        })
    }

    fn validate_restriction(
        &self,
        restriction: &ShopRestriction,
        containing: ShopVisibility,
    ) -> Result<(), ShopCatalogError> {
        validate_identity(&restriction.restriction_id, "restriction_id")?;
        validate_text_value(&restriction.label, "restriction_label")?;
        validate_restriction_kind(&restriction.kind)?;
        if restriction.requirements.len() > SHOP_MAX_REQUIREMENTS {
            return Err(ShopCatalogError::InvalidInput("requirements"));
        }
        if let ShopField::Available(0) = restriction.capacity {
            return Err(ShopCatalogError::InvalidInput("capacity"));
        }
        for requirement in &restriction.requirements {
            validate_requirement(requirement)?;
        }
        self.validate_references(&restriction.references, containing)
    }

    fn validate_service(&self, service: &ShopServiceInput) -> Result<(), ShopCatalogError> {
        let invalid = || ShopCatalogError::InvalidService {
            shop_id: self.input.shop_id.clone(),
            service_id: service.service_id.clone(),
        };
        validate_service_declaration(service, invalid)?;
        validate_price(&service.cost, &self.input.shop_id, &service.service_id)?;
        if service.eligibility.len() > SHOP_MAX_REQUIREMENTS
            || service.selection_candidates.len() > SHOP_MAX_SELECTION_CANDIDATES
        {
            return Err(invalid());
        }
        for requirement in &service.eligibility {
            validate_requirement(requirement).map_err(|_| invalid())?;
        }
        validate_stock(&service.stock).map_err(|_| invalid())?;
        reject_exposed_value(service.visibility, &service.label).map_err(|_| invalid())?;
        reject_purchase_action(&service.purchase_action).map_err(|_| invalid())?;
        self.validate_references(&service.selection_candidates, service.visibility)?;
        self.validate_references(&service.references, service.visibility)
    }

    fn validate_restock(&self) -> Result<(), ShopCatalogError> {
        let mut previous = self.input.generation;
        for rule in &self.input.restock {
            let invalid = || ShopCatalogError::InvalidRestock {
                shop_id: self.input.shop_id.clone(),
                rule_id: rule.rule_id.clone(),
            };
            validate_identity(&rule.rule_id, "rule_id").map_err(|_| invalid())?;
            validate_text_value(&rule.label, "rule_label").map_err(|_| invalid())?;
            validate_trigger_kind(&rule.trigger.kind).map_err(|_| invalid())?;
            validate_identity(&rule.trigger.trigger_id, "trigger_id").map_err(|_| invalid())?;
            validate_text_value(&rule.trigger.label, "trigger_label").map_err(|_| invalid())?;
            if !within_restock_bounds(rule) || rule.generation <= previous {
                return Err(invalid());
            }
            previous = rule.generation;
            reject_exposed_value(rule.visibility, &rule.label).map_err(|_| invalid())?;
            self.validate_references(&rule.restocks_entries, rule.visibility)
                .map_err(|_| invalid())?;
            self.validate_references(&rule.references, rule.visibility)
                .map_err(|_| invalid())?;
        }
        Ok(())
    }

    fn validate_references(
        &self,
        references: &[ShopSemanticReference],
        containing: ShopVisibility,
    ) -> Result<(), ShopCatalogError> {
        if references.len() > SHOP_MAX_REFERENCES {
            return Err(ShopCatalogError::InvalidInput("references"));
        }
        for reference in references {
            validate_reference_kind(&reference.kind)?;
            validate_identity(&reference.id, "reference_id")?;
            validate_text_value(&reference.label, "reference_label")?;
            self.validate_reference_target(reference, containing)?;
        }
        Ok(())
    }

    /// Rejects an intra-shop or cross-shop reference that dangles or leaks a restricted target.
    fn validate_reference_target(
        &self,
        reference: &ShopSemanticReference,
        containing: ShopVisibility,
    ) -> Result<(), ShopCatalogError> {
        let leak = |reference_kind| ShopCatalogError::HiddenReferenceLeak {
            shop_id: self.input.shop_id.clone(),
            reference_kind,
        };
        match &reference.kind {
            ShopReferenceKind::Entry => {
                if self.entry_ids.contains(reference.id.as_str()) {
                    Ok(())
                } else {
                    Err(self.dangling(&reference.kind, &reference.id))
                }
            }
            ShopReferenceKind::RestockRule => {
                if self.rule_ids.contains(reference.id.as_str()) {
                    Ok(())
                } else {
                    Err(self.dangling(&reference.kind, &reference.id))
                }
            }
            ShopReferenceKind::Shop if reference.id != self.input.shop_id => {
                let target = self
                    .targets
                    .get(&reference.id)
                    .ok_or_else(|| self.dangling(&reference.kind, &reference.id))?;
                if visibility_rank(containing) > visibility_rank(target.visibility) {
                    return Err(leak(ShopReferenceKind::Shop));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn dangling(&self, reference_kind: &ShopReferenceKind, id: &str) -> ShopCatalogError {
        ShopCatalogError::DanglingReference {
            shop_id: self.input.shop_id.clone(),
            reference_kind: reference_kind.clone(),
            id: id.to_owned(),
        }
    }
}

fn collect_entry_ids(input: &ShopDefinitionInput) -> Result<BTreeSet<&str>, ShopCatalogError> {
    let mut ids = BTreeSet::new();
    for entry in &input.entries {
        if !ids.insert(entry.entry_id.as_str()) {
            return Err(ShopCatalogError::DuplicateEntry {
                shop_id: input.shop_id.clone(),
                entry_id: entry.entry_id.clone(),
            });
        }
    }
    Ok(ids)
}

fn collect_rule_ids(input: &ShopDefinitionInput) -> Result<BTreeSet<&str>, ShopCatalogError> {
    let mut ids = BTreeSet::new();
    for rule in &input.restock {
        if !ids.insert(rule.rule_id.as_str()) {
            return Err(ShopCatalogError::InvalidRestock {
                shop_id: input.shop_id.clone(),
                rule_id: rule.rule_id.clone(),
            });
        }
    }
    Ok(ids)
}

fn collect_service_ids(input: &ShopDefinitionInput) -> Result<(), ShopCatalogError> {
    let mut ids = BTreeSet::new();
    for service in &input.services {
        if !ids.insert(service.service_id.as_str()) {
            return Err(ShopCatalogError::DuplicateService {
                shop_id: input.shop_id.clone(),
                service_id: service.service_id.clone(),
            });
        }
    }
    Ok(())
}
