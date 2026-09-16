// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

use super::RuntimeRequest;
use serde_json::json;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use sts2_protocol::{
    GAME_INFORMATION_CONTENT_MANIFEST_V1_ARTIFACT,
    GAME_INFORMATION_CONTENT_MANIFEST_V1_PROTOCOL_VERSION,
    GAME_INFORMATION_CONTENT_MANIFEST_V1_SCHEMA_DIGEST,
    GAME_INFORMATION_CONTENT_MANIFEST_V1_SCHEMA_SOURCE, GameInformationContentManifestV1Codec,
};

type RecordedRequest = (u32, String, String, String, String, String, String, String);

static CONTENT_MANIFEST_REQUEST: Mutex<Option<RecordedRequest>> = Mutex::new(None);
static CONTENT_MANIFEST_RESPONSE: Mutex<Option<Vec<u8>>> = Mutex::new(None);
static CONTENT_MANIFEST_STATUS: Mutex<Option<i32>> = Mutex::new(None);
const CONTENT_MANIFEST_ERROR: &[u8] = br#"{"protocol_version":"game-information-content-manifest-v1","schema_digest":"416a39769445e6e462c5d5b5504f29010c255e2116a73094e55c7268e47f2ba6","provenance":{"artifact":"sts2-protocol/game-information-content-manifest-v1","source":"schemas/game-information-content-manifest-v1.schema.json","generator":"hand-authored"},"correlation_id":"corr-1","kind":"error_response","manifest":null,"error":{"code":"missing_capability","reason":"source_unavailable"}}"#;

fn success_manifest_body(definition_count: usize) -> Vec<u8> {
    let codec = GameInformationContentManifestV1Codec::new().expect("pinned protocol schema");
    let definitions = (0..definition_count)
        .map(|index| {
            json!({
                "entity_kind": "card",
                "namespaced_id": format!("base:card:{index:04}"),
                "handled": true,
                "origin": { "package_id": "base:game", "package_version": "0.107.1" },
                "override_chain": [],
                "semantic_revision": "a".repeat(64),
                "localized_text_revision": "b".repeat(64)
            })
        })
        .collect::<Vec<_>>();
    codec
        .encode(&json!({
            "protocol_version": GAME_INFORMATION_CONTENT_MANIFEST_V1_PROTOCOL_VERSION,
            "schema_digest": GAME_INFORMATION_CONTENT_MANIFEST_V1_SCHEMA_DIGEST,
            "provenance": {
                "artifact": GAME_INFORMATION_CONTENT_MANIFEST_V1_ARTIFACT,
                "source": GAME_INFORMATION_CONTENT_MANIFEST_V1_SCHEMA_SOURCE,
                "generator": "hand-authored"
            },
            "correlation_id": "corr-1",
            "kind": "content_manifest_response",
            "manifest": {
                "game_build": "test-build",
                "adapter_compatibility": "adapter-v1",
                "catalog_generation": 1,
                "locale": "en_US",
                "packages": [{
                    "package_id": "base:game",
                    "package_version": "0.107.1",
                    "order": 0
                }],
                "families": [{
                    "entity_kind": "card",
                    "handled": true,
                    "definition_count": definition_count
                }],
                "definitions": definitions,
                "content_set_revision": "c".repeat(64),
                "localized_text_revision": "d".repeat(64),
                "inventory_revision": "e".repeat(64)
            },
            "error": null
        }))
        .expect("valid bounded manifest fixture")
}

fn error_manifest_body() -> Vec<u8> {
    GameInformationContentManifestV1Codec::new()
        .expect("pinned protocol schema")
        .encode(&json!({
            "protocol_version": GAME_INFORMATION_CONTENT_MANIFEST_V1_PROTOCOL_VERSION,
            "schema_digest": GAME_INFORMATION_CONTENT_MANIFEST_V1_SCHEMA_DIGEST,
            "provenance": {
                "artifact": GAME_INFORMATION_CONTENT_MANIFEST_V1_ARTIFACT,
                "source": GAME_INFORMATION_CONTENT_MANIFEST_V1_SCHEMA_SOURCE,
                "generator": "hand-authored"
            },
            "correlation_id": "corr-1",
            "kind": "error_response",
            "manifest": null,
            "error": {
                "code": "result_limit_exceeded",
                "reason": "serialized_payload_too_large"
            }
        }))
        .expect("valid bounded error fixture")
}

