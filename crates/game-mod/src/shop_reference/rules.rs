// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::{
    entry::{ShopRequirement, ShopRestrictionKind},
    error::ShopCatalogError,
    field::{ShopField, ShopText},
    identity::{validate_identity, validate_text_value},
    model::{
        SHOP_MAX_REFERENCES, SHOP_MAX_SERVICE_LIMITS, ShopReferenceKind, ShopServiceKind,
        ShopStockState, ShopVisibility,
    },
    restock::ShopRestockTriggerKind,
    service::{ShopProspectiveChange, ShopServiceInput, ShopServiceSelectionDomain},
};

/// Rejects a record whose visibility says the value must not be revealed.
pub(super) fn reject_exposed_value(
    visibility: ShopVisibility,
    label: &ShopText,
) -> Result<(), ShopCatalogError> {
    if matches!(visibility, ShopVisibility::Hidden | ShopVisibility::Unknown)
        && label.is_available()
    {
        return Err(ShopCatalogError::InvalidInput(
            "hidden_record_exposes_value",
        ));
    }
    Ok(())
}

/// Rejects a transient purchase action inside the source-only static slice.
pub(super) fn reject_purchase_action(
    action: &ShopField<super::model::ShopPurchaseActionReference>,
) -> Result<(), ShopCatalogError> {
    if action.is_available() {
        return Err(ShopCatalogError::InvalidInput("purchase_action"));
    }
    Ok(())
}

/// Rejects a stock state whose remaining quantity contradicts `InStock`.
pub(super) fn validate_stock(stock: &ShopStockState) -> Result<(), ShopCatalogError> {
    if let ShopStockState::InStock { quantity } = stock
        && let ShopField::Available(0) = quantity
    {
        return Err(ShopCatalogError::InvalidInput("stock_quantity"));
    }
    Ok(())
}

/// Rejects an eligibility requirement with an invalid identity, label, or reference bound.
pub(super) fn validate_requirement(requirement: &ShopRequirement) -> Result<(), ShopCatalogError> {
    validate_identity(&requirement.requirement_id, "requirement_id")?;
    validate_text_value(&requirement.label, "requirement_label")?;
    if requirement.references.len() > SHOP_MAX_REFERENCES {
        return Err(ShopCatalogError::InvalidInput("references"));
    }
    Ok(())
}

/// Rejects an item kind token that carries an invalid owner-defined identity.
pub(super) fn validate_item_kind(
    kind: &super::model::ShopItemKind,
) -> Result<(), ShopCatalogError> {
    if let super::model::ShopItemKind::Custom(value)
    | super::model::ShopItemKind::Unsupported(value) = kind
    {
        validate_identity(value, "item_kind")?;
    }
    Ok(())
}

fn validate_service_kind(kind: &ShopServiceKind) -> Result<(), ShopCatalogError> {
    if let ShopServiceKind::Custom(value) | ShopServiceKind::Unsupported(value) = kind {
        validate_identity(value, "service_kind")?;
    }
    Ok(())
}

fn validate_selection_domain(domain: &ShopServiceSelectionDomain) -> Result<(), ShopCatalogError> {
    if let ShopServiceSelectionDomain::Custom(value)
    | ShopServiceSelectionDomain::Unsupported(value) = domain
    {
        validate_identity(value, "selection_domain")?;
    }
    Ok(())
}

fn validate_prospective_change(change: &ShopProspectiveChange) -> Result<(), ShopCatalogError> {
    if let ShopProspectiveChange::Custom(value) | ShopProspectiveChange::Unsupported(value) = change
    {
        validate_identity(value, "prospective_change")?;
    }
    Ok(())
}

/// Rejects a restriction kind token that carries an invalid owner-defined identity.
pub(super) fn validate_restriction_kind(
    kind: &ShopRestrictionKind,
) -> Result<(), ShopCatalogError> {
    if let ShopRestrictionKind::Custom(value) = kind {
        validate_identity(value, "restriction_kind")?;
    }
    Ok(())
}

/// Rejects a restock trigger kind token that carries an invalid owner-defined identity.
pub(super) fn validate_trigger_kind(kind: &ShopRestockTriggerKind) -> Result<(), ShopCatalogError> {
    if let ShopRestockTriggerKind::Custom(value) | ShopRestockTriggerKind::Unsupported(value) = kind
    {
        validate_identity(value, "trigger_kind")?;
    }
    Ok(())
}

/// Rejects a reference family token that carries an invalid owner-defined identity.
pub(super) fn validate_reference_kind(kind: &ShopReferenceKind) -> Result<(), ShopCatalogError> {
    if let ShopReferenceKind::Content { entity_kind } = kind {
        validate_identity(entity_kind, "reference_entity_kind")?;
    }
    Ok(())
}

/// Validates a service's own tokens and its kind/domain/change coherence.
pub(super) fn validate_service_declaration(
    service: &ShopServiceInput,
    invalid: impl Fn() -> ShopCatalogError,
) -> Result<(), ShopCatalogError> {
    validate_identity(&service.service_id, "service_id").map_err(|_| invalid())?;
    validate_text_value(&service.label, "service_label").map_err(|_| invalid())?;
    validate_service_kind(&service.kind).map_err(|_| invalid())?;
    validate_selection_domain(&service.selection_domain).map_err(|_| invalid())?;
    validate_prospective_change(&service.prospective_change).map_err(|_| invalid())?;
    validate_service_coherence(service).map_err(|_| invalid())?;
    validate_service_limits(service).map_err(|_| invalid())
}

/// Rejects a card service whose kind, selection domain, and prospective change disagree.
fn validate_service_coherence(service: &ShopServiceInput) -> Result<(), ShopCatalogError> {
    let expected = match service.kind {
        ShopServiceKind::CardRemoval => (
            ShopServiceSelectionDomain::DeckCard,
            ShopProspectiveChange::RemoveFromDeck,
        ),
        ShopServiceKind::CardUpgrade => (
            ShopServiceSelectionDomain::DeckCard,
            ShopProspectiveChange::UpgradeInDeck,
        ),
        ShopServiceKind::CardTransform => (
            ShopServiceSelectionDomain::DeckCard,
            ShopProspectiveChange::TransformInDeck,
        ),
        _ => return Ok(()),
    };
    if service.selection_domain != expected.0 || service.prospective_change != expected.1 {
        return Err(ShopCatalogError::InvalidInput("service_coherence"));
    }
    Ok(())
}

/// Rejects duplicate or contradictory service limits.
fn validate_service_limits(service: &ShopServiceInput) -> Result<(), ShopCatalogError> {
    if service.limits.len() > SHOP_MAX_SERVICE_LIMITS {
        return Err(ShopCatalogError::InvalidInput("limits"));
    }
    let mut seen = BTreeSet::new();
    for limit in &service.limits {
        validate_identity(&limit.limit_id, "limit_id")?;
        validate_text_value(&limit.label, "limit_label")?;
        if !seen.insert(limit.limit_id.as_str()) {
            return Err(ShopCatalogError::InvalidInput("limits"));
        }
        if let (ShopField::Available(value), ShopField::Available(remaining)) =
            (&limit.value, &limit.remaining)
            && remaining > value
        {
            return Err(ShopCatalogError::InvalidInput("limits"));
        }
    }
    Ok(())
}
