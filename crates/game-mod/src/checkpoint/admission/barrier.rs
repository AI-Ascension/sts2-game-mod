// SPDX-License-Identifier: MIT

use std::cell::Cell;
use std::rc::Rc;

use super::super::error::CheckpointCaptureRejection;

/// Host-reported settlement of the candidate capture boundary.
///
/// The host game thread classifies what it is doing before the owner may validate or snapshot a
/// boundary. Only [`Self::Quiescent`] admits a capture; the other variants are reported so the
/// owner returns an explicit rejection instead of a mixed artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointSettlement {
    /// The admitted action and its host save/effect work have settled; nothing is queued.
    Quiescent,
    /// An admitted effect is still executing.
    MidEffect,
    /// Enemy execution owns the current host turn.
    EnemyExecution,
    /// A pending selection is transitioning between owners.
    PendingSelectionTransition,
    /// The host settlement could not be classified.
    Unknown,
}

impl CheckpointSettlement {
    /// Returns the stable settlement token used in diagnostics and fixtures.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Quiescent => "quiescent",
            Self::MidEffect => "mid_effect",
            Self::EnemyExecution => "enemy_execution",
            Self::PendingSelectionTransition => "pending_selection_transition",
            Self::Unknown => "unknown",
        }
    }

    /// Returns whether this settlement admits a coherent snapshot.
    #[must_use]
    pub const fn is_quiescent(self) -> bool {
        matches!(self, Self::Quiescent)
    }
}

/// Cloneable handle to the host mutation path guarded by [`CheckpointCaptureBarrier`].
///
/// The handle is the only advertised way for host work to reach a gameplay mutation, so a refused
/// [`Self::try_mutate`] is a refused mutation. It is intentionally not `Send`: the barrier, the
/// producer, and the host callbacks that use it stay on the host game thread and only immutable
/// owned bytes leave that thread.
#[derive(Clone, Debug, Default)]
pub struct CheckpointMutationFence {
    held: Rc<Cell<bool>>,
    admitted: Rc<Cell<u64>>,
}

impl CheckpointMutationFence {
    /// Attempts a host mutation, returning `busy` while a capture owns the barrier.
    pub fn try_mutate(&self) -> Result<(), CheckpointCaptureRejection> {
        if self.held.get() {
            return Err(CheckpointCaptureRejection::Busy);
        }
        self.admitted.set(self.admitted.get().saturating_add(1));
        Ok(())
    }

    /// Returns whether a capture currently fences host mutation.
    #[must_use]
    pub fn is_fenced(&self) -> bool {
        self.held.get()
    }

    /// Returns how many host mutations passed the fence.
    #[must_use]
    pub fn admitted_mutations(&self) -> u64 {
        self.admitted.get()
    }
}

/// Owner barrier that excludes host mutation between boundary validation and snapshotting.
///
/// A capture holds the returned [`CheckpointBarrierGuard`] across validation, production, and
/// persistence. While it is held, every [`CheckpointMutationFence::try_mutate`] on a handle from
/// [`Self::fence`] is refused, and a nested hold attempt is refused as `busy`. The barrier never
/// draws RNG, mutates gameplay, or persists anything itself; it only orders owner work against host
/// work. Holding is decided through shared state so a host callback that only has a shared handle
/// can attempt it, and a refusal leaves the state untouched.
#[derive(Debug, Default)]
pub struct CheckpointCaptureBarrier {
    held: Rc<Cell<bool>>,
    admitted: Rc<Cell<u64>>,
}

impl CheckpointCaptureBarrier {
    /// Creates an unheld barrier with no admitted host mutations.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a cloneable host handle fenced by this barrier.
    #[must_use]
    pub fn fence(&self) -> CheckpointMutationFence {
        CheckpointMutationFence {
            held: Rc::clone(&self.held),
            admitted: Rc::clone(&self.admitted),
        }
    }

    /// Returns whether a capture currently holds the barrier.
    #[must_use]
    pub fn is_held(&self) -> bool {
        self.held.get()
    }

    /// Returns how many host mutations passed the barrier.
    #[must_use]
    pub fn admitted_mutations(&self) -> u64 {
        self.admitted.get()
    }

    /// Validates the host settlement and takes the barrier for one capture operation.
    ///
    /// Settlement is classified before mutual exclusion, so an unsafe host phase is never
    /// reported as a mere contention failure.
    ///
    /// # Errors
    ///
    /// Returns [`CheckpointCaptureRejection::UnsafeBoundary`] for any non-quiescent settlement and
    /// [`CheckpointCaptureRejection::Busy`] when another capture already holds the barrier. The
    /// barrier state is unchanged by a rejected attempt.
    pub fn try_hold(
        &self,
        settlement: CheckpointSettlement,
    ) -> Result<CheckpointBarrierGuard, CheckpointCaptureRejection> {
        if !settlement.is_quiescent() {
            return Err(CheckpointCaptureRejection::UnsafeBoundary);
        }
        if self.held.get() {
            return Err(CheckpointCaptureRejection::Busy);
        }
        self.held.set(true);
        Ok(CheckpointBarrierGuard {
            held: Rc::clone(&self.held),
            settlement,
        })
    }
}

/// Exclusive guard held for the whole validate/produce/persist window of one capture.
///
/// Dropping the guard releases the barrier. The guard keeps its own handle to the barrier state so
/// the barrier itself stays movable and a nested hold attempt is expressible.
#[derive(Debug)]
pub struct CheckpointBarrierGuard {
    held: Rc<Cell<bool>>,
    settlement: CheckpointSettlement,
}

impl CheckpointBarrierGuard {
    /// Returns the settlement admitted by [`CheckpointCaptureBarrier::try_hold`].
    #[must_use]
    pub const fn settlement(&self) -> CheckpointSettlement {
        self.settlement
    }
}

impl Drop for CheckpointBarrierGuard {
    fn drop(&mut self) {
        self.held.set(false);
    }
}
