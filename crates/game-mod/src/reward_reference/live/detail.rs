// SPDX-License-Identifier: MIT

use super::super::{
    RewardCatalogError, RewardCatalogReader, RewardField, RewardItem, RewardLiveError,
    RewardUnavailableReason, RewardVisibilityScope,
};
use super::identity::{
    RewardLiveActionReference, RewardLiveGroupReference, RewardLiveItemInstanceReference,
    RewardLiveItemReference, RewardLiveOfferReference,
};
use super::projection::{self, projected_status};
use super::snapshot::{
    RewardLiveActionDetail, RewardLiveCollection, RewardLiveGroupDetail, RewardLiveItemDetail,
    RewardLiveOffer, RewardLiveOfferDetail, RewardLiveSelectionDetail,
};

pub(super) fn project_detail(
    offer: &RewardLiveOffer,
    reference: RewardLiveOfferReference,
    scope: RewardVisibilityScope,
    catalog: &RewardCatalogReader,
) -> Result<RewardLiveOfferDetail, RewardLiveError> {
    let definition = match &offer.definition {
        RewardField::Available(reference) => match catalog.get(reference, scope) {
            Ok(definition) => RewardField::Available(definition),
            Err(RewardCatalogError::ExcludedByScope) => {
                RewardField::Unavailable(RewardUnavailableReason::Denied)
            }
            Err(error) => return Err(RewardLiveError::Catalog(error)),
        },
        RewardField::Unavailable(reason) => RewardField::Unavailable(*reason),
    };
    let groups = offer
        .groups
        .entries
        .iter()
        .filter(|group| projection::visible(group.visibility, scope))
        .map(|group| project_group(offer, group, &reference, scope, catalog))
        .collect::<Result<Vec<_>, _>>()?;
    let items = offer
        .items
        .entries
        .iter()
        .filter(|item| projection::visible(item.visibility, scope))
        .map(|item| project_item(item, &reference, None, scope, catalog))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(RewardLiveOfferDetail {
        reference,
        definition,
        kind: offer.kind.clone(),
        state: offer.state.clone(),
        visibility: offer.visibility,
        groups: RewardLiveCollection::new(
            projected_status(
                offer.groups.status,
                offer.groups.entries.len(),
                groups.len(),
            ),
            groups,
        ),
        items: RewardLiveCollection::new(
            projected_status(offer.items.status, offer.items.entries.len(), items.len()),
            items,
        ),
        disposition: super::projection::offer_disposition(offer, scope),
    })
}

