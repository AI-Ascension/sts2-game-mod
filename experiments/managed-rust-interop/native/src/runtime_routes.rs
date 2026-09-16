// SPDX-License-Identifier: MIT

use super::runtime_dispatch::{dispatch as dispatch_callback, dispatch_with_body};
use super::{
    CALLBACK_ACTION, CALLBACK_CONTENT_MANIFEST, CALLBACK_COOP_ACTION, CALLBACK_COOP_LEGAL_CATALOG,
    CALLBACK_COOP_OBSERVATION, CALLBACK_COOP_RECOVER, CALLBACK_COOP_REJOIN, CALLBACK_COOP_VOTE,
    CALLBACK_LOOKUP_BINDING, CALLBACK_RUNTIME_MAP, CALLBACK_RUNTIME_V2_ACTION,
    CALLBACK_RUNTIME_V2_OPERATION, CALLBACK_RUNTIME_V2_STATE, CALLBACK_RUNTIME_V4_EXPERT,
    CALLBACK_RUNTIME_V4_EXPERT_ACTION, CALLBACK_RUNTIME_V4_EXPERT_REST_ACTION,
    CALLBACK_SEEDED_OPERATION, CALLBACK_SEEDED_RUN, RuntimeRequestCallback, http,
};
use sts2_game_mod::{ExactRestoreAuthorization, exact_restore_unavailable_response};

pub(super) const CALLBACK_GAMEPLAY: u32 = super::CALLBACK_GAMEPLAY;

pub(super) fn dispatch(
    callback: RuntimeRequestCallback,
    request: &http::Request,
    listener_address: &str,
    stream: &mut super::io::Connection<'_>,
) -> std::io::Result<()> {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health/ready") if request.body.is_empty() => {
            let response = format!(r#"{{"status":"ready","listener":"{listener_address}"}}"#);
            http::write_response(stream, 200, response.as_bytes())
        }
        (
            "POST",
            path @ ("/v1/exact-restore/begin"
            | "/v1/exact-restore/chunk"
            | "/v1/exact-restore/finish"
            | "/v1/exact-restore/commit"
            | "/v1/exact-restore/lookup"),
        ) if request.content_type_is_json() => exact_restore_unavailable(path, request, stream),
        ("GET", "/api/v1/runtime/state") if request.body.is_empty() => {
            dispatch_callback(callback, 1, request, stream)
        }
        ("POST", "/api/v1/runtime/action") if request.content_type_is_json() => {
            dispatch_callback(callback, CALLBACK_ACTION, request, stream)
        }
        ("GET", "/api/v2/runtime/state") if request.body.is_empty() => {
            dispatch_callback(callback, CALLBACK_RUNTIME_V2_STATE, request, stream)
        }
        ("POST", "/api/v2/runtime/action") if request.content_type_is_json() => {
            dispatch_callback(callback, CALLBACK_RUNTIME_V2_ACTION, request, stream)
        }
        ("POST", "/v2/seeded-run") if request.content_type_is_json() => {
            dispatch_callback(callback, CALLBACK_SEEDED_RUN, request, stream)
        }
        ("GET", path) if request.body.is_empty() && path.starts_with("/v2/seeded-operations/") => {
            let operation_id = &path["/v2/seeded-operations/".len()..];
            dispatch_operation(
                callback,
                CALLBACK_SEEDED_OPERATION,
                request,
                operation_id,
                stream,
            )
        }
        ("GET", path)
            if request.body.is_empty() && path.starts_with("/api/v2/runtime/operations/") =>
        {
            let operation_id = &path["/api/v2/runtime/operations/".len()..];
            dispatch_operation(
                callback,
                CALLBACK_RUNTIME_V2_OPERATION,
                request,
                operation_id,
                stream,
            )
        }
        ("GET", "/api/v4/runtime/expert-state") if request.body.is_empty() => {
            dispatch_callback(callback, CALLBACK_RUNTIME_V4_EXPERT, request, stream)
        }
        ("POST", "/api/v4/runtime/expert-action") if request.content_type_is_json() => {
            dispatch_callback(callback, CALLBACK_RUNTIME_V4_EXPERT_ACTION, request, stream)
        }
        ("GET", path)
            if request.body.is_empty() && path.starts_with("/api/v4/runtime/expert-actions/") =>
        {
            let operation_id = &path["/api/v4/runtime/expert-actions/".len()..];
            dispatch_operation(
                callback,
                CALLBACK_RUNTIME_V4_EXPERT_ACTION,
                request,
                operation_id,
                stream,
            )
        }
        ("POST", "/api/v4/runtime/expert-rest-action") if request.content_type_is_json() => {
            dispatch_callback(
                callback,
                CALLBACK_RUNTIME_V4_EXPERT_REST_ACTION,
                request,
                stream,
            )
        }
        ("GET", path)
            if request.body.is_empty()
                && path.starts_with("/api/v4/runtime/expert-rest-actions/") =>
        {
            let operation_id = &path["/api/v4/runtime/expert-rest-actions/".len()..];
            dispatch_operation(
                callback,
                CALLBACK_RUNTIME_V4_EXPERT_REST_ACTION,
                request,
                operation_id,
                stream,
            )
        }
        ("GET", "/api/v1/coop/native/observation") if request.body.is_empty() => {
            dispatch_callback(callback, CALLBACK_COOP_OBSERVATION, request, stream)
        }
        ("POST", "/api/v1/coop/native/action") if request.content_type_is_json() => {
            dispatch_callback(callback, CALLBACK_COOP_ACTION, request, stream)
        }
        ("POST", "/api/v1/coop/native/vote") if request.content_type_is_json() => {
            dispatch_callback(callback, CALLBACK_COOP_VOTE, request, stream)
        }
        ("POST", "/api/v1/coop/native/rejoin") if request.content_type_is_json() => {
            dispatch_callback(callback, CALLBACK_COOP_REJOIN, request, stream)
        }
        ("POST", "/api/v1/coop/native/recover") if request.content_type_is_json() => {
            dispatch_callback(callback, CALLBACK_COOP_RECOVER, request, stream)
        }
        ("POST", "/api/v1/coop/native/legal-catalog") if request.content_type_is_json() => {
            dispatch_callback(callback, CALLBACK_COOP_LEGAL_CATALOG, request, stream)
        }
        ("GET", "/api/map/v1/snapshot") if request.body.is_empty() => {
            dispatch_callback(callback, CALLBACK_RUNTIME_MAP, request, stream)
        }
        ("POST", "/api/v1/game-information/lookup-binding") if request.content_type_is_json() => {
            dispatch_callback(callback, CALLBACK_LOOKUP_BINDING, request, stream)
        }
        ("GET", "/api/v1/game-information/content-manifest") if request.body.is_empty() => {
            dispatch_callback(callback, CALLBACK_CONTENT_MANIFEST, request, stream)
        }
        _ => dispatch_gameplay(callback, request, stream),
    }
}

