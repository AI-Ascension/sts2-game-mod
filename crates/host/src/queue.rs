// SPDX-License-Identifier: MIT

use std::collections::VecDeque;

/// Failure returned when main-thread work cannot be admitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueueError {
    /// The queue has been closed and rejects new work.
    Closed,
    /// The queue is at capacity.
    Full { capacity: usize },
}

/// Bounded FIFO storage for work that must execute on the game main thread.
#[derive(Debug)]
pub struct MainThreadQueue<T> {
    capacity: usize,
    closed: bool,
    items: VecDeque<T>,
}

impl<T> MainThreadQueue<T> {
    /// Creates a queue with the supplied item capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            closed: false,
            items: VecDeque::with_capacity(capacity),
        }
    }

    /// Attempts to append one item without dropping an earlier item.
    pub fn enqueue(&mut self, item: T) -> Result<(), QueueError> {
        if self.closed {
            return Err(QueueError::Closed);
        }
        if self.items.len() >= self.capacity {
            return Err(QueueError::Full {
                capacity: self.capacity,
            });
        }
        self.items.push_back(item);
        Ok(())
    }

    /// Stops admission while retaining already queued work for draining.
    pub fn close(&mut self) {
        self.closed = true;
    }

    /// Reports whether new work is rejected.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.closed
    }

    /// Reports the current item count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Reports whether no work is waiting.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Reports the configured item capacity.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Removes at most budget items in FIFO order.
    pub fn drain(&mut self, budget: usize) -> Vec<T> {
        let count = budget.min(self.items.len());
        (0..count).filter_map(|_| self.items.pop_front()).collect()
    }

    /// Removes the first queued item matching a predicate without changing FIFO order for the rest.
    pub fn remove_matching(&mut self, mut predicate: impl FnMut(&T) -> bool) -> Option<T> {
        let index = self.items.iter().position(&mut predicate)?;
        self.items.remove(index)
    }
}
