// SPDX-License-Identifier: MIT

//! Exhaustive coverage for the coarse field-status mapping
//! (`sts2-game-mod#84`).
//!
//! `ActionUnavailableReason::status` is the boundary that decides whether an
//! unavailable field is reported as absent, unsupported, denied, withheld or
//! genuinely unknown. Every one of those distinctions exists so a consumer can
//! tell "there is no value here" apart from "the host could not classify this
//! value". Collapsing any of them into `Available` would report an
//! unobservable field as an observed one.
//!
//! These are deterministic, host-free tests over the mapping itself. No native
//! game, no extractor and no content manifest is required, and none is
//! claimed.

#![allow(clippy::expect_used, dead_code)]

use sts2_game_mod::{ActionField, ActionFieldStatus, ActionText, ActionUnavailableReason};

/// Every reason the host can report for a field it did not produce.
const ALL_REASONS: [ActionUnavailableReason; 9] = [
    ActionUnavailableReason::NotObserved,
    ActionUnavailableReason::Unsupported,
    ActionUnavailableReason::NotIntegrated,
    ActionUnavailableReason::Undescribed,
    ActionUnavailableReason::Denied,
    ActionUnavailableReason::Failed,
    ActionUnavailableReason::Unknown,
    ActionUnavailableReason::NotApplicable,
    ActionUnavailableReason::Withheld,
];

#[test]
fn every_unavailable_reason_maps_to_its_documented_status() {
    let expected = [
        (
            ActionUnavailableReason::NotObserved,
            ActionFieldStatus::NotObserved,
        ),
        (
            ActionUnavailableReason::Unsupported,
            ActionFieldStatus::Unsupported,
        ),
        (
            ActionUnavailableReason::NotIntegrated,
            ActionFieldStatus::Unsupported,
        ),
        (
            ActionUnavailableReason::Undescribed,
            ActionFieldStatus::Unsupported,
        ),
        (ActionUnavailableReason::Denied, ActionFieldStatus::Denied),
        (ActionUnavailableReason::Failed, ActionFieldStatus::Failed),
        (ActionUnavailableReason::Unknown, ActionFieldStatus::Unknown),
        (
            ActionUnavailableReason::NotApplicable,
            ActionFieldStatus::NotApplicable,
        ),
        (
            ActionUnavailableReason::Withheld,
            ActionFieldStatus::Withheld,
        ),
    ];

    for (reason, status) in expected {
        assert_eq!(
            reason.status(),
            status,
            "{reason:?} must report as {status:?}"
        );
    }
}

/// The load-bearing property: no reason may ever be reported as `Available`.
///
/// This is the assertion that fails if any arm of the mapping is rewritten to
/// `Available`, which is the specific corruption this file exists to catch.
#[test]
fn no_unavailable_reason_is_ever_reported_as_available() {
    for reason in ALL_REASONS {
        assert_ne!(
            reason.status(),
            ActionFieldStatus::Available,
            "{reason:?} reported an unobserved field as observed"
        );
    }
}

/// The mapping must also be total and deterministic across repeated calls.
#[test]
fn the_status_mapping_is_stable_across_repeated_calls() {
    for reason in ALL_REASONS {
        assert_eq!(reason.status(), reason.status());
    }
}

/// An unclassifiable field stays unavailable through the `ActionField` wrapper
/// rather than yielding a value.
#[test]
fn an_unknown_field_yields_no_value_and_reports_unknown() {
    let field: ActionField<u32> = ActionField::unavailable(ActionUnavailableReason::Unknown);

    assert_eq!(field.status(), ActionFieldStatus::Unknown);
    assert_eq!(field.value(), None);
    assert_eq!(field.reason(), Some(ActionUnavailableReason::Unknown));
}

/// The same holds for localized text, so a withheld or unclassifiable string
/// cannot be read as empty text.
#[test]
fn unknown_text_yields_no_text_and_reports_unknown() {
    let text = ActionText::Unavailable(ActionUnavailableReason::Unknown);

    assert_eq!(text.status(), ActionFieldStatus::Unknown);
    assert_eq!(text.value(), None);
}

/// A genuinely observed value is the only thing that reports `Available`.
#[test]
fn only_an_observed_value_reports_available() {
    let observed: ActionField<u32> = ActionField::available(7);
    let unavailable: ActionField<u32> = ActionField::unavailable(ActionUnavailableReason::Unknown);

    assert_eq!(observed.status(), ActionFieldStatus::Available);
    assert_eq!(observed.value(), Some(&7));
    assert_ne!(unavailable.status(), ActionFieldStatus::Available);
}
