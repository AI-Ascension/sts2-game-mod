// SPDX-License-Identifier: MIT

use sts2_mod_http_adapter::{
    HttpAdapter, HttpMethod, HttpPort, HttpRequest, HttpResponse, PAYLOAD_TOO_LARGE,
};

#[derive(Debug, Default)]
struct RecordingPort {
    calls: Vec<(HttpMethod, String, usize)>,
}

impl HttpPort for RecordingPort {
    fn dispatch(&mut self, request: HttpRequest<'_>) -> HttpResponse {
        self.calls.push((
            request.method(),
            request.path().to_owned(),
            request.body().len(),
        ));
        HttpResponse::new(202, [])
    }
}

#[test]
fn accepts_bounded_request_and_delegates_once() {
    let mut adapter = HttpAdapter::new(RecordingPort::default(), 4);
    let response = adapter.handle(HttpRequest::new(HttpMethod::Post, "/opaque", &[1, 2, 3]));
    assert_eq!(response.status(), 202);
    let port = adapter.into_port();
    assert_eq!(
        port.calls,
        vec![(HttpMethod::Post, String::from("/opaque"), 3)]
    );
}

#[test]
fn rejects_oversized_request_before_host_port() {
    let mut adapter = HttpAdapter::new(RecordingPort::default(), 2);
    let response = adapter.handle(HttpRequest::new(HttpMethod::Post, "/opaque", &[1, 2, 3]));
    assert_eq!(response.status(), PAYLOAD_TOO_LARGE);
    let port = adapter.into_port();
    assert!(port.calls.is_empty());
}
