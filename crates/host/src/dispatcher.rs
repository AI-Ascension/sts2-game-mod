// SPDX-License-Identifier: MIT

use crate::{HostError, HostPort, HostReceipt, HostRequest, MainThreadQueue, QueueError};

/// Coordinates bounded admission and deterministic main-thread draining.
#[derive(Debug)]
pub struct HostDispatcher<H> {
    host: H,
    queue: MainThreadQueue<HostRequest>,
    main_thread_budget: usize,
}

impl<H> HostDispatcher<H> {
    /// Creates a dispatcher with bounded capacity and a per-pump work budget.
    #[must_use]
    pub fn new(host: H, queue_capacity: usize, main_thread_budget: usize) -> Self {
        Self {
            host,
            queue: MainThreadQueue::new(queue_capacity),
            main_thread_budget,
        }
    }

    /// Attempts to admit one request without silently dropping it.
    pub fn enqueue(&mut self, request: HostRequest) -> Result<(), QueueError> {
        self.queue.enqueue(request)
    }

    /// Stops new admissions while retaining queued work.
    pub fn close(&mut self) {
        self.queue.close();
    }

    /// Reports the number of requests waiting for the host thread.
    #[must_use]
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// Reads the host projection through its explicit port.
    pub fn snapshot(&self) -> Result<crate::HostSnapshot, HostError>
    where
        H: HostPort,
    {
        self.host.snapshot()
    }

    /// Drains the configured work budget on the caller's host thread.
    pub fn pump_main_thread(&mut self) -> Vec<Result<HostReceipt, HostError>>
    where
        H: HostPort,
    {
        self.pump_with_budget(self.main_thread_budget)
    }

    /// Drains an explicit deterministic budget, primarily for tests and host schedulers.
    pub fn pump_with_budget(&mut self, budget: usize) -> Vec<Result<HostReceipt, HostError>>
    where
        H: HostPort,
    {
        self.queue
            .drain(budget)
            .into_iter()
            .map(|request| self.host.submit(request))
            .collect()
    }

    /// Returns the owned host adapter after the dispatcher is no longer needed.
    #[must_use]
    pub fn into_host(self) -> H {
        self.host
    }
}
