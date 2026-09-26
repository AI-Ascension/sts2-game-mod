// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

use std::collections::BTreeMap;

use super::http;

fn headers(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

#[test]
fn a_rejected_header_is_named_in_the_refusal_so_a_recurrence_is_decidable() {
    // Without the name, `{"error_code":"unsupported_header"}` cannot say which
    // request was refused, so an occurrence costs a re-run to adjudicate
    // (AI-Ascension/sts2-game-mod#239, the game-mod half of the same defect
    // AI-Ascension/sts2-gateway#113 fixed at its own site).
    let offered = headers(&[("accept", "application/json")]);
    let refused = refused_header(&offered);
    assert_eq!(
        refused,
        Some("accept"),
        "an unlisted header must be reported by name, not merely refused"
    );
    let body = http::unsupported_header_body(refused.expect("a refused header"));
    let value: serde_json::Value = serde_json::from_slice(&body).expect("the refusal body is JSON");
    assert_eq!(value["error_code"], "unsupported_header");
    assert_eq!(
        value["rejected_header"], "accept",
        "the refusal must name the header: {value}"
    );
}

#[test]
fn a_refusal_never_echoes_a_header_value() {
    // The value may be a credential, so only the name crosses the boundary.
    let secret = "Bearer super-secret-token-value";
    let offered = headers(&[("x-unlisted-credential", secret)]);
    let refused = refused_header(&offered);
    let body = http::unsupported_header_body(refused.expect("a refused header"));
    let rendered = String::from_utf8(body).expect("the refusal body is UTF-8");
    assert!(
        !rendered.contains(secret),
        "a header value leaked into the refusal: {rendered}"
    );
    assert!(
        rendered.contains("x-unlisted-credential"),
        "the refusal must still name the header: {rendered}"
    );
}

#[test]
fn a_crafted_header_name_cannot_forge_the_refusal_body() {
    // The name arrives off the wire, so it is escaped rather than interpolated
    // raw: without escaping, this name closes the JSON early and injects fields.
    let injected = r#"x"},"injected":"yes"#;
    let offered = headers(&[(injected, "value")]);
    let refused = refused_header(&offered);
    let body = http::unsupported_header_body(refused.expect("a refused header"));
    let value: serde_json::Value =
        serde_json::from_slice(&body).expect("the refusal body stays one JSON object");
    assert_eq!(value["error_code"], "unsupported_header");
    assert!(
        value.get("injected").is_none(),
        "a header name forged a field: {value}"
    );
    assert_eq!(value["rejected_header"], injected);
}

#[test]
fn every_listed_header_is_still_accepted_so_the_allowlist_is_unchanged() {
    // Naming the refusal must not have widened or narrowed the allowlist: all
    // twelve names the previous `headers_are_allowed` matched are still matched.
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
            refused_header(&once(name, "value")),
            None,
            "{name} is listed and must not be refused"
        );
    }
}

#[test]
fn a_request_carrying_only_listed_headers_is_never_refused() {
    let all_listed = headers(&[
        ("authorization", "Bearer synthetic"),
        ("content-length", "0"),
        ("content-type", "application/json"),
        ("host", "127.0.0.1:1234"),
        ("connection", "keep-alive"),
        ("x-sts2-instance-id", "instance"),
        ("x-sts2-caller-id", "caller"),
        ("x-sts2-session-id", "session"),
        ("x-sts2-lease-id", "lease"),
        ("x-sts2-lease-epoch", "1"),
        ("x-sts2-correlation-id", "request"),
        ("x-sts2-locale", "en"),
    ]);
    assert_eq!(refused_header(&all_listed), None);
}

fn refused_header(headers: &BTreeMap<String, String>) -> Option<&str> {
    http::first_rejected_header(headers)
}

fn once(name: &str, value: &str) -> BTreeMap<String, String> {
    headers(&[(name, value)])
}
