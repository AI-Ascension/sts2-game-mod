// SPDX-License-Identifier: MIT

use std::net::TcpStream;

use super::{
    CALLBACK_ACTION, CALLBACK_RUNTIME_V2_ACTION, CALLBACK_RUNTIME_V2_OPERATION,
    CALLBACK_RUNTIME_V2_STATE, CALLBACK_RUNTIME_V3_ACTION, CALLBACK_RUNTIME_V3_OPERATION,
    CALLBACK_RUNTIME_V3_STATE, RuntimeRequestCallback, dispatch as dispatch_callback,
    dispatch_with_body, http,
};

pub(super) fn dispatch(
    callback: RuntimeRequestCallback,
    request: &http::Request,
    listener_address: &str,
    stream: &mut TcpStream,
) -> std::io::Result<()> {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health/ready") if request.body.is_empty() => {
            let response = format!(r#"{{"status":"ready","listener":"{listener_address}"}}"#);
            http::write_response(stream, 200, response.as_bytes())
        }
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
        ("GET", "/api/v3/runtime/state") if request.body.is_empty() => {
            dispatch_callback(callback, CALLBACK_RUNTIME_V3_STATE, request, stream)
        }
        ("POST", "/api/v3/runtime/action") if request.content_type_is_json() => {
            dispatch_callback(callback, CALLBACK_RUNTIME_V3_ACTION, request, stream)
        }
        ("GET", path) if request.body.is_empty() => {
            let Some(operation_id) = path.strip_prefix("/api/v2/runtime/operations/") else {
                let Some(operation_id) = path.strip_prefix("/api/v3/runtime/operations/") else {
                    return http::write_response(
                        stream,
                        404,
                        b"{\"error_code\":\"route_not_found\"}",
                    );
                };
                return dispatch_operation(
                    callback,
                    CALLBACK_RUNTIME_V3_OPERATION,
                    request,
                    operation_id,
                    stream,
                );
            };
            dispatch_operation(
                callback,
                CALLBACK_RUNTIME_V2_OPERATION,
                request,
                operation_id,
                stream,
            )
        }
        _ => http::write_response(stream, 404, b"{\"error_code\":\"route_not_found\"}"),
    }
}

fn dispatch_operation(
    callback: RuntimeRequestCallback,
    kind: u32,
    request: &http::Request,
    operation_id: &str,
    stream: &mut TcpStream,
) -> std::io::Result<()> {
    if !http::safe_header_value(operation_id) || operation_id.contains('/') {
        return http::write_response(stream, 400, b"{\"error_code\":\"unsafe_operation_id\"}");
    }
    dispatch_with_body(callback, kind, request, operation_id.as_bytes(), stream)
}
