// SPDX-License-Identifier: MIT

//! Bounded host-side queue and ABI seams for the game mod.
//!
//! Work is admitted to a bounded FIFO and drained by the owning game thread:
//!
//! ```
//! use sts2_game_mod_host::MainThreadQueue;
//!
//! let mut queue = MainThreadQueue::new(1);
//! assert!(queue.enqueue("main-thread work").is_ok());
//! assert_eq!(queue.drain(1), vec!["main-thread work"]);
//! ```

mod abi;
mod dispatcher;
mod host;
mod queue;

pub use abi::{ABI_VERSION, AbiDescriptor, AbiError, AbiPort, validate_abi};
pub use dispatcher::HostDispatcher;
pub use host::{HostError, HostPort, HostReceipt, HostRequest, HostSnapshot};
pub use queue::{MainThreadQueue, QueueError};
