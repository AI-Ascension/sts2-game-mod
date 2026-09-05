// SPDX-License-Identifier: MIT

use super::{
    CALLBACK_ACTION, CALLBACK_RUNTIME_V2_ACTION, CALLBACK_RUNTIME_V2_OPERATION,
    CALLBACK_RUNTIME_V2_STATE, RuntimeRequestCallback, dispatch as dispatch_callback,
    dispatch_with_body, http,
};

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
        _ => dispatch_gameplay(callback, request, stream),
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
    dispatch_callback(callback, super::CALLBACK_GAMEPLAY, request, stream)
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
mod tests {
    use super::{dispatch, http};
    use crate::runtime::RuntimeRequest;
    use std::io::Read;
    use std::net::{TcpListener, TcpStream};

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
            &mut crate::runtime::io::Connection::new(
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
}
