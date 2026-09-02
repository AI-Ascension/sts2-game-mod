// SPDX-License-Identifier: MIT

use sts2_mod_host::{
    ABI_VERSION, AbiDescriptor, AbiError, AbiPort, HostDispatcher, HostError, HostPort,
    HostReceipt, HostRequest, HostSnapshot, MainThreadQueue, QueueError, validate_abi,
};

#[derive(Debug, Default)]
struct FakeHost {
    generation: u64,
    requests: Vec<HostRequest>,
    ready: bool,
}

impl HostPort for FakeHost {
    fn snapshot(&self) -> Result<HostSnapshot, HostError> {
        Ok(HostSnapshot {
            generation: self.generation,
            ready: self.ready,
        })
    }

    fn submit(&mut self, request: HostRequest) -> Result<HostReceipt, HostError> {
        if !self.ready {
            return Err(HostError::NotReady);
        }
        self.generation += 1;
        let receipt = HostReceipt {
            request_id: request.request_id,
            generation: self.generation,
        };
        self.requests.push(request);
        Ok(receipt)
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
fn bounded_queue_rejects_overflow_and_preserves_fifo() {
    let mut queue = MainThreadQueue::new(2);
    assert_eq!(queue.enqueue(10), Ok(()));
    assert_eq!(queue.enqueue(20), Ok(()));
    assert_eq!(queue.enqueue(30), Err(QueueError::Full { capacity: 2 }));
    assert_eq!(queue.drain(1), vec![10]);
    assert_eq!(queue.drain(8), vec![20]);
}

#[test]
fn dispatcher_uses_fake_host_on_a_deterministic_budget() {
    let fake = FakeHost {
        ready: true,
        ..FakeHost::default()
    };
    let mut dispatcher = HostDispatcher::new(fake, 4, 1);
    assert_eq!(dispatcher.enqueue(HostRequest::new(7, [1, 2])), Ok(()));
    assert_eq!(dispatcher.enqueue(HostRequest::new(8, [3, 4])), Ok(()));
    assert_eq!(
        dispatcher.pump_main_thread(),
        vec![Ok(HostReceipt {
            request_id: 7,
            generation: 1,
        })]
    );
    assert_eq!(dispatcher.queue_len(), 1);
    assert_eq!(
        dispatcher.pump_with_budget(4),
        vec![Ok(HostReceipt {
            request_id: 8,
            generation: 2,
        })]
    );
}

#[test]
fn abi_validation_accepts_current_and_rejects_version_drift() {
    assert_eq!(validate_abi(&FakeAbi(AbiDescriptor::current())), Ok(()));
    assert_eq!(
        validate_abi(&FakeAbi(AbiDescriptor {
            version: ABI_VERSION + 1,
            ..AbiDescriptor::current()
        })),
        Err(AbiError::VersionMismatch {
            expected: ABI_VERSION,
            actual: ABI_VERSION + 1,
        })
    );
}

#[test]
fn closed_queue_rejects_new_work() {
    let mut queue = MainThreadQueue::new(1);
    queue.close();
    assert_eq!(queue.enqueue(1), Err(QueueError::Closed));
}
