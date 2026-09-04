// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::io::{Read, Write};

const MAX_HEADER_BYTES: usize = 8 * 1024;
const MAX_BODY_BYTES: usize = 16 * 1024;
const MAX_IDENTITY_BYTES: usize = 128;

pub(super) struct Request {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) headers: BTreeMap<String, String>,
    pub(super) body: Vec<u8>,
}

impl Request {
    pub(super) fn content_type_is_json(&self) -> bool {
        self.headers.get("content-type").map(String::as_str) == Some("application/json")
    }
}

pub(super) fn read_request(stream: &mut impl Read) -> Result<Request, u16> {
    let mut bytes = Vec::with_capacity(MAX_HEADER_BYTES);
    let mut buffer = [0_u8; 1024];
    let header_end = loop {
        if let Some(end) = find_header_end(&bytes) {
            break end;
        }
        if bytes.len() >= MAX_HEADER_BYTES {
            return Err(413);
        }
        let read = stream.read(&mut buffer).map_err(|_| 400_u16)?;
        if read == 0 {
            return Err(400);
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() > MAX_HEADER_BYTES + MAX_BODY_BYTES {
            return Err(413);
        }
    };
    if header_end + 4 > MAX_HEADER_BYTES {
        return Err(413);
    }
    let header_text = std::str::from_utf8(&bytes[..header_end]).map_err(|_| 400_u16)?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().ok_or(400_u16)?;
    let mut request_parts = request_line.split_ascii_whitespace();
    let method = request_parts.next().ok_or(400_u16)?.to_owned();
    let path = request_parts.next().ok_or(400_u16)?.to_owned();
    if request_parts.next() != Some("HTTP/1.1") || request_parts.next().is_some() {
        return Err(400);
    }
    let mut headers = BTreeMap::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(400);
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        if name.is_empty()
            || value.len() > MAX_HEADER_BYTES
            || headers.insert(name, value.to_owned()).is_some()
        {
            return Err(400);
        }
    }
    let content_length = match headers.get("content-length") {
        Some(value) => value.parse::<usize>().map_err(|_| 400_u16)?,
        None => 0,
    };
    if content_length > MAX_BODY_BYTES {
        return Err(413);
    }
    let body_start = header_end + 4;
    if bytes.len() < body_start {
        return Err(400);
    }
    let available = bytes.len() - body_start;
    if available > content_length {
        return Err(400);
    }
    let mut body = bytes[body_start..].to_vec();
    while body.len() < content_length {
        let remaining = content_length - body.len();
        let read_capacity = remaining.min(buffer.len());
        let read = stream
            .read(&mut buffer[..read_capacity])
            .map_err(|_| 400_u16)?;
        if read == 0 {
            return Err(400);
        }
        body.extend_from_slice(&buffer[..read]);
    }
    Ok(Request {
        method,
        path,
        headers,
        body,
    })
}

pub(super) fn headers_are_allowed(headers: &BTreeMap<String, String>) -> bool {
    headers.keys().all(|name| {
        matches!(
            name.as_str(),
            "authorization"
                | "content-length"
                | "content-type"
                | "host"
                | "connection"
                | "x-sts2-instance-id"
                | "x-sts2-caller-id"
                | "x-sts2-session-id"
                | "x-sts2-lease-id"
                | "x-sts2-lease-epoch"
                | "x-sts2-correlation-id"
        )
    })
}

pub(super) fn safe_header_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTITY_BYTES
        && !value.contains("..")
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

pub(super) fn write_response(
    stream: &mut impl Write,
    status: u16,
    body: &[u8],
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        409 => "Conflict",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Error",
    };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)
}

#[cfg(test)]
mod tests {
    use super::{find_header_end, safe_header_value};

    #[test]
    fn accepts_only_bounded_identity_values() {
        assert!(safe_header_value("instance-1"));
        assert!(safe_header_value("session:1/test"));
        assert!(!safe_header_value("../escape"));
        assert!(!safe_header_value("header value"));
    }

    #[test]
    fn detects_complete_http_headers() {
        assert_eq!(find_header_end(b"GET / HTTP/1.1\r\n\r\nbody"), Some(14));
        assert_eq!(find_header_end(b"GET / HTTP/1.1\n\n"), None);
    }

    #[test]
    fn rejects_header_terminator_beyond_header_budget() {
        let mut bytes = b"GET / HTTP/1.1\r\nHost: ".to_vec();
        bytes.resize(super::MAX_HEADER_BYTES - 1, b'x');
        bytes.extend_from_slice(b"\r\n\r\n");
        assert!(matches!(
            super::read_request(&mut bytes.as_slice()),
            Err(413)
        ));
    }
}
