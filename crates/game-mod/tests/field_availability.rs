// SPDX-License-Identifier: MIT

#[path = "support/field_availability.rs"]
mod fixture;

use std::collections::BTreeMap;

use fixture::{field, reference, schema, store, store_with_detail_bytes};
use sts2_game_mod::{
    LocalBasicQuery, LocalCompleteness, LocalEntityFixture, LocalFieldStatus, LocalFieldValue,
    LocalKindFixture, LocalReadError, LocalReadReference, LocalReasonCode,
};

#[test]
fn outcomes_preserve_zero_empty_and_unavailable_distinctions() -> Result<(), LocalReadError> {
    let mut store = store(true)?;
    let page = store.read_basic(&LocalBasicQuery::all("synthetic_entity", 1))?;
    let item = page.entries.first().ok_or(LocalReadError::EntityNotFound)?;

    assert_eq!(
        field(&item.fields, "health")?.value(),
        Some(&LocalFieldValue::Integer(0))
    );
    assert_eq!(
        field(&item.fields, "tags")?.value(),
        Some(&LocalFieldValue::TextList(Vec::new()))
    );
    assert_eq!(
        field(&item.fields, "not_applicable")?.status(),
        LocalFieldStatus::NotApplicable
    );
    assert_eq!(
        field(&item.fields, "hidden")?.status(),
        LocalFieldStatus::NotObserved
    );
    assert_eq!(
        field(&item.fields, "unsupported")?.status(),
        LocalFieldStatus::Unsupported
    );
    assert_eq!(
        field(&item.fields, "denied")?.status(),
        LocalFieldStatus::Denied
    );
    assert_eq!(
        field(&item.fields, "failed")?.reason(),
        Some(LocalReasonCode::ExtractionFailed)
    );
    assert_eq!(
        field(&item.fields, "unknown")?.status(),
        LocalFieldStatus::Unknown
    );
    assert_eq!(
        field(&item.fields, "detail_text")?.reason(),
        Some(LocalReasonCode::SurfaceNotObserved)
    );
    assert_eq!(item.detail_links.len(), 1);
    assert_eq!(item.detail_links[0].field_group, "details");
    assert_eq!(page.origin.reference, reference());
    Ok(())
}

#[test]
fn unknown_and_protected_requests_fail_before_host_access() -> Result<(), LocalReadError> {
    let mut store = store(true)?;
    let unknown = LocalBasicQuery::new("synthetic_entity", ["arbitrary.reflection.path"], 1, None);
    assert_eq!(
        store.read_basic(&unknown),
        Err(LocalReadError::UnknownField)
    );
    let denied = LocalBasicQuery::new("synthetic_entity", ["denied"], 1, None);
    assert_eq!(store.read_basic(&denied), Err(LocalReadError::DeniedField));
    Ok(())
}

#[test]
fn detail_recovery_is_allowlisted_and_identity_bound() -> Result<(), LocalReadError> {
    let mut store = store(true)?;
    let page = store.read_basic(&LocalBasicQuery::new(
        "synthetic_entity",
        ["detail_text"],
        1,
        None,
    ))?;
    let link = page
        .entries
        .first()
        .and_then(|entry| entry.detail_links.first())
        .ok_or(LocalReadError::UnknownFieldGroup)?
        .clone();
    let detail = store.read_detail(&link)?;
    assert_eq!(
        field(&detail.fields, "detail_text")?.value(),
        Some(&LocalFieldValue::Text("recovered".to_owned()))
    );

    let mut stale = link.clone();
    stale.origin.reference.epoch = 6;
    assert_eq!(
        store.read_detail(&stale),
        Err(LocalReadError::StaleReference)
    );
    let mut wrong_kind = link.clone();
    wrong_kind.origin.source_kind = "other_kind".to_owned();
    assert_eq!(
        store.read_detail(&wrong_kind),
        Err(LocalReadError::StaleReference)
    );

    let mut arbitrary_group = link;
    arbitrary_group.field_group = "host.memory".to_owned();
    assert_eq!(
        store.read_detail(&arbitrary_group),
        Err(LocalReadError::UnknownFieldGroup)
    );
    Ok(())
}

