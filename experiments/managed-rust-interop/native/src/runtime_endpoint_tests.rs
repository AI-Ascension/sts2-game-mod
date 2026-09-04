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
