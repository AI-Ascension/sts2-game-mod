// SPDX-License-Identifier: MIT

//! Minimal, reviewed client for the game-mod `seeded-run-v1` runtime route.
//!
//! It assembles a canonical `start_request` (computing the selected-context digest locally and
//! reading the schema digest from the contract file) and sends it to the authenticated runtime
//! listener. Host-derived values (lease/session identities, profile baseline, compatibility
//! digests) are supplied by the operator; this tool does not invent or fabricate them.

pub mod context;
pub mod request;
pub mod wire;

#[cfg(test)]
mod tests;

pub use context::{CanonicalContext, ClientError, Compatibility, IdentityDigest, ProfileBaseline};
pub use request::{StartRequest, StartRequestParams, build_start_request};
