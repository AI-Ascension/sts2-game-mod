// SPDX-License-Identifier: MIT

use super::super::{
    RewardActionKind, RewardFieldStatus, RewardOfferState, RewardVisibility, RewardVisibilityScope,
};
use super::identity::RewardLiveOfferReference;
use super::page::RewardLiveOfferEntry;
use super::snapshot::{RewardLiveOffer, RewardLiveOfferDisposition};

pub(super) fn visible(visibility: RewardVisibility, scope: RewardVisibilityScope) -> bool {
    match visibility {
        RewardVisibility::Visible => true,
        RewardVisibility::OwnerOnly => scope == RewardVisibilityScope::Owner,
        RewardVisibility::Hidden | RewardVisibility::Unknown => false,
    }
}

pub(super) fn project_summary(
    offer: &RewardLiveOffer,
    reference: RewardLiveOfferReference,
    scope: RewardVisibilityScope,
) -> RewardLiveOfferEntry {
    let groups = offer
        .groups
        .entries
        .iter()
        .filter(|group| visible(group.visibility, scope))
        .count();
    let items = offer
        .items
        .entries
        .iter()
        .filter(|item| visible(item.visibility, scope))
        .count();
    RewardLiveOfferEntry {
        reference,
        kind: offer.kind.clone(),
        state: offer.state.clone(),
        visibility: offer.visibility,
        groups_status: projected_status(offer.groups.status, offer.groups.entries.len(), groups),
        groups_count: groups,
        items_status: projected_status(offer.items.status, offer.items.entries.len(), items),
        items_count: items,
        disposition: offer_disposition(offer, scope),
    }
}

pub(super) fn offer_disposition(
    offer: &RewardLiveOffer,
    scope: RewardVisibilityScope,
) -> RewardLiveOfferDisposition {
    match &offer.state {
        super::super::RewardField::Unavailable(_) => RewardLiveOfferDisposition::StateUnavailable,
        super::super::RewardField::Available(RewardOfferState::BlockedCapacity) => {
            RewardLiveOfferDisposition::BlockedCapacity
        }
        super::super::RewardField::Available(RewardOfferState::Unknown) => {
            RewardLiveOfferDisposition::UnknownState
        }
        super::super::RewardField::Available(
            RewardOfferState::Claimed | RewardOfferState::Skipped,
        ) => RewardLiveOfferDisposition::NoObservedSelectableAction,
        super::super::RewardField::Available(state) => {
            let observed = offer.groups.entries.iter().any(|group| {
                visible(group.visibility, scope)
                    && group.actions.entries.iter().any(|action| {
                        visible(action.visibility, scope)
                            && match state {
                                RewardOfferState::ReplacementRequired => {
                                    action.kind == RewardActionKind::Replace
                                }
                                _ => matches!(
                                    &action.kind,
                                    &RewardActionKind::Choose
                                        | &RewardActionKind::Claim
                                        | &RewardActionKind::Replace
                                ),
                            }
                    })
            });
            if observed {
                RewardLiveOfferDisposition::ObservedSelectableAction
            } else {
                RewardLiveOfferDisposition::NoObservedSelectableAction
            }
        }
    }
}

pub(super) fn projected_status(
    source: RewardFieldStatus,
    total: usize,
    visible: usize,
) -> RewardFieldStatus {
    if source != RewardFieldStatus::Available {
        source
    } else if visible == total {
        RewardFieldStatus::Available
    } else if visible == 0 {
        RewardFieldStatus::Denied
    } else {
        RewardFieldStatus::Partial
    }
}

pub(super) fn find_item<'a>(
    offer: &'a RewardLiveOffer,
    id: &str,
) -> Option<&'a super::snapshot::RewardLiveItem> {
    offer
        .items
        .entries
        .iter()
        .find(|item| item.id.as_str() == id)
}
