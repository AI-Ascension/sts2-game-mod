// SPDX-License-Identifier: MIT

use super::{RuntimeRequestCallback, auth, http, io, routes};

pub(super) fn handle_connection(
    stream: &mut io::Connection<'_>,
    listener_address: &str,
    token: &[u8],
    callback: RuntimeRequestCallback,
) -> std::io::Result<()> {
    let request = match http::read_request(stream) {
        Ok(value) => value,
        Err(status) => {
            return http::write_response(stream, status, b"{\"error_code\":\"malformed_request\"}");
        }
    };
    if !http::headers_are_allowed(&request.headers) {
        return http::write_response(stream, 400, b"{\"error_code\":\"unsupported_header\"}");
    }
    if !auth::bearer_token_matches(request.headers.get("authorization"), token) {
        return http::write_response(stream, 401, b"{\"error_code\":\"unauthorized\"}");
    }

    routes::dispatch(callback, &request, listener_address, stream)
}
