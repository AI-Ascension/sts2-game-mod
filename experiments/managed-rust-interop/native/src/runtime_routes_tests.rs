// SPDX-License-Identifier: MIT

use super::super::{RuntimeRequest, RuntimeRequestCallback, endpoint_tests};
use super::{dispatch, http};
use serde_json::Value;
use std::io::Read;
use std::net::{TcpListener, TcpStream};

#[test]
fn admitted_slash_identity_reaches_operation_lookup_unchanged() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let mut client = TcpStream::connect(listener.local_addr()?)?;
    client.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
    let (mut server, _) = listener.accept()?;
    let request = http::Request {
        method: String::from("GET"),
        path: String::from("/api/v2/runtime/operations/run/operation"),
        headers: [
            "x-sts2-instance-id",
            "x-sts2-caller-id",
            "x-sts2-session-id",
            "x-sts2-lease-id",
            "x-sts2-lease-epoch",
            "x-sts2-correlation-id",
        ]
        .into_iter()
        .map(|name| (name.to_owned(), String::from("1")))
        .collect(),
        body: Vec::new(),
    };
    let stop = std::sync::atomic::AtomicBool::new(false);
    dispatch(
        echo_operation,
        &request,
        "127.0.0.1:0",
        &mut super::super::io::Connection::new(
            &mut server,
            &stop,
            std::time::Duration::from_secs(2),
        ),
    )?;
    drop(server);
    let mut response = String::new();
    client.read_to_string(&mut response)?;
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.ends_with("run/operation"));
    Ok(())
}

#[test]
fn live_observation_bootstrap_route_reaches_its_callback() -> std::io::Result<()> {
    let body = b"{}";
    let request = format!(
        concat!(
            "POST /api/v1/game-information/live-observation-bootstrap HTTP/1.1\r\n",
            "Authorization: Bearer synthetic\r\n",
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            "X-Sts2-Instance-Id: instance-1\r\nX-Sts2-Caller-Id: caller-1\r\n",
            "X-Sts2-Session-Id: session-1\r\nX-Sts2-Lease-Id: lease-1\r\n",
            "X-Sts2-Lease-Epoch: 1\r\nX-Sts2-Correlation-Id: corr-1\r\n",
            "X-Sts2-Locale: en-US\r\n\r\n"
        ),
        body.len()
    );
    let mut request = request.into_bytes();
    request.extend_from_slice(body);
    let response = endpoint_tests::exchange(&request, echo_operation)?;
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.ends_with("{}"));
    Ok(())
}

#[test]
fn live_observation_bootstrap_route_rejects_wrong_method_and_extra_segment() -> std::io::Result<()>
{
    for method_path in [
        ("GET", "/api/v1/game-information/live-observation-bootstrap"),
        (
            "POST",
            "/api/v1/game-information/live-observation-bootstrap/extra",
        ),
    ] {
        let body = if method_path.0 == "POST" { "{}" } else { "" };
        let request = format!(
            concat!(
                "{} {} HTTP/1.1\r\nAuthorization: Bearer synthetic\r\n",
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                "X-Sts2-Instance-Id: instance-1\r\nX-Sts2-Caller-Id: caller-1\r\n",
                "X-Sts2-Session-Id: session-1\r\nX-Sts2-Lease-Id: lease-1\r\n",
                "X-Sts2-Lease-Epoch: 1\r\nX-Sts2-Correlation-Id: corr-1\r\n",
                "X-Sts2-Locale: en-US\r\n\r\n{}"
            ),
            method_path.0,
            method_path.1,
            body.len(),
            body
        );
        let response = endpoint_tests::exchange(request.as_bytes(), unexpected_callback)?;
        assert!(response.starts_with("HTTP/1.1 404"));
    }
    Ok(())
}

