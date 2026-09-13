// SPDX-License-Identifier: MIT

use serde::Serialize;

use super::{ExternalInputEvidence, RngAuditWitness, RngCursorEvidence, RngStreamEvidence};

/// Projection-safe state marker; it never carries a cursor or raw state digest.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RngStateAvailability {
    /// An opaque state digest and cursor were privately verified.
    Known,
    /// The producer verified the canonical zero state.
    KnownZero,
    /// No private state evidence was available.
    Unavailable,
}

/// Projection of one private stream without raw state or cursor evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RngStreamProjection {
    /// Stable stream identity.
    pub stream_id: String,
    /// Audit category.
    pub category: String,
    /// Native owner/type identity.
    pub owner: String,
    /// Algorithm/version when observable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub algorithm_version: Option<String>,
    /// Seed-origin kind without independent source details.
    pub seed_origin: String,
    /// Projection-safe state availability.
    pub state: RngStateAvailability,
    /// Creation lifecycle boundary.
    pub creation_boundary: String,
    /// Reset lifecycle boundary.
    pub reset_boundary: String,
    /// Stable call-category inventory.
    pub call_categories: Vec<String>,
    /// Serialization support classification.
    pub serialization: String,
    /// Gameplay relevance.
    pub gameplay: String,
}

/// Projection of one external source without its evidence note.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RngExternalInputProjection {
    /// Input category.
    pub kind: String,
    /// Owner/source identity.
    pub owner: String,
    /// Gameplay relevance.
    pub gameplay: String,
    /// Control classification.
    pub control: String,
}

/// Negotiable capability/provenance metadata for a validated witness.
///
/// This value is suitable for a future control-plane adapter. It intentionally
/// omits state digests, cursors, independent seed source details, and evidence
/// notes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RngAuditProjection {
    /// Owner-local schema identity.
    pub schema: &'static str,
    /// Owner-local profile identity.
    pub profile: &'static str,
    /// Exact game/build binding.
    pub game_build: String,
    /// Adapter compatibility binding.
    pub adapter_compatibility: String,
    /// Supported mode binding.
    pub supported_mode: String,
    /// Profile compatibility binding.
    pub profile_compatibility: String,
    /// Content manifest identity, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_manifest: Option<String>,
    /// Canonical requested/read-back run seed.
    pub canonical_seed: String,
    /// Master-seed derivation version.
    pub seed_derivation_version: String,
    /// First stable seeded lifecycle boundary.
    pub seeded_boundary: String,
    /// Complete inventory status.
    pub coverage: String,
    /// External-input declaration.
    pub external_input_declaration: String,
    /// Sorted projection-safe stream inventory.
    pub streams: Vec<RngStreamProjection>,
    /// Sorted external-input inventory.
    pub external_inputs: Vec<RngExternalInputProjection>,
}

impl RngAuditProjection {
    pub(super) fn from_witness(witness: &RngAuditWitness) -> Self {
        let binding = witness.binding();
        Self {
            schema: super::RNG_AUDIT_SCHEMA,
            profile: super::RNG_AUDIT_PROFILE,
            game_build: binding.game_build.clone(),
            adapter_compatibility: binding.adapter_compatibility.clone(),
            supported_mode: binding.supported_mode.clone(),
            profile_compatibility: binding.profile_compatibility.clone(),
            content_manifest: binding.content_manifest.clone(),
            canonical_seed: binding.canonical_seed.clone(),
            seed_derivation_version: binding.seed_derivation_version.clone(),
            seeded_boundary: binding.seeded_boundary.clone(),
            coverage: witness.coverage().code().to_owned(),
            external_input_declaration: binding.external_input_declaration.code().to_owned(),
            streams: witness.streams.iter().map(stream_projection).collect(),
            external_inputs: witness
                .external_inputs
                .iter()
                .map(external_projection)
                .collect(),
        }
    }

    /// Encodes the projection for synthetic contract checks.
    pub fn to_json_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

fn stream_projection(stream: &RngStreamEvidence) -> RngStreamProjection {
    RngStreamProjection {
        stream_id: stream.stream_id.clone(),
        category: stream.category.code().to_owned(),
        owner: stream.owner.clone(),
        algorithm_version: stream.algorithm_version.clone(),
        seed_origin: stream.seed_origin.kind().to_owned(),
        state: state_availability(&stream.initial_state),
        creation_boundary: stream.creation_boundary.clone(),
        reset_boundary: stream.reset_boundary.clone(),
        call_categories: stream.call_categories.clone(),
        serialization: stream.serialization.code().to_owned(),
        gameplay: stream.gameplay.code().to_owned(),
    }
}

fn external_projection(input: &ExternalInputEvidence) -> RngExternalInputProjection {
    RngExternalInputProjection {
        kind: input.kind.code().to_owned(),
        owner: input.owner.clone(),
        gameplay: input.gameplay.code().to_owned(),
        control: input.control.code().to_owned(),
    }
}

fn state_availability(state: &RngCursorEvidence) -> RngStateAvailability {
    match state {
        RngCursorEvidence::Known { .. } => RngStateAvailability::Known,
        RngCursorEvidence::KnownZero { .. } => RngStateAvailability::KnownZero,
        RngCursorEvidence::Unavailable => RngStateAvailability::Unavailable,
    }
}
