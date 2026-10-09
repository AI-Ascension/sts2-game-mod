// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::super::{
    RewardCatalogError, RewardField, RewardFieldStatus, RewardItem, RewardLiveError,
    RewardOfferDefinition, RewardVisibilityScope,
};
use super::identity::{
    RewardLiveActionReference, RewardLiveItemReference, RewardLiveOfferReference,
    RewardLiveSnapshotFence, RewardLiveSnapshotReference,
};
use super::page::{RewardLiveContinuation, RewardLiveOfferListQuery, RewardLiveOfferPage};
use super::reader::{RetainedSnapshot, RewardLiveReader};
use super::snapshot::{
    REWARD_LIVE_MAX_PAGE_ITEMS, RewardLiveActionDetail, RewardLiveItemDetail, RewardLiveOffer,
    RewardLiveOfferDetail,
};
use super::{detail, projection};

impl RewardLiveReader {
    /// Lists visible live offers from the named retained snapshot.
    pub fn list_offers(
        &self,
        query: RewardLiveOfferListQuery,
    ) -> Result<RewardLiveOfferPage, RewardLiveError> {
        let retained = self.validate_snapshot_reference(&query.snapshot)?;
        if query.locale != retained.input.catalog.locale {
            return Err(RewardLiveError::Catalog(RewardCatalogError::LocaleMismatch));
        }
        if query.limit == 0 || query.limit > REWARD_LIVE_MAX_PAGE_ITEMS {
            return Err(RewardLiveError::InvalidPageSize);
        }
        let offset = match query.continuation {
            Some(token) => {
                if !Arc::ptr_eq(&token.scope_owner, &self.scope) {
                    return Err(RewardLiveError::WrongReader);
                }
                if !Arc::ptr_eq(&token.snapshot.fence, &query.snapshot.fence)
                    || token.snapshot.reader_generation != query.snapshot.reader_generation
                    || token.locale != query.locale
                    || token.visibility != query.scope
                    || token.limit != query.limit
                {
                    return Err(RewardLiveError::StaleContinuation);
                }
                token.offset
            }
            None => 0,
        };
        let all = retained
            .input
            .offers
            .entries
            .iter()
            .filter(|offer| projection::visible(offer.visibility, query.scope))
            .collect::<Vec<_>>();
        if offset > all.len() {
            return Err(RewardLiveError::StaleContinuation);
        }
        let end = offset
            .checked_add(query.limit)
            .ok_or(RewardLiveError::StaleContinuation)?
            .min(all.len());
        let entries = all[offset..end]
            .iter()
            .map(|offer| {
                projection::project_summary(
                    offer,
                    RewardLiveOfferReference::new(query.snapshot.clone(), offer.id.clone()),
                    query.scope,
                )
            })
            .collect::<Vec<_>>();
        let continuation = if end < all.len() {
            Some(RewardLiveContinuation {
                scope_owner: Arc::clone(&self.scope),
                snapshot: query.snapshot.clone(),
                locale: query.locale.clone(),
                visibility: query.scope,
                limit: query.limit,
                offset: end,
            })
        } else {
            None
        };
        let status = projected_status(
            retained.input.offers.status,
            retained.input.offers.entries.len(),
            all.len(),
        );
        Ok(RewardLiveOfferPage {
            snapshot: query.snapshot,
            entries,
            total: all.len(),
            status,
            continuation,
        })
    }

    /// Gets one exact visible live offer and its resolved static definition and nested records.
    pub fn get_offer(
        &self,
        reference: &RewardLiveOfferReference,
        scope: RewardVisibilityScope,
    ) -> Result<RewardLiveOfferDetail, RewardLiveError> {
        let retained = self.validate_offer_reference(reference)?;
        let offer = find_offer(&retained.input, reference.offer_id().as_str())
            .ok_or(RewardLiveError::NotFound)?;
        if !projection::visible(offer.visibility, scope) {
            return Err(RewardLiveError::NotFound);
        }
        detail::project_detail(offer, reference.clone(), scope, &self.catalog)
    }

