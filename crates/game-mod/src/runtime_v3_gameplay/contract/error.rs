// SPDX-License-Identifier: MIT

/// Deterministic validation failures for the fair-play gameplay profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeV3GameplayValidationError {
    Metadata,
    Provenance,
    InvalidIdentity,
    InvalidText,
    GenerationBounds,
    CollectionBounds,
    /// Two entries in one offered set carry the same identity.
    DuplicateChoice,
    /// A disclosed entry tried to disclose contents of its own.
    OfferedDisclosureTooDeep,
    ObservationShape,
    ActionShape,
    DuplicateAction,
    TransitionShape,
    RecoveryShape,
    ResultShape,
}

impl std::fmt::Display for RuntimeV3GameplayValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Metadata => "runtime-v3-gameplay metadata is unsupported",
            Self::Provenance => "runtime-v3-gameplay provenance is unsupported",
            Self::InvalidIdentity => "runtime-v3-gameplay identity is invalid",
            Self::InvalidText => "runtime-v3-gameplay visible text is invalid",
            Self::GenerationBounds => "runtime-v3-gameplay generation is outside the bound",
            Self::CollectionBounds => "runtime-v3-gameplay collection exceeds its bound",
            Self::DuplicateChoice => "runtime-v3-gameplay offered identities must be unique",
            Self::OfferedDisclosureTooDeep => {
                "runtime-v3-gameplay disclosed contents are bounded to one level"
            }
            Self::ObservationShape => "runtime-v3-gameplay observation is invalid",
            Self::ActionShape => "runtime-v3-gameplay action is invalid",
            Self::DuplicateAction => "runtime-v3-gameplay action IDs must be unique",
            Self::TransitionShape => "runtime-v3-gameplay transition witness is invalid",
            Self::RecoveryShape => "runtime-v3-gameplay recovery request is invalid",
            Self::ResultShape => "runtime-v3-gameplay message shape is invalid",
        })
    }
}

impl std::error::Error for RuntimeV3GameplayValidationError {}
