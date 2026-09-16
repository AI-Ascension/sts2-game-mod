// SPDX-License-Identifier: MIT

mod engine;
mod fixtures;

pub(crate) use engine::{
    RecordingApplier, SharedOwner, StoreState, create_engine, current_owner, owner_with_generation,
    send, send_blob,
};
#[cfg(target_os = "linux")]
pub(crate) use fixtures::TestDirectory;
pub(crate) use fixtures::{
    Fixture, authorization_for, chunk_payload, hex_digest, request, sha256, uuid,
};

pub(crate) const PRINCIPAL: &str = "fixture-harness";
