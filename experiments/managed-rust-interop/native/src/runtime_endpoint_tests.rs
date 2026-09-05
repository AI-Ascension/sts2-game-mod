// SPDX-License-Identifier: MIT

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

fn exchange(request: &[u8], callback: super::RuntimeRequestCallback) -> std::io::Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let mut peer = TcpStream::connect(listener.local_addr()?)?;
    peer.set_read_timeout(Some(Duration::from_secs(2)))?;
    peer.write_all(request)?;
    let (mut stream, _) = listener.accept()?;
    let stop = AtomicBool::new(false);
    let mut connection = super::io::Connection::new(&mut stream, &stop, Duration::from_secs(1));
    super::handle_connection(&mut connection, "127.0.0.1:1234", b"synthetic", callback)?;
    drop(stream);
    let mut response = String::new();
    peer.read_to_string(&mut response)?;
    Ok(response)
}

#[test]
fn bounded_connection_preserves_health_authentication() -> std::io::Result<()> {
    let missing = exchange(b"GET /health/ready HTTP/1.1\r\n\r\n", invalid_callback)?;
    assert!(missing.starts_with("HTTP/1.1 401 "));
    let wrong = exchange(
        b"GET /health/ready HTTP/1.1\r\nAuthorization: Bearer wrong\r\n\r\n",
        invalid_callback,
    )?;
    assert!(wrong.starts_with("HTTP/1.1 401 "));
    let valid = exchange(
        b"GET /health/ready HTTP/1.1\r\nAuthorization: Bearer synthetic\r\n\r\n",
        invalid_callback,
    )?;
    assert!(valid.starts_with("HTTP/1.1 200 "));
    assert!(valid.contains("\"status\":\"ready\""));
    Ok(())
}

#[test]
fn callback_cannot_claim_bytes_beyond_owned_output() -> std::io::Result<()> {
    let response = exchange(
        concat!(
            "GET /api/v1/runtime/state HTTP/1.1\r\n",
            "Authorization: Bearer synthetic\r\n",
            "X-Sts2-Instance-Id: instance\r\nX-Sts2-Caller-Id: caller\r\n",
            "X-Sts2-Session-Id: session\r\nX-Sts2-Lease-Id: lease\r\n",
            "X-Sts2-Lease-Epoch: 1\r\nX-Sts2-Correlation-Id: request\r\n\r\n"
        )
        .as_bytes(),
        invalid_callback,
    )?;
    assert!(response.starts_with("HTTP/1.1 500 "));
    assert!(response.ends_with("{\"error_code\":\"callback_failed\"}"));
    Ok(())
}

#[test]
fn v2_routes_preserve_callback_ids_without_admitting_legacy_v3() -> std::io::Result<()> {
    for (method, path, body, expected) in [
        ("GET", "/api/v2/runtime/state", "", 203),
        ("POST", "/api/v2/runtime/action", "{}", 204),
        ("GET", "/api/v2/runtime/operations/run/operation", "", 205),
        ("GET", "/api/v3/runtime/state", "", 404),
        ("POST", "/api/v3/runtime/action", "{}", 404),
        ("GET", "/api/v3/runtime/operations/run/operation", "", 404),
    ] {
        let request = format!(
            concat!(
                "{method} {path} HTTP/1.1\r\nAuthorization: Bearer synthetic\r\n",
                "Content-Type: application/json\r\nContent-Length: {length}\r\n",
                "X-Sts2-Instance-Id: instance\r\nX-Sts2-Caller-Id: caller\r\n",
                "X-Sts2-Session-Id: session\r\nX-Sts2-Lease-Id: lease\r\n",
                "X-Sts2-Lease-Epoch: 1\r\nX-Sts2-Correlation-Id: request\r\n\r\n{body}"
            ),
            method = method,
            path = path,
            length = body.len(),
            body = body,
        );
        let response = exchange(request.as_bytes(), callback_kind_status)?;
        assert!(
            response.starts_with(&format!("HTTP/1.1 {expected} ")),
            "{path}: {response}"
        );
    }
    Ok(())
}

unsafe extern "C" fn callback_kind_status(
    request: *const super::RuntimeRequest,
    _: *mut u8,
    _: usize,
    length: *mut usize,
) -> i32 {
    // SAFETY: dispatch owns the live aligned request and writable output length
    // for this synchronous call. Neither pointer is retained or aliased for writes.
    unsafe {
        length.write(0);
        200 + (*request).kind as i32
    }
}

unsafe extern "C" fn invalid_callback(
    _: *const super::RuntimeRequest,
    _: *mut u8,
    capacity: usize,
    length: *mut usize,
) -> i32 {
    // SAFETY: dispatch provides a live, aligned writable usize for this call.
    unsafe { length.write(capacity + 1) };
    200
}
