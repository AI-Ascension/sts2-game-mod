// SPDX-License-Identifier: MIT

use super::{ShopItemKind, ShopReferenceKind, ShopUnavailableReason};

/// Failure before an owned shop snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopSourceError {
    /// No supported shop registry is active for the selected host/build.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for ShopSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ShopSourceError {}

/// Sanitized failures while producing or reading source-only shop reference data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShopCatalogError {
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
    /// The manifest does not inventory a shop family.
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
    /// This is what keeps a supported item from staying hard-coded unknown: the entry's stated
    /// kind must agree with the family of the definition it points at.
    KindDisagreesWithDefinition {
        /// Owning shop definition identity.
        shop_id: String,
        /// Entry whose kind disagrees.
        entry_id: String,
        /// Kind the source reported.
        reported: ShopItemKind,
        /// Family the referenced definition resolves to.
        family: String,
    },
    /// A shop offers two entries with the same identity.
    DuplicateEntry {
        /// Owning shop definition identity.
        shop_id: String,
        /// Repeated entry identity.
        entry_id: String,
    },
    /// A shop defines two services with the same identity.
    DuplicateService {
        /// Owning shop definition identity.
        shop_id: String,
        /// Repeated service identity.
        service_id: String,
    },
    /// An entry price is impossible: negative amount, missing currency, or contradictory pairing.
    InvalidPrice {
        /// Owning shop definition identity.
        shop_id: String,
        /// Entry whose price is rejected.
        entry_id: String,
    },
    /// Sale or stacked-discount state is impossible for the reported price or stock.
    InvalidDiscount {
        /// Owning shop definition identity.
        shop_id: String,
        /// Entry whose discount is rejected.
        entry_id: String,
    },
    /// A discount or pricing contributor repeats an identity.
    DuplicateContributor {
        /// Owning shop definition identity.
        shop_id: String,
        /// Entry or service whose contributor repeats.
        owner_id: String,
        /// Repeated contributor identity.
        contributor_id: String,
    },
    /// A service declares an impossible selection domain, limit, or prospective change.
    InvalidService {
        /// Owning shop definition identity.
        shop_id: String,
        /// Service whose declaration is rejected.
        service_id: String,
    },
    /// A restock rule declares an impossible or non-monotonic generation.
    InvalidRestock {
        /// Owning shop definition identity.
        shop_id: String,
        /// Rule whose generation is rejected.
        rule_id: String,
    },
    /// A record more visible than its target would disclose a restricted definition.
    ///
    /// The restricted identity is deliberately omitted so the rejection itself cannot disclose it.
    HiddenReferenceLeak {
        /// Owning shop definition identity.
        shop_id: String,
        /// Reference family whose target is more restricted.
        reference_kind: ShopReferenceKind,
    },
    /// A typed reference inside a shop names an identity absent from the same snapshot.
    DanglingReference {
        /// Owning shop definition identity.
        shop_id: String,
        /// Reference family.
        reference_kind: ShopReferenceKind,
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
    UnavailableField(ShopUnavailableReason),
    /// No definition has the requested identity.
    NotFound,
    /// A reference was produced for another manifest/locale/producer.
    StaleReference,
    /// A stock reference was produced by an earlier restock generation.
    StaleStockReference {
        /// Entry whose stock was referenced.
        entry_id: String,
        /// Generation the reference was bound to.
        referenced: u64,
        /// Generation the catalog currently reports.
        current: u64,
    },
}

impl std::fmt::Display for ShopCatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ShopCatalogError {}

pub(super) fn map_source_error(error: ShopSourceError) -> ShopCatalogError {
    match error {
        ShopSourceError::NoActiveSource => ShopCatalogError::NoActiveSource,
        ShopSourceError::AccessDenied => ShopCatalogError::SourceAccessDenied,
        ShopSourceError::Malformed => ShopCatalogError::MalformedSource,
    }
}
