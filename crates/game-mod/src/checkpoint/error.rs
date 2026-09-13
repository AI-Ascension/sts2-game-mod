// SPDX-License-Identifier: MIT

use super::capability::{CheckpointBoundary, CheckpointUnavailableReason};

/// Why a capture request was rejected without producing an artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointCaptureRejection {
    /// The requested phase remains unavailable for the stated reason.
    UnsupportedBoundary {
        /// Requested phase.
        boundary: CheckpointBoundary,
        /// Unavailability gate.
        reason: CheckpointUnavailableReason,
    },
    /// Host effects or transitions make a coherent snapshot unsafe.
    UnsafeBoundary,
    /// Another capture or mutation owns the capture barrier.
    Busy,
    /// Required state coverage is unknown or incomplete.
    UnsupportedCoverage,
    /// Persistence failed; no durable success may be published.
    PersistenceFailed,
    /// The operation identity was already used with a different request.
    OperationConflict,
    /// The supplied bytes are not a non-empty canonical payload.
    InvalidCanonicalBytes,
    /// The supplied canonical payload exceeds the protocol bound.
    Oversize {
        /// Number of bytes supplied.
        bytes: usize,
    },
}

impl CheckpointCaptureRejection {
    /// Returns the stable machine-readable rejection code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnsupportedBoundary { .. } => "unsupported_boundary",
            Self::UnsafeBoundary => "unsafe_boundary",
            Self::Busy => "busy",
            Self::UnsupportedCoverage => "unsupported_coverage",
            Self::PersistenceFailed => "persistence_failed",
            Self::OperationConflict => "operation_conflict",
            Self::InvalidCanonicalBytes => "invalid_canonical_bytes",
            Self::Oversize { .. } => "oversize",
        }
    }
}

impl std::fmt::Display for CheckpointCaptureRejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for CheckpointCaptureRejection {}
