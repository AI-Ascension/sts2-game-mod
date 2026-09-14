// SPDX-License-Identifier: MIT

use super::{RewardSemanticReferenceKind, RewardUnavailableReason};

/// Failure before an owned reward snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RewardSourceError {
    /// No supported reward registry is active for the selected host/build.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for RewardSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RewardSourceError {}

/// Sanitized failures while producing or reading source-only reward reference data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RewardCatalogError {
    /// The source failed before an owned snapshot was available.
    NoActiveSource,
    /// The source denied a read.
    SourceAccessDenied,
    /// The source returned malformed data.
    MalformedSource,
    /// The source snapshot names another content manifest.
    ManifestMismatch,
    /// The source snapshot uses another locale.
    LocaleMismatch,
    /// The source snapshot uses another producer identity.
    ProducerVersionMismatch,
    /// The manifest does not inventory a reward family.
    MissingFamily,
    /// The source advertises a family it cannot project.
    UnsupportedFamily,
    /// The source family is temporarily unavailable.
    UnavailableFamily,
    /// The family identity in the source snapshot is wrong.
    FamilyIdentityMismatch,
    /// Source and manifest family counts disagree.
    FamilyCountMismatch,
    /// A source definition is absent from the manifest.
    UnknownDefinition(String),
    /// A manifest definition is absent from the source snapshot.
    MissingDefinition(String),
    /// A source definition repeats an identity.
    DuplicateDefinition(String),
    /// An input field is invalid or exceeds a local collection bound.
    InvalidInput(&'static str),
    /// A definition exceeds the aggregate byte bound.
    DefinitionTooLarge {
        /// Configured aggregate bound.
        limit: usize,
        /// Estimated actual size.
        actual: usize,
    },
    /// A typed reference is absent from the manifest.
    UnknownManifestReference {
        /// Manifest entity family.
        entity_kind: String,
        /// Namespaced definition identity.
        namespaced_id: String,
    },
    /// An item, rule, or selection references an item absent from its reward definition.
    UnknownItemReference {
        /// Owning reward definition identity.
        reward_id: String,
        /// Missing item identity.
        item_id: String,
    },
    /// A definition references a reward absent from the same snapshot or manifest.
    UnknownRewardReference {
        /// Owning reward definition identity.
        reward_id: String,
        /// Missing target reward identity.
        target_id: String,
    },
    /// A selection or generation rule is more visible than an item it references.
    HiddenFutureLeak {
        /// Owning reward definition identity.
        reward_id: String,
        /// Hidden item identity that would have leaked.
        item_id: String,
    },
    /// A record more visible than its target would disclose a restricted reward or item.
    ///
    /// The restricted identity is deliberately omitted so the rejection itself cannot disclose it.
    HiddenReferenceLeak {
        /// Owning reward definition identity.
        reward_id: String,
        /// Reference family whose target is more restricted.
        reference_kind: RewardSemanticReferenceKind,
    },
    /// A defined item is not offered by any generation rule.
    UncoveredItem {
        /// Owning reward definition identity.
        reward_id: String,
        /// Item absent from every generation pool.
        item_id: String,
    },
    /// A defined item is offered by more than one generation rule.
    DuplicateItemMembership {
        /// Owning reward definition identity.
        reward_id: String,
        /// Item offered by multiple generation rules.
        item_id: String,
    },
    /// A list page size is zero or exceeds its local bound.
    InvalidPageSize,
    /// A continuation is stale, reused, or bound to another query.
    InvalidContinuation,
    /// The definition is hidden by the selected visibility scope.
    ExcludedByScope,
    /// A requested field is explicitly unavailable for the stated reason.
    UnavailableField(RewardUnavailableReason),
    /// No definition has the requested identity.
    NotFound,
    /// A reference was produced for another manifest/locale/producer.
    StaleReference,
}

impl std::fmt::Display for RewardCatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RewardCatalogError {}

pub(super) fn map_source_error(error: RewardSourceError) -> RewardCatalogError {
    match error {
        RewardSourceError::NoActiveSource => RewardCatalogError::NoActiveSource,
        RewardSourceError::AccessDenied => RewardCatalogError::SourceAccessDenied,
        RewardSourceError::Malformed => RewardCatalogError::MalformedSource,
    }
}
