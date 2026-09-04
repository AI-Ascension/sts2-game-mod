// SPDX-License-Identifier: MIT

use std::sync::atomic::{AtomicUsize, Ordering};

static CALLS: AtomicUsize = AtomicUsize::new(0);
const ROUTES: [(&str, &str, &str); 6] = [
    ("GET", "state", "state_request"),
    ("GET", "legal-actions", "legal_actions_request"),
    ("POST", "action", "dispatch_action_request"),
    ("POST", "wait", "wait_request"),
    ("GET", "reobserve", "reobserve_request"),
    ("POST", "recover", "recover_request"),
];

fn request(method: &str, route: &str, body: &str) -> std::io::Result<String> {
    let wire = format!(
        concat!(
            "{} /api/v3/runtime/{} HTTP/1.1\r\n",
            "Authorization: Bearer synthetic\r\nContent-Type: application/json\r\n",
            "Content-Length: {}\r\nX-Sts2-Instance-Id: instance\r\n",
            "X-Sts2-Caller-Id: caller\r\nX-Sts2-Session-Id: session\r\n",
            "X-Sts2-Lease-Id: lease\r\nX-Sts2-Lease-Epoch: 1\r\n",
            "X-Sts2-Correlation-Id: correlation\r\n\r\n{}"
        ),
        method,
        route,
        body.len(),
        body
    );
    super::endpoint_tests::exchange(wire.as_bytes(), callback)
}

#[test]
fn gameplay_routes_admit_only_their_own_top_level_message_kind() -> std::io::Result<()> {
    for (method, route, expected_kind) in ROUTES {
        let before = CALLS.load(Ordering::SeqCst);
        let wrong_method = if method == "GET" { "POST" } else { "GET" };
        let body = format!(r#"{{"kind":"{expected_kind}"}}"#);
        assert!(request(wrong_method, route, &body)?.starts_with("HTTP/1.1 404 "));
        assert_eq!(CALLS.load(Ordering::SeqCst), before);
        for (_, _, supplied_kind) in ROUTES {
            let before = CALLS.load(Ordering::SeqCst);
            let body = format!(r#"{{"kind":"{supplied_kind}","other":{{"kind":"nested"}}}}"#);
            let response = request(method, route, &body)?;
            let matches = expected_kind == supplied_kind;
            assert!(
                response.starts_with(if matches {
                    "HTTP/1.1 204 "
                } else {
                    "HTTP/1.1 400 "
                }),
                "{method} {route}: {supplied_kind}: {response}"
            );
            assert_eq!(CALLS.load(Ordering::SeqCst), before + usize::from(matches));
        }
    }
    for body in [
        "",
        "null",
        "[]",
        "[\"state_request\"]",
        "{}",
        "{\"kind\":null}",
        "{\"kind\":5}",
        "{\"kind\":\"state_request\"",
        "{\"kind\":\"state_request\"} {}",
        "{\"other\":{\"kind\":\"state_request\"}}",
        "{\"kind\":\"state_request\",\"kind\":\"dispatch_action_request\"}",
        "{\"kind\":\"dispatch_action_request\",\"kind\":\"state_request\"}",
        "{\"kind\":\"state_request\",\"kind\":\"state_request\"}",
        r#"{"kind":"state_request","ki\u006ed":"state_request"}"#,
        r#"{"kind":"state_request","other":1,"other":2}"#,
    ] {
        let before = CALLS.load(Ordering::SeqCst);
        assert!(request("GET", "state", body)?.starts_with("HTTP/1.1 400 "));
        assert_eq!(CALLS.load(Ordering::SeqCst), before);
    }
    Ok(())
}

unsafe extern "C" fn callback(
    request: *const super::RuntimeRequest,
    _: *mut u8,
    _: usize,
    length: *mut usize,
) -> i32 {
    CALLS.fetch_add(1, Ordering::SeqCst);
    // SAFETY: dispatch provides valid borrowed pointers for the callback duration.
    unsafe {
        length.write(0);
        if (*request).kind == super::CALLBACK_GAMEPLAY {
            204
        } else {
            500
        }
    }
}
