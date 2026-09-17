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
    if words.next() != Some("HTTP/1.1") || words.next().is_some() {
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

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::net::TcpListener;
    use std::thread;

    use super::read_request;

    fn exchange(request_line: &str) -> Result<super::Request, String> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
        let address = listener.local_addr().map_err(|error| error.to_string())?;
        let request_line = request_line.to_owned();
        let client = thread::spawn(move || {
            let mut stream =
                std::net::TcpStream::connect(address).map_err(|error| error.to_string())?;
            let request = format!("{request_line}\r\nHost: localhost\r\nContent-Length: 0\r\n\r\n");
            stream
                .write_all(request.as_bytes())
                .map_err(|error| error.to_string())
        });
        let (mut server, _) = listener.accept().map_err(|error| error.to_string())?;
        let parsed = read_request(&mut server);
        client
            .join()
            .map_err(|_| String::from("client panicked"))??;
        parsed
    }

    #[test]
    fn accepts_standard_http11_request_line_over_tcp() {
        let request = exchange("POST /api/v1/runtime/recovery HTTP/1.1").expect("request");
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/api/v1/runtime/recovery");
    }

    #[test]
    fn rejects_wrong_http_version_and_extra_tokens_over_tcp() {
        assert!(exchange("POST /api/v1/runtime/recovery HTTP/1.0").is_err());
        assert!(exchange("POST /api/v1/runtime/recovery HTTP/1.1 extra").is_err());
    }
}
