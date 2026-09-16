// SPDX-License-Identifier: MIT

//! Bounded exact-restore staging owned by the game-mod target.
//!
//! The production host remains unavailable: no exact local owner provider or host restore port
//! exists. A production begin therefore refuses before opening durable storage or accepting bytes.
//! The staged engine is exercised with explicit synthetic owner and applier ports only.

mod engine;
mod private_store;
mod wire;

pub use engine::{
    ExactRestoreAuthorization, ExactRestoreCapability, ExactRestoreCurrentOwner,
    ExactRestoreEngine, ExactRestoreError, ExactRestoreOwnerFence, ExactRestoreStore,
    ExactRestoreUnavailableOwner, NoExactRestoreApplier, RestoreApplyOutcome, RestoreClosureView,
    RestoreHostApplier, RestoreOwnerProvider, exact_restore_unavailable_response,
};
pub use private_store::SecureExactRestoreStore;

/// Exact-restore protocol schema digest copied from sts2-protocol main `5d5a368`.
pub const EXACT_RESTORE_SCHEMA_DIGEST: &str =
    "2289d888c33eac46873408303c4423eab762e3f7bd6132ae8ae88d0d3b1858e4";
/// Maximum complete exact-restore request or response frame.
pub const EXACT_RESTORE_MAX_FRAME_BYTES: usize = 16_384;
/// Maximum encoded base64 chunk member.
pub const EXACT_RESTORE_MAX_CHUNK_BASE64_BYTES: usize = 10_924;
/// Maximum decoded chunk size.
pub const EXACT_RESTORE_MAX_CHUNK_BYTES: usize = 8_192;
/// Maximum artifact or manifest byte length.
pub const EXACT_RESTORE_MAX_BLOB_BYTES: u64 = 16 * 1024 * 1024;
/// Maximum closure bytes, counting manifest once and distinct canonical/restore blobs.
pub const EXACT_RESTORE_MAX_CLOSURE_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum manifest artifact references including aliases.
pub const EXACT_RESTORE_MAX_REFERENCES: usize = 64;
/// Maximum terminal receipts retained without eviction.
pub const EXACT_RESTORE_MAX_TERMINAL_RECEIPTS: usize = 256;

const SCHEMA_JSON: &str =
    include_str!("../../../../protocol-artifact/exact-restore-v1/schema.json");
