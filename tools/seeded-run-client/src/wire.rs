// SPDX-License-Identifier: MIT

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::context::ClientError;

/// Sends a compact JSON body to `POST /v2/seeded-run` and returns `(status, body)`.
pub fn post_seeded_run(
    host: &str,
    port: u16,
    token: &str,
    identity_headers: &[(&str, &str)],
    body: &str,
) -> Result<(u16, String), ClientError> {
    let mut stream = TcpStream::connect((host, port))
        .map_err(|error| ClientError::Transport(error.to_string()))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(15)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(15)));

    let mut request = String::new();
    request.push_str("POST /v2/seeded-run HTTP/1.1\r\n");
    request.push_str(&format!("Host: {host}\r\n"));
    request.push_str(&format!("Authorization: Bearer {token}\r\n"));
    for (name, value) in identity_headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    request.push_str("Content-Type: application/json\r\n");
    request.push_str(&format!("Content-Length: {}\r\n", body.len()));
    request.push_str("Connection: close\r\n\r\n");
    request.push_str(body);

    stream
        .write_all(request.as_bytes())
        .map_err(|error| ClientError::Transport(error.to_string()))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| ClientError::Transport(error.to_string()))?;
    Ok(parse_response(&response))
}

fn parse_response(raw: &str) -> (u16, String) {
    let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((raw, ""));
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    (status, body.to_owned())
}

#[cfg(test)]
mod tests {
    use super::parse_response;

    #[test]
    fn parses_status_and_body() {
        let (status, body) = parse_response("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}");
        assert_eq!(status, 200);
        assert_eq!(body, "{}");
    }

    #[test]
    fn malformed_response_yields_zero_status() {
        let (status, body) = parse_response("garbage");
        assert_eq!(status, 0);
        assert!(body.is_empty());
    }
}
