// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::super::model::{validate_identity, visibility_rank};
use super::super::{
    RewardCatalogBinding, RewardCatalogReader, RewardField, RewardFieldStatus, RewardLiveError,
    RewardLiveSnapshotInput, RewardOfferState, RewardVisibilityScope,
};
use super::snapshot::{
    REWARD_LIVE_MAX_ACTIONS_PER_GROUP, REWARD_LIVE_MAX_GROUPS_PER_OFFER,
    REWARD_LIVE_MAX_ITEMS_PER_OFFER, REWARD_LIVE_MAX_OFFERS,
};
mod fields;

pub(super) fn validate_snapshot(
    input: &RewardLiveSnapshotInput,
    expected_catalog: &RewardCatalogBinding,
    expected_instance: &str,
    catalog: &RewardCatalogReader,
) -> Result<(), RewardLiveError> {
    if &input.catalog != expected_catalog || catalog.catalog().binding() != expected_catalog {
        return Err(RewardLiveError::BindingMismatch("catalog"));
    }
    if input.instance_id.as_str() != expected_instance {
        return Err(RewardLiveError::BindingMismatch("instance"));
    }
    fields::validate_collection(&input.offers, "offers")?;
    if input.offers.entries.len() > REWARD_LIVE_MAX_OFFERS {
        return Err(RewardLiveError::InvalidInput("offer_count"));
    }
    if let RewardField::Available(revision) = &input.source_revision {
        validate_identity(revision, "source_revision").map_err(RewardLiveError::Catalog)?;
    }
    let mut offers = BTreeSet::new();
    for offer in &input.offers.entries {
        if !offers.insert(offer.id.as_str()) {
            return Err(RewardLiveError::InvalidInput("duplicate_offer_id"));
        }
        fields::validate_kind(&offer.kind)?;
        fields::validate_collection(&offer.groups, "groups")?;
        fields::validate_collection(&offer.items, "items")?;
        if offer.groups.entries.len() > REWARD_LIVE_MAX_GROUPS_PER_OFFER {
            return Err(RewardLiveError::InvalidInput("group_count"));
        }
        if offer.items.entries.len() > REWARD_LIVE_MAX_ITEMS_PER_OFFER {
            return Err(RewardLiveError::InvalidInput("item_count"));
        }
        let static_offer = validate_static_offer(offer, catalog)?;
        let mut groups = BTreeSet::new();
        let mut offer_items = BTreeSet::new();
        for item in &offer.items.entries {
            if !offer_items.insert(item.id.as_str()) {
                return Err(RewardLiveError::InvalidInput("duplicate_item_id"));
            }
            fields::validate_text_field(&item.label, "item_label")?;
            fields::validate_quantity(&item.quantity, &offer.kind)?;
            if let RewardField::Available(reference) = &item.definition {
                let resolved = catalog
                    .get_item(reference, RewardVisibilityScope::Owner)
                    .map_err(RewardLiveError::Catalog)?;
                if visibility_rank(item.visibility) > visibility_rank(resolved.visibility) {
                    return Err(RewardLiveError::InvalidInput("item_visibility_leak"));
                }
                if let Some(parent) = &static_offer
                    && reference.reward_id != parent.reference.reward_id
                {
                    return Err(RewardLiveError::BindingMismatch("item_reward"));
                }
                if let RewardField::Available(content) = &item.content
                    && content != &resolved.definition
                {
                    return Err(RewardLiveError::BindingMismatch("item_content"));
                }
            }
            if let RewardField::Available(content) = &item.content {
                fields::validate_semantic_reference(content)?;
                if item.definition.value().is_none()
                    && content.kind != super::super::RewardSemanticReferenceKind::Unknown
                {
                    return Err(RewardLiveError::BindingMismatch(
                        "content_without_static_item",
                    ));
                }
            }
        }
        for group in &offer.groups.entries {
            if !groups.insert(group.id.as_str()) {
                return Err(RewardLiveError::InvalidInput("duplicate_group_id"));
            }
            fields::validate_collection(&group.items, "group_items")?;
            fields::validate_collection(&group.actions, "actions")?;
            if group.actions.entries.len() > REWARD_LIVE_MAX_ACTIONS_PER_GROUP {
                return Err(RewardLiveError::InvalidInput("action_count"));
            }
            if let (RewardField::Available(min), RewardField::Available(max)) =
                (&group.choose_min, &group.choose_max)
                && min > max
            {
                return Err(RewardLiveError::InvalidInput("selection_bounds"));
            }
            fields::validate_text_field(&group.label, "group_label")?;
            validate_selection(offer, group, static_offer.as_ref(), catalog)?;
            let mut actions = BTreeSet::new();
            let mut memberships = BTreeSet::new();
            for item_id in &group.items.entries {
                if !offer_items.contains(item_id.as_str()) || !memberships.insert(item_id.as_str())
                {
                    return Err(RewardLiveError::InvalidInput("group_item_membership"));
                }
            }
            for action in &group.actions.entries {
                if !actions.insert(action.id.as_str()) {
                    return Err(RewardLiveError::InvalidInput("duplicate_action_id"));
                }
                fields::validate_text_field(&action.label, "action_label")?;
                fields::validate_collection(&action.targets, "action_targets")?;
                let mut targets = BTreeSet::new();
                for item_id in &action.targets.entries {
                    if !memberships.contains(item_id.as_str()) || !targets.insert(item_id.as_str())
                    {
                        return Err(RewardLiveError::InvalidInput("action_target_membership"));
                    }
                    let target = offer
                        .items
                        .entries
                        .iter()
                        .find(|item| item.id.as_str() == item_id.as_str())
                        .ok_or(RewardLiveError::InvalidInput("action_target_membership"))?;
                    if visibility_rank(action.visibility) > visibility_rank(target.visibility) {
                        return Err(RewardLiveError::InvalidInput("action_visibility_leak"));
                    }
                }
                fields::validate_action_kind(&action.kind)?;
            }
        }
        if let RewardField::Available(RewardOfferState::MultiStage { stage, total }) = &offer.state
            && (*stage == 0 || *total == 0 || stage > total)
        {
            return Err(RewardLiveError::InvalidInput("multi_stage_state"));
        }
    }
    Ok(())
}

