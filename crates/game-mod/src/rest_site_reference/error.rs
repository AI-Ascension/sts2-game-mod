// SPDX-License-Identifier: MIT

use super::{RestOptionKind, RestReferenceKind, RestUnavailableReason};

/// Failure before an owned rest-site snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestSourceError {
    /// No supported rest-site registry is active for the selected host/build.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for RestSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RestSourceError {}

/// Sanitized failures while producing or reading source-only rest-site reference data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RestSiteError {
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
    /// The manifest does not inventory a rest-site family.
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
    /// A report claims a kind that contradicts the definition family it resolves to.
    ///
    /// This is what keeps a supported option from staying hard-coded unknown: the option's stated
    /// kind must agree with the family of the reference it resolves to.
    KindDisagreesWithDefinition {
        /// Owning rest-site definition identity.
        site_id: String,
        /// Option whose kind disagrees.
        option_id: String,
        /// Kind the source reported.
        reported: RestOptionKind,
        /// Family the referenced definition resolves to.
        family: String,
    },
    /// A rest site offers two options with the same identity.
    DuplicateOption {
        /// Owning rest-site definition identity.
        site_id: String,
        /// Repeated option identity.
        option_id: String,
    },
    /// A rest option repeats a requirement identity.
    DuplicateRequirement {
        /// Owning rest-site definition identity.
        site_id: String,
        /// Option whose requirement repeats.
        option_id: String,
        /// Repeated requirement identity.
        requirement_id: String,
    },
    /// A rest option repeats a cost or limit identity.
    DuplicateCost {
        /// Owning rest-site definition identity.
        site_id: String,
        /// Option whose cost or limit repeats.
        option_id: String,
        /// Repeated cost or limit identity.
        cost_id: String,
    },
    /// A rest option repeats an effect identity.
    DuplicateEffect {
        /// Owning rest-site definition identity.
        site_id: String,
        /// Option whose effect repeats.
        option_id: String,
        /// Repeated effect identity.
        effect_id: String,
    },
    /// A selection requirement repeats a candidate identity.
    DuplicateCandidate {
        /// Owning rest-site definition identity.
        site_id: String,
        /// Option whose candidate repeats.
        option_id: String,
        /// Repeated candidate identity.
        candidate_id: String,
    },
    /// A host-reported option has neither a typed record nor a named coverage record.
    ///
    /// This is what stops a newly audited button from being silently omitted from the reference.
    UncoveredOption {
        /// Owning rest-site definition identity.
        site_id: String,
        /// Option the source reported but did not describe or cover.
        option_id: String,
    },
    /// Availability state and reason contradict each other.
    InvalidAvailability {
        /// Owning rest-site definition identity.
        site_id: String,
        /// Option whose availability is rejected.
        option_id: String,
    },
    /// A rest option declares an impossible effect, healing amount, or modifier.
    InvalidEffect {
        /// Owning rest-site definition identity.
        site_id: String,
        /// Option whose effect is rejected.
        option_id: String,
        /// Effect whose declaration is rejected.
        effect_id: String,
    },
    /// A selection requirement declares an impossible domain, bound, or candidate.
    InvalidSelection {
        /// Owning rest-site definition identity.
        site_id: String,
        /// Option whose selection requirement is rejected.
        option_id: String,
    },
    /// A prospective comparison declares an impossible before/after change.
    InvalidComparison {
        /// Owning rest-site definition identity.
        site_id: String,
        /// Option whose comparison is rejected.
        option_id: String,
        /// Prospective change whose declaration is rejected.
        change_id: String,
    },
    /// A record more visible than its target would disclose a restricted definition.
    ///
    /// The restricted identity is deliberately omitted so the rejection itself cannot disclose it.
    HiddenReferenceLeak {
        /// Owning rest-site definition identity.
        site_id: String,
        /// Reference family whose target is more restricted.
        reference_kind: RestReferenceKind,
    },
    /// A typed reference inside a rest site names an identity absent from the same snapshot.
    DanglingReference {
        /// Owning rest-site definition identity.
        site_id: String,
        /// Reference family.
        reference_kind: RestReferenceKind,
        /// Missing identity.
        id: String,
    },
    /// A list page size is zero or exceeds its local bound.
    InvalidPageSize,
    /// A continuation is stale, reused, or bound to another query.
    InvalidContinuation,
    /// The definition is hidden by the selected visibility scope.
    ExcludedByScope,
    /// A requested field is explicitly unavailable for the stated reason.
    UnavailableField(RestUnavailableReason),
    /// No definition has the requested identity.
    NotFound,
    /// A reference was produced for another manifest/locale/producer.
    StaleReference,
    /// An option reference was produced by an earlier option-set generation.
    StaleOptionSetReference {
        /// Option whose availability was referenced.
        option_id: String,
        /// Generation the reference was bound to.
        referenced: u64,
        /// Generation the catalog currently reports.
        current: u64,
    },
}

impl std::fmt::Display for RestSiteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RestSiteError {}

pub(super) fn map_source_error(error: RestSourceError) -> RestSiteError {
    match error {
        RestSourceError::NoActiveSource => RestSiteError::NoActiveSource,
        RestSourceError::AccessDenied => RestSiteError::SourceAccessDenied,
        RestSourceError::Malformed => RestSiteError::MalformedSource,
    }
}
