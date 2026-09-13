// SPDX-License-Identifier: MIT

//! Source-only boundary for a private seeded-run initialization audit.
//!
//! The types in this module are deliberately host-independent. They describe the
//! evidence an authorized host-thread producer would have to return; they do not
//! inspect host objects, draw randomness, or advertise exact-host support.

mod error;
mod fingerprint;
mod model;
mod port;
mod projection;
mod validation;
mod witness;

pub use error::{RngAuditError, RngAuditReadError, RngAuditUnavailableReason};
pub use model::{
    ExternalInputControl, ExternalInputDeclaration, ExternalInputEvidence, ExternalInputKind,
    GameplayImpact, RngAuditBinding, RngCoverageStatus, RngCursorEvidence, RngSeedOrigin,
    RngSerialization, RngStreamCategory, RngStreamEvidence,
};
pub use port::{RngAuditPort, UnavailableRngAudit};
pub use projection::{
    RngAuditProjection, RngExternalInputProjection, RngStateAvailability, RngStreamProjection,
};
pub use witness::RngAuditWitness;

/// Schema identity for the owner-local private audit evidence.
pub const RNG_AUDIT_SCHEMA: &str = "ascension.rng_audit.v1";
/// Profile identity for the private audit witness.
pub const RNG_AUDIT_PROFILE: &str = "asc-rng-audit-private-v1";
/// Maximum encoded identity/evidence text accepted by this owner-local seam.
pub const RNG_AUDIT_MAX_TEXT_BYTES: usize = 256;
/// Maximum number of streams in one initialization witness.
pub const RNG_AUDIT_MAX_STREAMS: usize = 64;
/// Maximum number of external inputs in one initialization witness.
pub const RNG_AUDIT_MAX_EXTERNAL_INPUTS: usize = 64;
/// Maximum number of call categories attached to one stream.
pub const RNG_AUDIT_MAX_CALL_CATEGORIES: usize = 16;
/// Maximum private canonical witness size.
pub const RNG_AUDIT_MAX_WITNESS_BYTES: usize = 64 * 1024;
/// Domain separator for the private witness fingerprint.
pub const RNG_AUDIT_FINGERPRINT_DOMAIN: &[u8] = b"AI-ASCENSION/RNG-AUDIT/v1\0";
