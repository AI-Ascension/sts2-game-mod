// SPDX-License-Identifier: MIT

use super::RuntimeRequest;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU32, Ordering};
use sts2_protocol::game_facts_reference_v1::{Binding, BindingMode, MessageKind, Query};

static KIND: AtomicU32 = AtomicU32::new(0);

unsafe extern "C" fn callback(
    request: *const RuntimeRequest,
    output: *mut u8,
    capacity: usize,
    length: *mut usize,
) -> i32 {
    if request.is_null() || output.is_null() || length.is_null() || capacity < 2 {
        return 500;
    }
    // SAFETY: The production dispatcher lends these buffers for this synchronous callback.
    unsafe {
        KIND.store((*request).kind, Ordering::SeqCst);
        std::ptr::copy_nonoverlapping(b"{}".as_ptr(), output, 2);
        *length = 2;
    }
    200
}

fn round_trip(
    method: &str,
    path: &str,
    body: &[u8],
    auth: &str,
    locale: &str,
    content_type: &str,
) -> std::io::Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let mut client = TcpStream::connect(listener.local_addr()?)?;
    client.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
    let (mut server, _) = listener.accept()?;
    write!(
        client,
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nAuthorization: {auth}\r\n\
        Content-Type: {content_type}\r\nContent-Length: {}\r\n\
        x-sts2-instance-id: instance-1\r\nx-sts2-caller-id: caller-1\r\n\
        x-sts2-session-id: session-1\r\nx-sts2-lease-id: lease-1\r\n\
        x-sts2-lease-epoch: 1\r\nx-sts2-correlation-id: corr-1\r\n{locale}\r\n",
        body.len()
    )?;
    client.write_all(body)?;
    let stop = std::sync::atomic::AtomicBool::new(false);
    super::connection::handle_connection(
        &mut super::io::Connection::new(&mut server, &stop, std::time::Duration::from_secs(2)),
        "127.0.0.1:0",
        b"token",
        callback,
    )?;
    drop(server);
    let mut response = String::new();
    client.read_to_string(&mut response)?;
    Ok(response)
}

#[test]
fn fixed_facts_routes_authenticate_and_reject_method_path_body_and_locale_mismatches()
-> Result<(), Box<dyn std::error::Error>> {
    let caps = "/api/v1/game-information/facts/capabilities";
    let query = "/api/v1/game-information/facts/query";
    let locale = "x-sts2-locale: en-US\r\n";
    let mut request = sts2_game_mod::LiveFixedCardFacts::capabilities("corr-1")?;
    request.kind = MessageKind::QueryRequest;
    request.capabilities = None;
    request.query = Some(Query {
        rules_reference_version: 2,
        rule_ids: vec!["resource.fixed_card_cost".to_owned()],
        binding: Binding {
            mode: BindingMode::Static,
            content_manifest_id: "manifest-1".to_owned(),
            locale: "en-US".to_owned(),
            visibility_scope: None,
            instance_ref: None,
            snapshot_ref: None,
        },
        scope: None,
        parent_observation: None,
    });
    let body = sts2_protocol::game_facts_reference_v1::GameFactsReferenceV1Codec::encode(&request)?;
    for (method, path, payload, auth, lang, media, status, kind) in [
        (
            "GET",
            caps,
            b"".as_slice(),
            "Bearer wrong",
            locale,
            "application/json",
            401,
            0,
        ),
        (
            "POST",
            query,
            body.as_slice(),
            "Bearer wrong",
            locale,
            "application/json",
            401,
            0,
        ),
        (
            "GET",
            caps,
            b"".as_slice(),
            "Bearer token",
            locale,
            "application/json",
            200,
            26,
        ),
        (
            "POST",
            query,
            body.as_slice(),
            "Bearer token",
            locale,
            "application/json",
            200,
            27,
        ),
        (
            "POST",
            caps,
            b"".as_slice(),
            "Bearer token",
            locale,
            "application/json",
            404,
            0,
        ),
        (
            "GET",
            caps,
            b"{}".as_slice(),
            "Bearer token",
            locale,
            "application/json",
            404,
            0,
        ),
        (
            "GET",
            query,
            b"".as_slice(),
            "Bearer token",
            locale,
            "application/json",
            404,
            0,
        ),
        (
            "POST",
            query,
            body.as_slice(),
            "Bearer token",
            locale,
            "text/plain",
            404,
            0,
        ),
        (
            "POST",
            query,
            b"{}".as_slice(),
            "Bearer token",
            locale,
            "application/json",
            400,
            0,
        ),
        (
            "GET",
            caps,
            b"".as_slice(),
            "Bearer token",
            "",
            "application/json",
            400,
            0,
        ),
        (
            "GET",
            "/api/v1/game-information/facts/capabilities/extra",
            b"".as_slice(),
            "Bearer token",
            locale,
            "application/json",
            404,
            0,
        ),
        (
            "POST",
            "/api/v1/game-information/facts/query/extra",
            body.as_slice(),
            "Bearer token",
            locale,
            "application/json",
            404,
            0,
        ),
    ] {
        KIND.store(0, Ordering::SeqCst);
        let response = round_trip(method, path, payload, auth, lang, media)?;
        assert!(
            response.starts_with(&format!("HTTP/1.1 {status}")),
            "{method} {path}: {response}"
        );
        assert_eq!(KIND.load(Ordering::SeqCst), kind);
    }
    Ok(())
}