#[test]
fn collection_pages_are_complete_only_after_opaque_continuation() -> Result<(), LocalReadError> {
    let mut store = store(true)?;
    let first = store.read_basic(&LocalBasicQuery::all("synthetic_entity", 1))?;
    assert_eq!(first.total, Some(2));
    assert_eq!(first.completeness, LocalCompleteness::Partial);
    let continuation = first
        .continuation
        .clone()
        .ok_or(LocalReadError::InvalidContinuation)?;
    assert_eq!(first.returned_count(), 1);

    let second = store.read_basic(&LocalBasicQuery::new(
        "synthetic_entity",
        std::iter::empty::<String>(),
        1,
        Some(continuation.clone()),
    ))?;
    assert_eq!(second.completeness, LocalCompleteness::Complete);
    assert!(second.continuation.is_none());
    assert_eq!(
        second.entries.first().map(|entry| entry.entity_id.as_str()),
        Some("entity-b")
    );
    assert_eq!(
        store.read_basic(&LocalBasicQuery::new(
            "synthetic_entity",
            std::iter::empty::<String>(),
            1,
            Some(continuation),
        )),
        Err(LocalReadError::InvalidContinuation)
    );
    Ok(())
}

#[test]
fn unknown_totals_stay_unknown_and_oversized_detail_is_typed() -> Result<(), LocalReadError> {
    let mut store = store(false)?;
    let first = store.read_basic(&LocalBasicQuery::all("synthetic_entity", 2))?;
    assert!(!first.count_known());
    assert_eq!(first.total, None);
    assert_eq!(first.completeness, LocalCompleteness::Complete);

    let second = store.read_basic(&LocalBasicQuery::all("synthetic_entity", 1))?;
    let continuation = second
        .continuation
        .clone()
        .ok_or(LocalReadError::InvalidContinuation)?;
    let final_page = store.read_basic(&LocalBasicQuery::new(
        "synthetic_entity",
        std::iter::empty::<String>(),
        1,
        Some(continuation),
    ))?;
    assert!(!final_page.count_known());
    assert_eq!(final_page.completeness, LocalCompleteness::Complete);

    let entity_b = store.read_basic(&LocalBasicQuery::new(
        "synthetic_entity",
        ["detail_text"],
        2,
        None,
    ))?;
    let link = entity_b
        .entries
        .get(1)
        .and_then(|entry| entry.detail_links.first())
        .ok_or(LocalReadError::UnknownFieldGroup)?
        .clone();
    assert_eq!(
        store.read_detail(&link),
        Err(LocalReadError::DetailTooLarge {
            limit: 64,
            actual: 128,
        })
    );
    Ok(())
}

#[test]
fn support_report_exposes_required_coverage() -> Result<(), LocalReadError> {
    let store = store(true)?;
    let report = store.support_report("synthetic_entity")?;
    assert_eq!(report.entity_count, 2);
    assert!(report.count_known);
    assert_eq!(report.required_fields.len(), 2);
    let health = report
        .required_fields
        .iter()
        .find(|field| field.field == "health")
        .ok_or(LocalReadError::UnknownField)?;
    assert_eq!(health.available_count, 2);
    assert_eq!(health.status, LocalFieldStatus::Available);
    let denied = report
        .required_fields
        .iter()
        .find(|field| field.field == "denied")
        .ok_or(LocalReadError::UnknownField)?;
    assert_eq!(denied.available_count, 0);
    assert_eq!(denied.status, LocalFieldStatus::Denied);
    assert_eq!(
        report
            .fields
            .iter()
            .find(|field| field.field == "unsupported")
            .map(|field| field.supported),
        Some(false)
    );
    Ok(())
}

