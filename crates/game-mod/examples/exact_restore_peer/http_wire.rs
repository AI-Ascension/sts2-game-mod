// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpStream;

pub(crate) const MAX_HTTP_HEADERS: usize = 16 * 1024;
pub(crate) const MAX_RECOVERY_FRAME: usize = 262_144;

pub(crate) struct Request {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body: Vec<u8>,
}

pub(crate) fn read_request(stream: &mut TcpStream) -> Result<Request, String> {
    let mut prefix = Vec::new();
    let header_end = loop {
        let mut byte = [0_u8; 1];
        stream
            .read_exact(&mut byte)
            .map_err(|error| format!("read request: {error}"))?;
        prefix.push(byte[0]);
        if prefix.len() > MAX_HTTP_HEADERS {
            return Err(String::from("HTTP headers exceed bound"));
        }
        if prefix.ends_with(b"\r\n\r\n") {
            break prefix.len() - 4;
        }
    };
    let header_text = std::str::from_utf8(&prefix[..header_end])
        .map_err(|_| String::from("HTTP headers are not UTF-8"))?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().ok_or("missing request line")?;
    let mut words = request_line.split_whitespace();
    let method = words.next().ok_or("missing HTTP method")?.to_owned();
    let path = words.next().ok_or("missing HTTP path")?.to_owned();
    if words.next().is_some() {
        return Err(String::from("invalid request line"));
    }
    let mut headers = BTreeMap::new();
    for line in lines {
        let (name, value) = line.split_once(':').ok_or("invalid HTTP header")?;
        headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
    }
    let length = headers
        .get("content-length")
        .ok_or("content-length is required")?
        .parse::<usize>()
        .map_err(|_| String::from("invalid content-length"))?;
    if length > MAX_RECOVERY_FRAME {
        return Err(String::from("HTTP body exceeds bound"));
    }
    let mut body = vec![0_u8; length];
    stream
        .read_exact(&mut body)
        .map_err(|error| format!("read body: {error}"))?;
    Ok(Request {
        method,
        path,
        headers,
        body,
    })
}

pub(crate) fn write_response(
    stream: &mut TcpStream,
    status: u16,
    body: &[u8],
) -> Result<(), String> {
    let reason = match status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        503 => "Service Unavailable",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .and_then(|_| stream.write_all(body))
        .map_err(|error| format!("write response: {error}"))
}