/// The production restore owner is unavailable. This bearer-authenticated route validates the
/// closed protocol frame and correlation, then refuses before constructing storage or accepting
/// bytes. Header identities are echoed for request matching and are not treated as live authority.
fn exact_restore_unavailable(
    path: &str,
    request: &http::Request,
    stream: &mut super::io::Connection<'_>,
) -> std::io::Result<()> {
    let expected_kind = match path {
        "/v1/exact-restore/begin" => "exact_restore_begin_request",
        "/v1/exact-restore/chunk" => "exact_restore_chunk_request",
        "/v1/exact-restore/finish" => "exact_restore_finish_blob_request",
        "/v1/exact-restore/commit" => "exact_restore_commit_request",
        "/v1/exact-restore/lookup" => "exact_restore_lookup_request",
        _ => return http::write_response(stream, 404, b"{\"error_code\":\"route_not_found\"}"),
    };
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
        return http::write_response(stream, 400, b"{\"error_code\":\"invalid_identity\"}");
    }
    let Ok(lease_epoch) = lease_epoch.parse::<u64>() else {
        return http::write_response(stream, 400, b"{\"error_code\":\"invalid_lease_epoch\"}");
    };
    // This fixed value identifies only the already bearer-authenticated gateway transport.
    // `caller_id` is required for the established envelope but is not an authority witness here.
    let authorization = match ExactRestoreAuthorization::new(
        String::from("bearer-authenticated-gateway"),
        correlation_id.clone(),
        instance_id.clone(),
        session_id.clone(),
        lease_id.clone(),
        lease_epoch,
    ) {
        Ok(value) => value,
        Err(_) => {
            return http::write_response(stream, 400, b"{\"error_code\":\"invalid_identity\"}");
        }
    };
    match exact_restore_unavailable_response(
        &request.body,
        &authorization,
        "bearer-authenticated-gateway",
        expected_kind,
    ) {
        Ok(response) => http::write_response(stream, response.status, &response.body),
        Err(_) => http::write_response(
            stream,
            400,
            b"{\"error_code\":\"invalid_exact_restore_frame\"}",
        ),
    }
}

fn dispatch_gameplay(
    callback: RuntimeRequestCallback,
    request: &http::Request,
    stream: &mut super::io::Connection<'_>,
) -> std::io::Result<()> {
    let expected = super::gameplay_route::expected_kind(&request.method, &request.path)
        .filter(|_| request.content_type_is_json());
    let Some(expected) = expected else {
        return http::write_response(stream, 404, b"{\"error_code\":\"route_not_found\"}");
    };
    if !super::gameplay_route::body_matches(&request.body, expected) {
        return http::write_response(stream, 400, b"{\"error_code\":\"invalid_route_message\"}");
    }
    dispatch_callback(callback, CALLBACK_GAMEPLAY, request, stream)
}

fn dispatch_operation(
    callback: RuntimeRequestCallback,
    kind: u32,
    request: &http::Request,
    operation_id: &str,
    stream: &mut super::io::Connection<'_>,
) -> std::io::Result<()> {
    // The entire suffix is an opaque protocol identity, not a filesystem path.
    // '/' is admitted in action operation IDs and must remain retrievable here.
    if !http::safe_header_value(operation_id) {
        return http::write_response(stream, 400, b"{\"error_code\":\"unsafe_operation_id\"}");
    }
    dispatch_with_body(callback, kind, request, operation_id.as_bytes(), stream)
}

#[cfg(test)]
#[path = "runtime_routes_tests.rs"]
mod tests;