#[test]
fn exact_restore_routes_refuse_before_managed_callback_or_staging()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../protocol-artifact/exact-restore-v1/golden/frames.json"
    ))?;
    for (kind, path, expected_status, expected_outcome, expected_error) in [
        (
            "exact_restore_begin_request",
            "/v1/exact-restore/begin",
            409,
            "REJECTED",
            "no_restore_adapter",
        ),
        (
            "exact_restore_chunk_request",
            "/v1/exact-restore/chunk",
            503,
            "UNAVAILABLE",
            "native_unavailable",
        ),
        (
            "exact_restore_finish_blob_request",
            "/v1/exact-restore/finish",
            503,
            "UNAVAILABLE",
            "native_unavailable",
        ),
        (
            "exact_restore_commit_request",
            "/v1/exact-restore/commit",
            503,
            "UNAVAILABLE",
            "native_unavailable",
        ),
        (
            "exact_restore_lookup_request",
            "/v1/exact-restore/lookup",
            503,
            "UNAVAILABLE",
            "native_unavailable",
        ),
    ] {
        let frame = fixture["frames"]
            .as_array()
            .and_then(|frames| {
                frames
                    .iter()
                    .find(|frame| frame["kind"].as_str() == Some(kind))
            })
            .ok_or("missing exact-restore golden request")?;
        let owner = &frame["payload"]["expected_owner"];
        let correlation_id = frame["correlation_id"]
            .as_str()
            .ok_or("missing correlation id")?;
        let response = send_http_request(path, frame, owner, correlation_id, unexpected_callback)?;
        let (headers, response_body) = response
            .split_once("\r\n\r\n")
            .ok_or("missing HTTP response body")?;
        assert!(headers.starts_with(&format!("HTTP/1.1 {expected_status} ")));
        assert!(response_body.len() <= 16_384);
        let response: Value = serde_json::from_str(response_body)?;
        assert_eq!(response["kind"], "exact_restore_error_response");
        assert_eq!(response["correlation_id"], frame["message_id"]);
        assert_eq!(
            response["payload"]["operation_id"],
            frame["payload"]["operation_id"]
        );
        assert_eq!(response["payload"]["outcome"], expected_outcome);
        assert_eq!(response["payload"]["error_code"], expected_error);
        assert_eq!(response["payload"]["host_effect"], "not_started");
    }

    let wrong_phase = fixture["frames"]
        .as_array()
        .and_then(|frames| {
            frames
                .iter()
                .find(|frame| frame["kind"].as_str() == Some("exact_restore_chunk_request"))
        })
        .ok_or("missing chunk request")?;
    let owner = &wrong_phase["payload"]["expected_owner"];
    let response = send_http_request(
        "/v1/exact-restore/begin",
        wrong_phase,
        owner,
        wrong_phase["correlation_id"]
            .as_str()
            .ok_or("missing correlation")?,
        unexpected_callback,
    )?;
    let (headers, response_body) = response
        .split_once("\r\n\r\n")
        .ok_or("missing HTTP response body")?;
    assert!(headers.starts_with("HTTP/1.1 400 "));
    let response: Value = serde_json::from_str(response_body)?;
    assert_eq!(response["error_code"], "invalid_exact_restore_frame");
    Ok(())
}

fn send_http_request(
    path: &str,
    frame: &Value,
    owner: &Value,
    correlation: &str,
    callback: RuntimeRequestCallback,
) -> Result<String, Box<dyn std::error::Error>> {
    let body = serde_json::to_vec(frame)?;
    let request = format!(
        concat!(
            "POST {path} HTTP/1.1\r\nAuthorization: Bearer synthetic\r\n",
            "Content-Type: application/json\r\nContent-Length: {length}\r\n",
            "X-Sts2-Instance-Id: {instance}\r\nX-Sts2-Caller-Id: harness\r\n",
            "X-Sts2-Session-Id: {session}\r\nX-Sts2-Lease-Id: {lease}\r\n",
            "X-Sts2-Lease-Epoch: {epoch}\r\n",
            "X-Sts2-Correlation-Id: {correlation}\r\n\r\n"
        ),
        path = path,
        length = body.len(),
        instance = owner["instance_id"].as_str().ok_or("missing instance")?,
        session = owner["session_id"].as_str().ok_or("missing session")?,
        lease = owner["lease_id"].as_str().ok_or("missing lease")?,
        epoch = owner["lease_epoch"].as_u64().ok_or("missing epoch")?,
        correlation = correlation,
    );
    let mut request = request.into_bytes();
    request.extend_from_slice(&body);
    Ok(endpoint_tests::exchange(&request, callback)?)
}

unsafe extern "C" fn echo_operation(
    request: *const RuntimeRequest,
    output: *mut u8,
    capacity: usize,
    length: *mut usize,
) -> i32 {
    // SAFETY: dispatch provides live request/output pointers and the bounded output capacity.
    let request = unsafe { &*request };
    if request.body_len > capacity {
        return 500;
    }
    // SAFETY: both buffers are owned by dispatch and are disjoint for body_len bytes.
    unsafe {
        std::ptr::copy_nonoverlapping(request.body, output, request.body_len);
        *length = request.body_len;
    }
    200
}

unsafe extern "C" fn unexpected_callback(
    _: *const RuntimeRequest,
    _: *mut u8,
    _: usize,
    length: *mut usize,
) -> i32 {
    // SAFETY: Runtime dispatch supplies a live writable output length for the call.
    unsafe { length.write(0) };
    299
}
