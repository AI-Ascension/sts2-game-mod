// SPDX-License-Identifier: MIT

use sts2_mod_host::{
    AbiError, AbiPort, HostDispatcher, HostError, HostPort, HostReceipt, HostRequest, QueueError,
    validate_abi,
};
use sts2_mod_http_adapter::{HttpAdapter, HttpPort, HttpRequest, HttpResponse};

mod poc;
mod protocol_artifact;

pub use poc::{
    EffectWitness, POC_MAX_EVIDENCE_RECORDS, POC_MAX_REQUEST_BYTES, PocAction, PocBoundaryRecord,
    PocCoreError, PocCorePort, PocCoreState, PocMessage, PocMessageKind, PocMod, PocModError,
    PocObservation, PocProvenance, PocRoute, PocStatus, PocValidationError,
};
pub use protocol_artifact::{
    ArtifactError, POC_ARTIFACT, POC_MAX_GENERATION, POC_PROTOCOL_VERSION, POC_SCHEMA_DIGEST,
    POC_SCHEMA_PACKAGE,
};

/// Target-owned composition of host admission, HTTP bounds, and ABI validation.
#[derive(Debug)]
pub struct ModRuntime<H> {
    dispatcher: HostDispatcher<H>,
    max_body_bytes: usize,
    next_request_id: u64,
}

impl<H> ModRuntime<H> {
    /// Creates a runtime seam with explicit queue, pump, and request-body bounds.
    #[must_use]
    pub fn new(
        host: H,
        queue_capacity: usize,
        main_thread_budget: usize,
        max_body_bytes: usize,
    ) -> Self {
        Self {
            dispatcher: HostDispatcher::new(host, queue_capacity, main_thread_budget),
            max_body_bytes,
            next_request_id: 0,
        }
    }

    /// Admits an already-owned host request without running host code.
    pub fn enqueue_host_request(&mut self, request: HostRequest) -> Result<(), QueueError> {
        self.dispatcher.enqueue(request)
    }

    /// Runs the configured amount of work on the caller's game thread.
    pub fn pump_main_thread(&mut self) -> Vec<Result<HostReceipt, HostError>>
    where
        H: HostPort,
    {
        self.dispatcher.pump_main_thread()
    }

    /// Returns a host-owned state projection.
    pub fn snapshot(&self) -> Result<sts2_mod_host::HostSnapshot, HostError>
    where
        H: HostPort,
    {
        self.dispatcher.snapshot()
    }

    /// Validates an ABI provider before it is used by this runtime.
    pub fn validate_native_abi<P>(&self, port: &P) -> Result<(), AbiError>
    where
        P: AbiPort,
    {
        validate_abi(port)
    }

    /// Applies the HTTP body bound and admits one opaque host request.
    ///
    /// This is an initialization seam, not a public route catalog. Route and payload semantics
    /// belong in a later owner-local contract with deterministic fixtures.
    pub fn handle_http(&mut self, request: HttpRequest<'_>) -> HttpResponse
    where
        H: HostPort,
    {
        let max_body_bytes = self.max_body_bytes;
        let port = DispatcherPort {
            dispatcher: &mut self.dispatcher,
            next_request_id: &mut self.next_request_id,
        };
        let mut adapter = HttpAdapter::new(port, max_body_bytes);
        adapter.handle(request)
    }

    /// Reports queued work awaiting the main thread.
    #[must_use]
    pub fn queue_len(&self) -> usize {
        self.dispatcher.queue_len()
    }

    /// Closes admission while retaining already queued work.
    pub fn close(&mut self) {
        self.dispatcher.close();
    }
}

struct DispatcherPort<'a, H> {
    dispatcher: &'a mut HostDispatcher<H>,
    next_request_id: &'a mut u64,
}

impl<H: HostPort> HttpPort for DispatcherPort<'_, H> {
    fn dispatch(&mut self, request: HttpRequest<'_>) -> HttpResponse {
        let request_id = *self.next_request_id;
        *self.next_request_id += 1;
        let host_request = HostRequest::new(request_id, request.body());
        let status = match self.dispatcher.enqueue(host_request) {
            Ok(()) => 202,
            Err(QueueError::Full { .. }) => 429,
            Err(QueueError::Closed) => 503,
        };
        HttpResponse::new(status, [])
    }
}
