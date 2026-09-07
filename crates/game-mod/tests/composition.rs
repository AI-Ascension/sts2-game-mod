// SPDX-License-Identifier: MIT

use sts2_game_mod::ModRuntime;
use sts2_game_mod_host::{
    AbiDescriptor, AbiPort, HostError, HostPort, HostReceipt, HostRequest, HostSnapshot,
};
use sts2_game_mod_http_adapter::{HttpMethod, HttpRequest, PAYLOAD_TOO_LARGE};

#[derive(Debug, Default)]
struct FakeHost {
    generation: u64,
    requests: Vec<HostRequest>,
}

impl HostPort for FakeHost {
    fn snapshot(&self) -> Result<HostSnapshot, HostError> {
        Ok(HostSnapshot {
            generation: self.generation,
            is_ready: true,
        })
    }

    fn submit(&mut self, request: HostRequest) -> Result<HostReceipt, HostError> {
        self.generation += 1;
        self.requests.push(request.clone());
        Ok(HostReceipt {
            request_id: request.request_id,
            generation: self.generation,
        })
    }
}

#[derive(Debug)]
struct FakeAbi(AbiDescriptor);

impl AbiPort for FakeAbi {
    fn descriptor(&self) -> AbiDescriptor {
        self.0
    }
}

#[test]
fn http_admission_reaches_fake_host_only_after_main_thread_pump() {
    let mut runtime = ModRuntime::new(FakeHost::default(), 2, 1, 8);
    let response = runtime.handle_http(HttpRequest::new(HttpMethod::Post, "/opaque", &[4, 5]));
    assert_eq!(response.status(), 202);
    assert_eq!(runtime.queue_len(), 1);
    assert_eq!(
        runtime.pump_main_thread(),
        vec![Ok(HostReceipt {
            request_id: 0,
            generation: 1,
        })]
    );
    assert_eq!(
        runtime.snapshot(),
        Ok(HostSnapshot {
            generation: 1,
            is_ready: true,
        })
    );
}

#[test]
fn body_limit_and_abi_gate_fail_closed() {
    let mut runtime = ModRuntime::new(FakeHost::default(), 2, 1, 2);
    let response = runtime.handle_http(HttpRequest::new(HttpMethod::Post, "/opaque", &[1, 2, 3]));
    assert_eq!(response.status(), PAYLOAD_TOO_LARGE);
    assert_eq!(runtime.queue_len(), 0);
    assert_eq!(
        runtime.validate_native_abi(&FakeAbi(AbiDescriptor::current())),
        Ok(())
    );
}

#[test]
fn close_rejects_http_admission_without_dropping_existing_work() {
    let mut runtime = ModRuntime::new(FakeHost::default(), 2, 2, 8);
    assert_eq!(
        runtime.enqueue_host_request(HostRequest::new(41, [9])),
        Ok(())
    );
    runtime.close();
    let response = runtime.handle_http(HttpRequest::new(HttpMethod::Post, "/opaque", &[7]));
    assert_eq!(response.status(), 503);
    assert_eq!(runtime.queue_len(), 1);
    assert_eq!(
        runtime.pump_main_thread(),
        vec![Ok(HostReceipt {
            request_id: 41,
            generation: 1,
        })]
    );
}