unsafe fn read_request_text(pointer: *const u8, length: usize) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    // SAFETY: The caller uses a request field pointer and its matching dispatcher-owned length.
    let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };
    std::str::from_utf8(bytes).ok().map(str::to_owned)
}

unsafe extern "C" fn record_content_manifest(
    request: *const RuntimeRequest,
    output: *mut u8,
    capacity: usize,
    length: *mut usize,
) -> i32 {
    if request.is_null() || output.is_null() || length.is_null() {
        return 500;
    }
    // SAFETY: The route dispatcher passes the live request and output buffers for this call.
    let request = unsafe { &*request };
    // SAFETY: These pointers and lengths are dispatcher-owned for the callback duration.
    let (
        Some(instance_id),
        Some(caller_id),
        Some(session_id),
        Some(lease_id),
        Some(lease_epoch),
        Some(correlation_id),
        Some(locale),
    ) = (unsafe {
        (
            read_request_text(request.instance_id, request.instance_id_len),
            read_request_text(request.caller_id, request.caller_id_len),
            read_request_text(request.session_id, request.session_id_len),
            read_request_text(request.lease_id, request.lease_id_len),
            read_request_text(request.lease_epoch, request.lease_epoch_len),
            read_request_text(request.correlation_id, request.correlation_id_len),
            read_request_text(request.locale, request.locale_len),
        )
    })
    else {
        return 400;
    };
    if let Ok(mut record) = CONTENT_MANIFEST_REQUEST.lock() {
        *record = Some((
            request.kind,
            instance_id,
            caller_id,
            session_id,
            lease_id,
            lease_epoch,
            correlation_id,
            locale,
        ));
    } else {
        return 500;
    }
    let response = match CONTENT_MANIFEST_RESPONSE.lock() {
        Ok(value) => value.as_deref().unwrap_or(CONTENT_MANIFEST_ERROR).to_vec(),
        Err(_) => return 500,
    };
    if response.len() > capacity {
        return 500;
    }
    // SAFETY: The static response fits the dispatcher-provided output capacity.
    unsafe {
        std::ptr::copy_nonoverlapping(response.as_ptr(), output, response.len());
        *length = response.len();
    }
    match CONTENT_MANIFEST_STATUS.lock() {
        Ok(value) => value.unwrap_or(503),
        Err(_) => 500,
    }
}

fn content_manifest_round_trip(authorization: &str) -> std::io::Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let mut client = TcpStream::connect(listener.local_addr()?)?;
    client.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
    let (mut server, _) = listener.accept()?;
    write!(
        client,
        "GET /api/v1/game-information/content-manifest HTTP/1.1\r\n\
         Host: localhost\r\n\
         Authorization: {authorization}\r\n\
         Content-Length: 0\r\n\
         x-sts2-instance-id: instance-1\r\n\
         x-sts2-caller-id: caller-1\r\n\
         x-sts2-session-id: session-1\r\n\
         x-sts2-lease-id: lease-1\r\n\
         x-sts2-lease-epoch: 1\r\n\
         x-sts2-correlation-id: corr-1\r\n\
         x-sts2-locale: en_US\r\n\r\n"
    )?;
    let stop = std::sync::atomic::AtomicBool::new(false);
    super::connection::handle_connection(
        &mut super::io::Connection::new(&mut server, &stop, std::time::Duration::from_secs(2)),
        "127.0.0.1:0",
        b"token",
        record_content_manifest,
    )?;
    drop(server);
    let mut response = String::new();
    client.read_to_string(&mut response)?;
    Ok(response)
}