#[test]
fn stale_reference_expires_cursors_and_detail_links() -> Result<(), LocalReadError> {
    let mut store = store(true)?;
    let first = store.read_basic(&LocalBasicQuery::all("synthetic_entity", 1))?;
    let continuation = first
        .continuation
        .clone()
        .ok_or(LocalReadError::InvalidContinuation)?;
    let link = first
        .entries
        .first()
        .and_then(|entry| entry.detail_links.first())
        .ok_or(LocalReadError::UnknownFieldGroup)?
        .clone();
    store.replace_reference(LocalReadReference::new("content-1", "snapshot-2", 8));
    assert_eq!(
        store.read_basic(&LocalBasicQuery::new(
            "synthetic_entity",
            std::iter::empty::<String>(),
            1,
            Some(continuation),
        )),
        Err(LocalReadError::InvalidContinuation)
    );
    assert_eq!(
        store.read_detail(&link),
        Err(LocalReadError::StaleReference)
    );
    Ok(())
}

#[test]
fn replacing_a_kind_expires_old_cursor_and_detail_identity() -> Result<(), LocalReadError> {
    let mut store = store(true)?;
    let first = store.read_basic(&LocalBasicQuery::all("synthetic_entity", 1))?;
    let continuation = first
        .continuation
        .clone()
        .ok_or(LocalReadError::InvalidContinuation)?;
    let link = first
        .entries
        .first()
        .and_then(|entry| entry.detail_links.first())
        .ok_or(LocalReadError::UnknownFieldGroup)?
        .clone();

    let replacement =
        LocalKindFixture::new(schema(), std::iter::empty::<LocalEntityFixture>(), true);
    store.add_kind(replacement)?;
    assert_eq!(
        store.read_basic(&LocalBasicQuery::new(
            "synthetic_entity",
            std::iter::empty::<String>(),
            1,
            Some(continuation),
        )),
        Err(LocalReadError::InvalidContinuation)
    );
    assert_eq!(
        store.read_detail(&link),
        Err(LocalReadError::StaleReference)
    );
    let fresh = store.read_basic(&LocalBasicQuery::all("synthetic_entity", 1))?;
    assert!(fresh.entries.is_empty());
    Ok(())
}

#[test]
fn detail_size_must_be_present_and_nonzero() -> Result<(), LocalReadError> {
    for detail_bytes in [BTreeMap::new(), BTreeMap::from([("details".to_owned(), 0)])] {
        let mut store = store_with_detail_bytes(detail_bytes)?;
        let page = store.read_basic(&LocalBasicQuery::new(
            "synthetic_entity",
            ["detail_text"],
            1,
            None,
        ))?;
        let link = page
            .entries
            .first()
            .and_then(|entry| entry.detail_links.first())
            .ok_or(LocalReadError::UnknownFieldGroup)?
            .clone();
        assert_eq!(
            store.read_detail(&link),
            Err(LocalReadError::DetailSizeUnavailable)
        );
    }
    Ok(())
}

#[test]
fn continuation_cannot_cross_store_scope_even_when_tokens_collide() -> Result<(), LocalReadError> {
    let mut first_store = store(true)?;
    let first_page = first_store.read_basic(&LocalBasicQuery::all("synthetic_entity", 1))?;
    let first_token = first_page
        .continuation
        .clone()
        .ok_or(LocalReadError::InvalidContinuation)?;

    let mut second_store = store(true)?;
    let second_page = second_store.read_basic(&LocalBasicQuery::all("synthetic_entity", 1))?;
    let second_token = second_page
        .continuation
        .clone()
        .ok_or(LocalReadError::InvalidContinuation)?;
    assert_eq!(first_token.token(), second_token.token());
    assert_eq!(
        second_store.read_basic(&LocalBasicQuery::new(
            "synthetic_entity",
            std::iter::empty::<String>(),
            1,
            Some(first_token),
        )),
        Err(LocalReadError::InvalidContinuation)
    );
    let second_page = second_store.read_basic(&LocalBasicQuery::new(
        "synthetic_entity",
        std::iter::empty::<String>(),
        1,
        Some(second_token),
    ))?;
    assert_eq!(
        second_page
            .entries
            .first()
            .map(|entry| entry.entity_id.as_str()),
        Some("entity-b")
    );
    Ok(())
}