fn project_group(
    offer: &RewardLiveOffer,
    group: &super::snapshot::RewardLiveGroup,
    offer_reference: &RewardLiveOfferReference,
    scope: RewardVisibilityScope,
    catalog: &RewardCatalogReader,
) -> Result<RewardLiveGroupDetail, RewardLiveError> {
    let reference = RewardLiveGroupReference::new(offer_reference.clone(), group.id.clone());
    let items = group
        .items
        .entries
        .iter()
        .filter_map(|id| {
            projection::find_item(offer, id.as_str())
                .filter(|item| projection::visible(item.visibility, scope))
                .map(|_| {
                    RewardLiveItemReference::new(
                        offer_reference.clone(),
                        Some(reference.clone()),
                        id.clone(),
                    )
                })
        })
        .collect::<Vec<_>>();
    let actions = group
        .actions
        .entries
        .iter()
        .filter(|action| projection::visible(action.visibility, scope))
        .map(|action| {
            let targets = action
                .targets
                .entries
                .iter()
                .filter_map(|id| {
                    projection::find_item(offer, id.as_str())
                        .filter(|item| projection::visible(item.visibility, scope))
                        .map(|_| {
                            RewardLiveItemReference::new(
                                offer_reference.clone(),
                                Some(reference.clone()),
                                id.clone(),
                            )
                        })
                })
                .collect::<Vec<_>>();
            Ok(RewardLiveActionDetail {
                reference: RewardLiveActionReference::new(reference.clone(), action.id.clone()),
                kind: action.kind.clone(),
                targets: RewardLiveCollection::new(
                    projected_status(
                        action.targets.status,
                        action.targets.entries.len(),
                        targets.len(),
                    ),
                    targets,
                ),
                visibility: action.visibility,
            })
        })
        .collect::<Result<Vec<_>, RewardLiveError>>()?;
    let selection = match &group.selection {
        RewardField::Available(selection) => match catalog.get(&selection.reward, scope) {
            Ok(definition) => RewardField::Available(RewardLiveSelectionDetail {
                reference: selection.clone(),
                definition: definition.selection,
            }),
            Err(RewardCatalogError::ExcludedByScope) => {
                RewardField::Unavailable(RewardUnavailableReason::Denied)
            }
            Err(error) => return Err(RewardLiveError::Catalog(error)),
        },
        RewardField::Unavailable(reason) => RewardField::Unavailable(*reason),
    };
    Ok(RewardLiveGroupDetail {
        reference,
        selection,
        label: group.label.clone(),
        choose_min: group.choose_min.clone(),
        choose_max: group.choose_max.clone(),
        optional_skip: group.optional_skip.clone(),
        visibility: group.visibility,
        items: RewardLiveCollection::new(
            projected_status(group.items.status, group.items.entries.len(), items.len()),
            items,
        ),
        actions: RewardLiveCollection::new(
            projected_status(
                group.actions.status,
                group.actions.entries.len(),
                actions.len(),
            ),
            actions,
        ),
    })
}

pub(super) fn project_item(
    item: &super::snapshot::RewardLiveItem,
    offer: &RewardLiveOfferReference,
    group: Option<RewardLiveGroupReference>,
    scope: RewardVisibilityScope,
    catalog: &RewardCatalogReader,
) -> Result<RewardLiveItemDetail, RewardLiveError> {
    let (definition, content) = resolve_item(item, scope, catalog)?;
    let instance = match &item.instance_id {
        RewardField::Available(item_instance_id) => {
            RewardField::Available(RewardLiveItemInstanceReference {
                offer: offer.clone(),
                item_instance_id: item_instance_id.clone(),
            })
        }
        RewardField::Unavailable(reason) => RewardField::Unavailable(*reason),
    };
    Ok(RewardLiveItemDetail {
        reference: RewardLiveItemReference::new(offer.clone(), group, item.id.clone()),
        instance,
        definition,
        content,
        label: item.label.clone(),
        quantity: item.quantity.clone(),
        evidence: item.evidence,
        visibility: item.visibility,
    })
}

fn resolve_item(
    item: &super::snapshot::RewardLiveItem,
    scope: RewardVisibilityScope,
    catalog: &RewardCatalogReader,
) -> Result<
    (
        RewardField<RewardItem>,
        RewardField<super::super::RewardSemanticReference>,
    ),
    RewardLiveError,
> {
    match &item.definition {
        RewardField::Available(reference) => match catalog.get_item(reference, scope) {
            Ok(resolved) => Ok((RewardField::Available(resolved), item.content.clone())),
            Err(RewardCatalogError::ExcludedByScope) => Ok((
                RewardField::Unavailable(RewardUnavailableReason::Denied),
                unavailable_content(&item.content, RewardUnavailableReason::Denied),
            )),
            Err(error) => Err(RewardLiveError::Catalog(error)),
        },
        RewardField::Unavailable(reason) => Ok((
            RewardField::Unavailable(*reason),
            unavailable_content(&item.content, RewardUnavailableReason::Unsupported),
        )),
    }
}

fn unavailable_content(
    content: &RewardField<super::super::RewardSemanticReference>,
    reason: RewardUnavailableReason,
) -> RewardField<super::super::RewardSemanticReference> {
    match content {
        RewardField::Available(_) => RewardField::Unavailable(reason),
        RewardField::Unavailable(existing) => RewardField::Unavailable(*existing),
    }
}