fn validate_static_offer(
    offer: &super::snapshot::RewardLiveOffer,
    catalog: &RewardCatalogReader,
) -> Result<Option<super::super::RewardOfferDefinition>, RewardLiveError> {
    let RewardField::Available(reference) = &offer.definition else {
        return Ok(None);
    };
    let definition = catalog
        .get(reference, RewardVisibilityScope::Owner)
        .map_err(RewardLiveError::Catalog)?;
    if definition.kind != offer.kind {
        return Err(RewardLiveError::BindingMismatch("reward_kind"));
    }
    if visibility_rank(offer.visibility) > visibility_rank(definition.visibility) {
        return Err(RewardLiveError::InvalidInput("offer_visibility_leak"));
    }
    Ok(Some(definition))
}

fn validate_selection(
    offer: &super::snapshot::RewardLiveOffer,
    group: &super::snapshot::RewardLiveGroup,
    static_offer: Option<&super::super::RewardOfferDefinition>,
    catalog: &RewardCatalogReader,
) -> Result<(), RewardLiveError> {
    let RewardField::Available(selection) = &group.selection else {
        return Ok(());
    };
    if let Some(static_offer) = static_offer {
        if selection.reward != static_offer.reference {
            return Err(RewardLiveError::BindingMismatch("selection_reward"));
        }
    } else {
        return Err(RewardLiveError::BindingMismatch("selection_without_reward"));
    }
    let definition = catalog
        .get(&selection.reward, RewardVisibilityScope::Owner)
        .map_err(RewardLiveError::Catalog)?;
    if definition.selection.group_id != selection.group_id {
        return Err(RewardLiveError::BindingMismatch("selection_group"));
    }
    if visibility_rank(group.visibility) > visibility_rank(definition.selection.visibility) {
        return Err(RewardLiveError::InvalidInput("group_visibility_leak"));
    }
    validate_identity(&selection.group_id, "selection_group").map_err(RewardLiveError::Catalog)?;
    if offer.definition.status() != RewardFieldStatus::Available {
        return Err(RewardLiveError::BindingMismatch("selection_without_reward"));
    }
    Ok(())
}
