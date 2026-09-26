// SPDX-License-Identifier: MIT

//! The closed-header-allow-list refusal path (AI-Ascension/sts2-game-mod#239).
//!
//! These live in a test file rather than inline in `runtime_http` so that
//! production module stays inside `repo-policy`'s size budget; the crate
//! already uses this `#[path]`-module shape for its other test suites.

use std::collections::BTreeMap;

use super::http::{first_rejected_header, json_unsupported_header};

fn headers(names: &[&str]) -> BTreeMap<String, String> {
    names
        .iter()
        .map(|name| ((*name).to_owned(), String::new()))
        .collect()
}

/// The rejected name, owned.
///
/// `first_rejected_header` borrows from the map it scans, so returning the
/// borrow straight out of a `headers(..)` temporary would not outlive the
/// call. Owning the answer keeps each assertion below a single expression.
fn rejected(names: &[&str]) -> Option<String> {
    first_rejected_header(&headers(names)).map(str::to_owned)
}

/// The refusal body, owned and rendered as text.
fn refusal_body(name: &str) -> String {
    let body = json_unsupported_header(name);
    String::from_utf8_lossy(&body).into_owned()
}

#[test]
fn admits_every_previously_listed_header() {
    // The allow-list widened by three negotiation headers; this pins the
    // twelve that were already listed so a future edit cannot quietly narrow
    // them. Before this change the count was twelve in total; it is fifteen
    // now.
    for name in [
        "authorization",
        "content-length",
        "content-type",
        "host",
        "connection",
        "x-sts2-instance-id",
        "x-sts2-caller-id",
        "x-sts2-session-id",
        "x-sts2-lease-id",
        "x-sts2-lease-epoch",
        "x-sts2-correlation-id",
        "x-sts2-locale",
    ] {
        assert_eq!(
            rejected(&[name]),
            None,
            "{name} was listed before this change and must stay listed"
        );
    }
}

#[test]
fn admits_the_negotiation_headers_standard_clients_send_unprompted() {
    // The regression this change exists for: `urllib` adds `accept-encoding`
    // automatically, with no way to opt out short of overriding the opener, so
    // a closed list that omits it refuses a correct client.
    // obs-vm-setup/capture/README.md records that probe returning 400 for a
    // working API.
    for name in ["accept", "accept-encoding", "idempotency-key"] {
        assert_eq!(
            rejected(&[name]),
            None,
            "{name} is sent unprompted by standard clients and must be admitted"
        );
    }
}

#[test]
fn a_urllib_shaped_request_is_now_admitted() {
    // The exact header set the recorded probe sent: the overlay's own headers
    // plus the two urllib adds on its own.
    let admitted = rejected(&[
        "host",
        "authorization",
        "content-type",
        "content-length",
        "x-sts2-instance-id",
        "x-sts2-caller-id",
        "x-sts2-session-id",
        "accept-encoding",
        "connection",
    ]);
    assert_eq!(admitted, None);
}

#[test]
fn an_unlisted_header_is_still_refused_and_is_named() {
    // The posture is unchanged: a header outside the list is still refused.
    // Only the refusal became legible.
    assert_eq!(
        rejected(&["x-not-admitted"]),
        Some("x-not-admitted".to_owned())
    );
}

#[test]
fn the_reported_name_is_deterministic_when_several_are_refused() {
    // `BTreeMap` iterates in key order, so a request carrying two unlisted
    // headers always names the same one and the answer does not drift between
    // runs of the same request.
    let refused = rejected(&["x-zebra", "x-alpha", "x-middle"]);
    assert_eq!(refused, Some("x-alpha".to_owned()));
    let reordered = rejected(&["x-middle", "x-alpha", "x-zebra"]);
    assert_eq!(refused, reordered);
}

#[test]
fn the_refusal_body_names_the_header_and_never_the_value() {
    // `authorization` is on the allow-list, so a refused request cannot
    // actually carry one. The refusal helper nonetheless takes only a name,
    // and that is the property worth pinning: a future edit that changed the
    // signature to take the whole request, or that formatted a value in
    // alongside the name, would put a credential in a response body. A value
    // is never an input here, so a secret cannot appear.
    let body = refusal_body("x-not-admitted");
    assert!(body.contains("\"error_code\":\"unsupported_header\""));
    assert!(body.contains("\"rejected_header\":\"x-not-admitted\""));
    assert!(!body.contains("Bearer "));
    assert!(!body.contains("x-sts2-peer-token"));
}

#[test]
fn a_name_carrying_json_delimiters_cannot_break_out_of_the_body() {
    // This listener does not constrain header names to the RFC 7230 token
    // charset at parse time -- `read_request` only checks that a name is
    // non-empty and not a duplicate -- so the serializer is the only barrier
    // between a client-supplied name and the JSON string. A name carrying a
    // quote, a backslash, or a brace must round-trip as data.
    let hostile = "x-evil\",\"injected\":\"yes";
    let body = refusal_body(hostile);
    let parsed = serde_json::from_str::<serde_json::Value>(&body);
    assert!(
        parsed.is_ok(),
        "the refusal body must stay parseable JSON; got {body}"
    );
    let parsed = parsed.unwrap_or(serde_json::Value::Null);
    assert_eq!(parsed["error_code"], "unsupported_header");
    assert_eq!(parsed["rejected_header"], hostile);
    assert!(
        parsed.get("injected").is_none(),
        "a client-supplied name must not introduce a sibling field"
    );
}
