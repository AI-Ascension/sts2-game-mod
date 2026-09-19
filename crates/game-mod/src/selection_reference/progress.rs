// SPDX-License-Identifier: MIT

use super::{
    candidate::SelectionBlockReason,
    definition::SelectionPickRule,
    field::{SelectionField, SelectionFieldStatus, SelectionText},
    model::{
        SelectionCancellation, SelectionCandidateReference, SelectionConfirmation, SelectionKind,
        SelectionNextDomain, SelectionParentOperation, SelectorGenerationReference,
    },
};

/// Picks a caller claims to have made for one selector generation.
///
/// The value is a caller observation, not a host fact: every reader method that accepts it
/// reconciles it against the catalog and refuses a selector generation, a candidate, or a pick
/// count the catalog does not support.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionProgressInput {
    /// Selector generation the caller observed.
    pub selector: SelectorGenerationReference,
    /// Candidate identities the caller picked, in order.
    pub selected: Vec<String>,
}

/// Whether the current picks may be confirmed, and what still stands in the way.
///
/// A prompt that may not yet be confirmed reports the shortfall instead of appearing confirmable,
/// so an early confirmation is never represented as a legal one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionConfirmationState {
    /// Whether the prompt may be confirmed with the picks made so far.
    pub may_confirm: bool,
    /// Whether the prompt offers a cancel or back step.
    pub may_cancel: bool,
    /// Picks still required before confirming, when a shortfall remains.
    pub unmet: SelectionField<u32>,
    /// Why the prompt may not be confirmed, when it may not be.
    pub reason: SelectionField<SelectionBlockReason>,
}

/// Reconciled progress of one selector generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionProgress {
    /// Selector generation this progress describes.
    pub selector: SelectorGenerationReference,
    /// Operation whose prompt owns this selector.
    pub parent: SelectionParentOperation,
    /// Exact-build selection family.
    pub kind: SelectionKind,
    /// Localized prompt text.
    pub prompt: SelectionText,
    /// Required and bounded pick counts.
    pub picks: SelectionPickRule,
    /// Candidates the caller picked, resolved against the catalog.
    pub selected: Vec<SelectionCandidateReference>,
    /// Number of picks the caller made.
    pub picked: usize,
    /// Picks still available, or an explicit non-value when the maximum is unknown.
    pub remaining: SelectionField<u32>,
    /// Candidates still selectable under the duplicate rule and scope.
    pub available: usize,
    /// Availability of the candidate list after scope withholding.
    pub candidates_status: SelectionFieldStatus,
    /// How the prompt completes once its picks are satisfied.
    pub confirmation: SelectionConfirmation,
    /// How the prompt may be abandoned, or an explicit non-value.
    pub cancellation: SelectionField<SelectionCancellation>,
    /// Whether the picks may be confirmed yet.
    pub confirmation_state: SelectionConfirmationState,
    /// Domain presented once the picks are complete, or an explicit non-value.
    pub next: SelectionField<SelectionNextDomain>,
    /// Whether every required pick has been made.
    pub complete: bool,
}

/// Returns the picks a selector requires before it may complete.
///
/// A required selector always requires at least one pick: a stated minimum is used as given, and a
/// required selector that states no minimum still requires one, so a required prompt can never be
/// confirmed with nothing picked.
pub(super) fn required_picks(rule: &SelectionPickRule) -> usize {
    match rule.minimum_picks() {
        Some(minimum) => minimum as usize,
        None if rule.required => 1,
        None => 0,
    }
}

/// Returns the picks still available for one maximum, or an explicit non-value.
pub(super) fn remaining_picks(rule: &SelectionPickRule, picked: usize) -> SelectionField<u32> {
    match rule.maximum_picks() {
        Some(maximum) => SelectionField::available(maximum.saturating_sub(picked as u32)),
        None => SelectionField::unavailable(super::field::SelectionUnavailableReason::NotObserved),
    }
}

/// Returns the confirmation state for one set of picks.
///
/// A shortfall is reported as an explicit unmet count rather than as a confirmable prompt.
pub(super) fn confirmation_state(
    rule: &SelectionPickRule,
    cancellation: &SelectionField<SelectionCancellation>,
    picked: usize,
) -> SelectionConfirmationState {
    let required = required_picks(rule);
    let shortfall = required.saturating_sub(picked);
    let may_cancel = cancellation.is_available();
    if shortfall == 0 {
        return SelectionConfirmationState {
            may_confirm: true,
            may_cancel,
            unmet: SelectionField::available(0),
            reason: SelectionField::unavailable(
                super::field::SelectionUnavailableReason::NotApplicable,
            ),
        };
    }
    SelectionConfirmationState {
        may_confirm: false,
        may_cancel,
        unmet: SelectionField::available(shortfall as u32),
        reason: SelectionField::available(SelectionBlockReason::RequirementUnsatisfied),
    }
}
