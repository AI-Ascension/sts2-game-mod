// SPDX-License-Identifier: MIT

use super::{
    CALLBACK_CONTENT_MANIFEST, CALLBACK_GAME_INFORMATION_QUERY,
    CALLBACK_LIVE_OBSERVATION_BOOTSTRAP, CALLBACK_LOOKUP_BINDING,
    MAX_CONTENT_MANIFEST_RESPONSE_BYTES, MAX_RESPONSE_BYTES, RuntimeRequest,
    RuntimeRequestCallback, http, io,
};

pub(super) fn dispatch(
    callback: RuntimeRequestCallback,
    kind: u32,
    request: &http::Request,
    stream: &mut io::Connection<'_>,
) -> std::io::Result<()> {
    dispatch_with_body(callback, kind, request, &request.body, stream)
}

pub(super) fn dispatch_with_body(
    callback: RuntimeRequestCallback,
    kind: u32,
    request: &http::Request,
    body: &[u8],
    stream: &mut io::Connection<'_>,
) -> std::io::Result<()> {
    let Some(instance_id) = request.headers.get("x-sts2-instance-id") else {
        return http::write_response(stream, 400, b"{\"error_code\":\"missing_instance_id\"}");
    };
    let Some(caller_id) = request.headers.get("x-sts2-caller-id") else {
        return http::write_response(stream, 400, b"{\"error_code\":\"missing_caller_id\"}");
    };
    let Some(session_id) = request.headers.get("x-sts2-session-id") else {
        return http::write_response(stream, 400, b"{\"error_code\":\"missing_session_id\"}");
    };
    let Some(lease_id) = request.headers.get("x-sts2-lease-id") else {
        return http::write_response(stream, 400, b"{\"error_code\":\"missing_lease_id\"}");
    };
    let Some(lease_epoch) = request.headers.get("x-sts2-lease-epoch") else {
        return http::write_response(stream, 400, b"{\"error_code\":\"missing_lease_epoch\"}");
    };
    let Some(correlation_id) = request.headers.get("x-sts2-correlation-id") else {
        return http::write_response(stream, 400, b"{\"error_code\":\"missing_correlation_id\"}");
    };
    let locale = request.headers.get("x-sts2-locale");
    if matches!(
        kind,
        CALLBACK_LOOKUP_BINDING
            | CALLBACK_CONTENT_MANIFEST
            | CALLBACK_LIVE_OBSERVATION_BOOTSTRAP
            | CALLBACK_GAME_INFORMATION_QUERY
    ) && locale.is_none()
    {
        return http::write_response(stream, 400, b"{\"error_code\":\"missing_locale\"}");
    }
    if [
        instance_id.as_str(),
        caller_id.as_str(),
        session_id.as_str(),
        lease_id.as_str(),
        lease_epoch.as_str(),
        correlation_id.as_str(),
    ]
    .into_iter()
    .any(|value| !http::safe_header_value(value))
    {
        return http::write_response(stream, 400, b"{\"error_code\":\"unsafe_identity\"}");
    }
    if kind == CALLBACK_LOOKUP_BINDING
        && !http::safe_header_value(locale.map_or("", String::as_str))
    {
        return http::write_response(stream, 400, b"{\"error_code\":\"unsafe_locale\"}");
    }
    if matches!(
        kind,
        CALLBACK_CONTENT_MANIFEST
            | CALLBACK_LIVE_OBSERVATION_BOOTSTRAP
            | CALLBACK_GAME_INFORMATION_QUERY
    ) && !content_manifest_locale(locale.map_or("", String::as_str))
    {
        return http::write_response(stream, 400, b"{\"error_code\":\"unsafe_locale\"}");
    }

    let native_request = RuntimeRequest {
        kind,
        instance_id: instance_id.as_bytes().as_ptr(),
        instance_id_len: instance_id.len(),
        caller_id: caller_id.as_bytes().as_ptr(),
        caller_id_len: caller_id.len(),
        session_id: session_id.as_bytes().as_ptr(),
        session_id_len: session_id.len(),
        lease_id: lease_id.as_bytes().as_ptr(),
        lease_id_len: lease_id.len(),
        lease_epoch: lease_epoch.as_bytes().as_ptr(),
        lease_epoch_len: lease_epoch.len(),
        correlation_id: correlation_id.as_bytes().as_ptr(),
        correlation_id_len: correlation_id.len(),
        locale: locale.map_or(std::ptr::null(), |value| value.as_bytes().as_ptr()),
        locale_len: locale.map_or(0, String::len),
        body: body.as_ptr(),
        body_len: body.len(),
    };
    let response_capacity = if kind == CALLBACK_CONTENT_MANIFEST {
        MAX_CONTENT_MANIFEST_RESPONSE_BYTES
    } else {
        MAX_RESPONSE_BYTES
    };
    let mut output = vec![0_u8; response_capacity];
    let mut output_length = 0_usize;
    // SAFETY: Request fields borrow live read-only buffers; output and length are distinct,
    // exclusively borrowed storage. The callback obeys capacity, does not unwind or retain pointers.
    // The start caller guarantees callback validity on this listener thread until stop joins it.
    let status = unsafe {
        callback(
            &native_request,
            output.as_mut_ptr(),
            output.len(),
            &mut output_length,
        )
    };
    if !(200..600).contains(&status) || output_length > output.len() {
        return http::write_response(stream, 500, b"{\"error_code\":\"callback_failed\"}");
    }
    http::write_response(stream, status as u16, &output[..output_length])
}

fn content_manifest_locale(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b':' | b'/' | b'_' | b'-')
        })
}