    /// Gets one exact live item, including its static definition before acquisition.
    pub fn get_item(
        &self,
        reference: &RewardLiveItemReference,
        scope: RewardVisibilityScope,
    ) -> Result<RewardLiveItemDetail, RewardLiveError> {
        let retained = self.validate_offer_reference(reference.offer())?;
        let offer = find_offer(&retained.input, reference.offer().offer_id().as_str())
            .ok_or(RewardLiveError::NotFound)?;
        if !projection::visible(offer.visibility, scope) {
            return Err(RewardLiveError::NotFound);
        }
        let item = projection::find_item(offer, reference.item_id().as_str())
            .ok_or(RewardLiveError::NotFound)?;
        if !projection::visible(item.visibility, scope) {
            return Err(RewardLiveError::NotFound);
        }
        let group = match reference.group() {
            Some(group_ref) => {
                self.validate_offer_reference(group_ref.offer())?;
                if !same_offer(group_ref.offer(), reference.offer()) {
                    return Err(RewardLiveError::StaleReference);
                }
                let group = offer
                    .groups
                    .entries
                    .iter()
                    .find(|group| group.id.as_str() == group_ref.group_id().as_str())
                    .ok_or(RewardLiveError::NotFound)?;
                if !group
                    .items
                    .entries
                    .iter()
                    .any(|id| id.as_str() == reference.item_id().as_str())
                    || !projection::visible(group.visibility, scope)
                {
                    return Err(RewardLiveError::NotFound);
                }
                Some(group_ref.clone())
            }
            None => None,
        };
        detail::project_item(item, reference.offer(), group, scope, &self.catalog)
    }

    /// Gets one exact host-observed action by its live group-scoped identity.
    pub fn get_action(
        &self,
        reference: &RewardLiveActionReference,
        scope: RewardVisibilityScope,
    ) -> Result<RewardLiveActionDetail, RewardLiveError> {
        let group = reference.group();
        self.validate_offer_reference(group.offer())?;
        let detail = self.get_offer(group.offer(), scope)?;
        let group_detail = detail
            .groups
            .entries
            .iter()
            .find(|entry| entry.reference.group_id() == group.group_id())
            .ok_or(RewardLiveError::NotFound)?;
        group_detail
            .actions
            .entries
            .iter()
            .find(|action| action.reference.action_id() == reference.action_id())
            .cloned()
            .ok_or(RewardLiveError::NotFound)
    }

    /// Resolves a live offer's optional static reward definition through the retained catalog.
    pub fn resolve_definition(
        &self,
        reference: &RewardLiveOfferReference,
        scope: RewardVisibilityScope,
    ) -> Result<RewardField<RewardOfferDefinition>, RewardLiveError> {
        Ok(self.get_offer(reference, scope)?.definition)
    }

    /// Resolves a live item's optional static item definition through the retained catalog.
    pub fn resolve_item_definition(
        &self,
        reference: &RewardLiveItemReference,
        scope: RewardVisibilityScope,
    ) -> Result<RewardField<RewardItem>, RewardLiveError> {
        Ok(self.get_item(reference, scope)?.definition)
    }

    fn validate_offer_reference(
        &self,
        reference: &RewardLiveOfferReference,
    ) -> Result<&RetainedSnapshot, RewardLiveError> {
        self.validate_snapshot_reference(reference.snapshot())
    }

    fn validate_snapshot_reference(
        &self,
        reference: &RewardLiveSnapshotReference,
    ) -> Result<&RetainedSnapshot, RewardLiveError> {
        if !Arc::ptr_eq(&reference.scope, &self.scope) {
            return Err(RewardLiveError::WrongReader);
        }
        let current = self
            .current
            .as_ref()
            .ok_or(RewardLiveError::StaleReference)?;
        if reference.reader_generation != current.reader_generation
            || reference.reader_generation != self.generation
            || !Arc::ptr_eq(&reference.fence, &current.fence)
        {
            return Err(RewardLiveError::StaleReference);
        }
        Ok(current)
    }

    pub(super) fn snapshot_reference(
        &self,
        fence: Arc<RewardLiveSnapshotFence>,
        reader_generation: u64,
    ) -> RewardLiveSnapshotReference {
        RewardLiveSnapshotReference::new(fence, Arc::clone(&self.scope), reader_generation)
    }
}

fn find_offer<'a>(
    input: &'a super::snapshot::RewardLiveSnapshotInput,
    id: &str,
) -> Option<&'a RewardLiveOffer> {
    input
        .offers
        .entries
        .iter()
        .find(|offer| offer.id.as_str() == id)
}

fn same_offer(left: &RewardLiveOfferReference, right: &RewardLiveOfferReference) -> bool {
    Arc::ptr_eq(&left.snapshot.fence, &right.snapshot.fence)
        && left.snapshot.reader_generation == right.snapshot.reader_generation
        && left.id == right.id
}

fn projected_status(source: RewardFieldStatus, total: usize, visible: usize) -> RewardFieldStatus {
    if source != RewardFieldStatus::Available {
        source
    } else if total == visible {
        RewardFieldStatus::Available
    } else if visible == 0 {
        RewardFieldStatus::Denied
    } else {
        RewardFieldStatus::Partial
    }
}