#[test]
fn fixed_content_manifest_get_records_owner_context_and_returns_protocol_error()
-> std::io::Result<()> {
    *CONTENT_MANIFEST_RESPONSE
        .lock()
        .map_err(|_| std::io::Error::other("content manifest response lock poisoned"))? = None;
    *CONTENT_MANIFEST_STATUS
        .lock()
        .map_err(|_| std::io::Error::other("content manifest status lock poisoned"))? = None;
    *CONTENT_MANIFEST_REQUEST
        .lock()
        .map_err(|_| std::io::Error::other("content manifest test lock poisoned"))? = None;

    let unauthorized = content_manifest_round_trip("Bearer wrong")?;
    assert!(unauthorized.starts_with("HTTP/1.1 401"));
    assert!(
        CONTENT_MANIFEST_REQUEST
            .lock()
            .map_err(|_| std::io::Error::other("content manifest test lock poisoned"))?
            .is_none()
    );

    let response = content_manifest_round_trip("Bearer token")?;
    assert!(response.starts_with("HTTP/1.1 503"));
    assert!(
        response
            .ends_with(std::str::from_utf8(CONTENT_MANIFEST_ERROR).map_err(std::io::Error::other)?)
    );
    assert_eq!(
        *CONTENT_MANIFEST_REQUEST
            .lock()
            .map_err(|_| std::io::Error::other("content manifest test lock poisoned"))?,
        Some((
            23,
            String::from("instance-1"),
            String::from("caller-1"),
            String::from("session-1"),
            String::from("lease-1"),
            String::from("1"),
            String::from("corr-1"),
            String::from("en_US"),
        ))
    );

    let large_body = success_manifest_body(1400);
    assert!(large_body.len() > 256 * 1024);
    *CONTENT_MANIFEST_RESPONSE
        .lock()
        .map_err(|_| std::io::Error::other("content manifest response lock poisoned"))? =
        Some(large_body.clone());
    *CONTENT_MANIFEST_STATUS
        .lock()
        .map_err(|_| std::io::Error::other("content manifest status lock poisoned"))? = Some(200);
    let large_response = content_manifest_round_trip("Bearer token")?;
    let (large_headers, large_response_body) = large_response
        .split_once("\r\n\r\n")
        .ok_or_else(|| std::io::Error::other("large response has no HTTP header boundary"))?;
    assert!(
        large_headers.starts_with("HTTP/1.1 200 OK"),
        "unexpected large response headers: {large_headers}"
    );
    assert!(large_headers.contains(&format!("Content-Length: {}", large_body.len())));
    assert_eq!(large_response_body.as_bytes(), large_body);
    let codec = GameInformationContentManifestV1Codec::new().expect("pinned protocol schema");
    let decoded = codec
        .decode(large_response_body.as_bytes())
        .expect("whole large response validates against the pinned schema");
    assert_eq!(
        decoded["schema_digest"],
        GAME_INFORMATION_CONTENT_MANIFEST_V1_SCHEMA_DIGEST
    );
    assert_eq!(
        decoded["manifest"]["definitions"].as_array().map(Vec::len),
        Some(1400)
    );

    let bounded_error = error_manifest_body();
    *CONTENT_MANIFEST_RESPONSE
        .lock()
        .map_err(|_| std::io::Error::other("content manifest response lock poisoned"))? =
        Some(bounded_error.clone());
    *CONTENT_MANIFEST_STATUS
        .lock()
        .map_err(|_| std::io::Error::other("content manifest status lock poisoned"))? = Some(413);
    let error_response = content_manifest_round_trip("Bearer token")?;
    let (error_headers, error_response_body) = error_response
        .split_once("\r\n\r\n")
        .ok_or_else(|| std::io::Error::other("error response has no HTTP header boundary"))?;
    assert!(error_headers.starts_with("HTTP/1.1 413 Payload Too Large"));
    assert!(error_response_body.len() < 1024);
    assert_eq!(error_response_body.as_bytes(), bounded_error);
    let decoded_error = codec
        .decode(error_response_body.as_bytes())
        .expect("bounded error validates against the pinned schema");
    assert_eq!(decoded_error["error"]["code"], "result_limit_exceeded");
    assert_eq!(
        decoded_error["error"]["reason"],
        "serialized_payload_too_large"
    );

    *CONTENT_MANIFEST_RESPONSE
        .lock()
        .map_err(|_| std::io::Error::other("content manifest response lock poisoned"))? = None;
    *CONTENT_MANIFEST_STATUS
        .lock()
        .map_err(|_| std::io::Error::other("content manifest status lock poisoned"))? = None;
    Ok(())
}
