// SPDX-License-Identifier: MIT

/// HTTP method understood by the target-local adapter seam.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpMethod {
    /// Read-oriented request.
    Get,
    /// Work-admission request.
    Post,
}

/// Borrowed request view received after a listener performs transport framing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HttpRequest<'a> {
    method: HttpMethod,
    path: &'a str,
    body: &'a [u8],
}

impl<'a> HttpRequest<'a> {
    /// Creates a request view without allocating or parsing a wire protocol.
    #[must_use]
    pub const fn new(method: HttpMethod, path: &'a str, body: &'a [u8]) -> Self {
        Self { method, path, body }
    }

    /// Returns the request method.
    #[must_use]
    pub const fn method(self) -> HttpMethod {
        self.method
    }

    /// Returns the target-local path.
    #[must_use]
    pub const fn path(self) -> &'a str {
        self.path
    }

    /// Returns the borrowed request body.
    #[must_use]
    pub const fn body(self) -> &'a [u8] {
        self.body
    }
}

/// Bounded response produced by a target-local handler.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}

impl HttpResponse {
    /// Creates a response with an owned body.
    #[must_use]
    pub fn new(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            body: body.into(),
        }
    }

    /// Returns the response status.
    #[must_use]
    pub const fn status(&self) -> u16 {
        self.status
    }

    /// Returns the owned response body.
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }
}

/// Status returned when a request exceeds the configured body limit.
pub const PAYLOAD_TOO_LARGE: u16 = 413;

/// Port implemented by the host-facing composition layer.
pub trait HttpPort {
    /// Handles one bounded request and returns an owned response.
    fn dispatch(&mut self, request: HttpRequest<'_>) -> HttpResponse;
}

/// Pure bounded adapter around a host-facing HTTP port.
#[derive(Debug)]
pub struct HttpAdapter<P> {
    port: P,
    max_body_bytes: usize,
}

impl<P> HttpAdapter<P> {
    /// Creates an adapter with an explicit request-body bound.
    #[must_use]
    pub const fn new(port: P, max_body_bytes: usize) -> Self {
        Self {
            port,
            max_body_bytes,
        }
    }

    /// Returns the configured request-body bound.
    #[must_use]
    pub const fn max_body_bytes(&self) -> usize {
        self.max_body_bytes
    }

    /// Handles a request without opening a socket or choosing a route.
    pub fn handle(&mut self, request: HttpRequest<'_>) -> HttpResponse
    where
        P: HttpPort,
    {
        if request.body().len() > self.max_body_bytes {
            return HttpResponse::new(PAYLOAD_TOO_LARGE, []);
        }
        self.port.dispatch(request)
    }

    /// Returns the inner host-facing port.
    #[must_use]
    pub fn into_port(self) -> P {
        self.port
    }
}
