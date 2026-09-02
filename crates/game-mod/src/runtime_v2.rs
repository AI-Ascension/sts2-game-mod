// SPDX-License-Identifier: MIT

mod artifact;
mod context;
mod contract;
mod dispatch;
mod fake;
mod message;
mod receipt;
mod runtime;
mod support;
mod types;

pub use artifact::{
    RUNTIME_V2_ARTIFACT, RUNTIME_V2_GENERATOR, RUNTIME_V2_MAX_GENERATION,
    RUNTIME_V2_MAX_LEASE_EPOCH, RUNTIME_V2_MAX_TURN_INDEX, RUNTIME_V2_PROTOCOL_VERSION,
    RUNTIME_V2_SCHEMA_DIGEST, RUNTIME_V2_SCHEMA_SOURCE, RuntimeV2ArtifactError,
    verify_runtime_v2_artifact,
};
pub use context::RuntimeV2Context;
pub use contract::{
    RuntimeV2Action, RuntimeV2CombatPhase, RuntimeV2EffectWitness, RuntimeV2Identity,
    RuntimeV2Kind, RuntimeV2Message, RuntimeV2Observation, RuntimeV2Provenance, RuntimeV2Status,
    RuntimeV2ValidationError,
};
pub use fake::{FakeRuntimeV2Game, RuntimeV2GameError, RuntimeV2GamePort};
pub use types::{
    RUNTIME_V2_MAX_QUEUE_CAPACITY, RUNTIME_V2_MAX_RECEIPTS, RUNTIME_V2_MAX_REQUEST_BYTES,
    RuntimeV2Config, RuntimeV2Error, RuntimeV2Mod,
};
