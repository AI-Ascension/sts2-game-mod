// SPDX-License-Identifier: MIT

use crate::ContentCursorBinding;

use super::ShopText;

/// Manifest family handled by this owner-local producer.
pub const SHOP_REFERENCE_ENTITY_KIND: &str = "shop";
/// Manifest family resolved for card entries and card removal services.
pub const SHOP_REFERENCE_CARD_KIND: &str = "card";
/// Manifest family resolved for relic entries.
pub const SHOP_REFERENCE_RELIC_KIND: &str = "relic";
/// Manifest family resolved for potion entries.
pub const SHOP_REFERENCE_POTION_KIND: &str = "potion";
/// Manifest family resolved for shop service definitions.
pub const SHOP_REFERENCE_SERVICE_KIND: &str = "shop-service";
/// Manifest family resolved for currency units.
pub const SHOP_REFERENCE_CURRENCY_KIND: &str = "currency";
/// Source-only producer identity; this is not a wire or native ABI version.
pub const SHOP_REFERENCE_PRODUCER_VERSION: &str = "game-shop-reference-producer-v1";
/// Maximum bytes accepted for one owner-defined identity.
pub const SHOP_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized or owner-defined text value.
pub const SHOP_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aggregate bytes retained for one shop definition.
pub const SHOP_MAX_DEFINITION_BYTES: usize = 128 * 1024;
/// Maximum shop definitions in one source snapshot.
pub const SHOP_MAX_DEFINITIONS: usize = 1_024;
/// Maximum entries returned by one bounded shop or entry page.
pub const SHOP_MAX_PAGE_ITEMS: usize = 64;
/// Maximum inventory entries on one shop definition.
pub const SHOP_MAX_ENTRIES: usize = 64;
/// Maximum services on one shop definition.
pub const SHOP_MAX_SERVICES: usize = 32;
/// Maximum pricing contributors on one entry or service price.
pub const SHOP_MAX_CONTRIBUTORS: usize = 32;
/// Maximum stacked discount tiers on one entry.
pub const SHOP_MAX_DISCOUNT_TIERS: usize = 8;
/// Maximum requirements on one restriction or service eligibility list.
pub const SHOP_MAX_REQUIREMENTS: usize = 32;
/// Maximum restrictions on one entry.
pub const SHOP_MAX_RESTRICTIONS: usize = 32;
/// Maximum limits on one service.
pub const SHOP_MAX_SERVICE_LIMITS: usize = 16;
/// Maximum selection candidates on one service.
pub const SHOP_MAX_SELECTION_CANDIDATES: usize = 64;
/// Maximum restock rules on one shop definition.
pub const SHOP_MAX_RESTOCK_RULES: usize = 32;
/// Maximum outstanding continuations a reader retains per list.
pub const SHOP_MAX_CONTINUATIONS: usize = 64;
/// Maximum unresolved formula inputs retained for one price.
pub const SHOP_MAX_UNRESOLVED_INPUTS: usize = 32;
/// Maximum semantic references on one owner record.
pub const SHOP_MAX_REFERENCES: usize = 128;
/// Highest accepted discount percentage.
pub const SHOP_MAX_DISCOUNT_PERCENT: u32 = 100;

/// Static catalog identity: content manifest, locale, and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShopCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized shop value.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

/// Exact static shop definition reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShopDefinitionReference {
    /// Catalog witness that owns this shop identity.
    pub catalog: ShopCatalogBinding,
    /// Namespaced shop definition identity.
    pub shop_id: String,
}

/// Exact static inventory entry reference scoped by its shop definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShopEntryReference {
    /// Catalog witness that owns this entry identity.
    pub catalog: ShopCatalogBinding,
    /// Owning shop definition identity.
    pub shop_id: String,
    /// Stable inventory entry identity.
    pub entry_id: String,
}

/// Exact static service reference scoped by its shop definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShopServiceReference {
    /// Catalog witness that owns this service identity.
    pub catalog: ShopCatalogBinding,
    /// Owning shop definition identity.
    pub shop_id: String,
    /// Exact-build service identity.
    pub service_id: String,
}

/// Live shop stock reference fenced by the restock generation that produced it.
///
/// A generation-bound reference is the only way this slice lets a caller point at observed stock:
/// after a restock replaces inventory, the earlier reference no longer describes the current
/// shop and is refused instead of being answered with newer stock.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShopStockReference {
    /// Exact static entry reference.
    pub entry: ShopEntryReference,
    /// Restock generation the caller observed.
    pub generation: u64,
}

/// Live run/instance/epoch fence for one observed shop.
///
/// This source-only slice does not read a live shop; the type exists so a future live reader cannot
/// confuse a run/instance identity with a static definition identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShopSnapshotReference {
    /// Observed run identity.
    pub run_id: String,
    /// Observed game instance identity.
    pub instance_id: String,
    /// Observed run epoch.
    pub epoch: u64,
    /// Observed coherent snapshot identity.
    pub snapshot_id: String,
}

/// Live inventory entry instance identity, deliberately distinct from any static entry ID.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShopEntryInstanceReference {
    /// Observed run/instance/epoch/snapshot fence.
    pub snapshot: ShopSnapshotReference,
    /// Observed live entry instance identity.
    pub entry_instance_id: String,
}

/// Transient purchase-action identity, distinct from a static entry or service identity.
///
/// The host assigns action identities for one rendered shop; they are never stable definition
/// identities, and this slice neither resolves nor performs them.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShopPurchaseActionReference {
    /// Observed run/instance/epoch/snapshot fence.
    pub snapshot: ShopSnapshotReference,
    /// Observed transient action identity.
    pub action_id: String,
}

/// Typed semantic reference with an explicit family.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShopSemanticReference {
    /// Referenced family.
    pub kind: ShopReferenceKind,
    /// Referenced identity.
    pub id: String,
    /// Localized label for the referenced identity.
    pub label: ShopText,
}

/// Family of a typed shop reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopReferenceKind {
    /// Another shop definition.
    Shop,
    /// A card definition.
    Card,
    /// A relic definition.
    Relic,
    /// A potion definition.
    Potion,
    /// A shop service definition.
    Service,
    /// A currency unit definition.
    Currency,
    /// An inventory entry inside a shop definition.
    Entry,
    /// A restock rule inside a shop definition.
    RestockRule,
    /// Any other manifest family named by the source.
    Content {
        /// Manifest entity family.
        entity_kind: String,
    },
    /// Source could not classify the reference.
    Unknown,
}

/// Owner-defined shop definition visibility.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopVisibility {
    /// Visible on the supported public reference surface.
    Visible,
    /// Visible only in an explicitly owner-authorized scope.
    OwnerOnly,
    /// Source knows a value exists but must not reveal it.
    Hidden,
    /// Source could not classify visibility.
    Unknown,
}

/// Scope requested by a static shop reference query.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopVisibilityScope {
    /// Publicly visible and unlocked definitions.
    Public,
    /// Public definitions plus locked references.
    Reference,
    /// Explicit owner-authorized scope.
    Owner,
}

/// Evidence label for a static shop fact.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopEvidence {
    /// The owner source exposes the value as an authoritative definition.
    Authoritative,
    /// The value was copied from a source-owned definition without runtime verification.
    SourceDerived,
    /// The value was independently authored and is not a host claim.
    IndependentlyAuthored,
    /// The value was observed on a visible surface.
    Observed,
    /// Evidence is insufficient to classify the value as authoritative.
    Unverified,
}

/// Kinds of item a shop inventory entry can offer.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopItemKind {
    /// A card definition.
    Card,
    /// A relic definition.
    Relic,
    /// A potion definition.
    Potion,
    /// A service offered as an inventory entry.
    Service,
    /// Owner-defined item kind.
    Custom(String),
    /// A kind is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the entry.
    Unknown,
}

/// Kinds of service a shop can offer.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopServiceKind {
    /// Remove a card from the deck.
    CardRemoval,
    /// Upgrade a card in the deck.
    CardUpgrade,
    /// Transform a card into another card.
    CardTransform,
    /// Owner-defined service kind.
    Custom(String),
    /// A kind is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the service.
    Unknown,
}

/// Observed stock state of one inventory entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShopStockState {
    /// The entry is purchasable; the quantity may be explicitly unavailable.
    InStock {
        /// Remaining purchasable quantity.
        quantity: super::ShopField<u32>,
    },
    /// The entry is sold out for this restock generation.
    SoldOut,
    /// The entry exists but is restricted by capacity or eligibility.
    Restricted,
    /// The entry is not offered by the current generation.
    NotOffered,
    /// Source could not classify the stock state.
    Unknown,
}

/// Source support state for the shop reference family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopFamilyState {
    /// Source can project all shop records.
    Handled,
    /// Family exists but no typed source adapter is available.
    Unsupported,
    /// Family is known but currently unavailable.
    Unavailable,
}

/// Static family coverage, including explicit unsupported/unavailable states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopFamilyCoverage {
    /// Family identity.
    pub entity_kind: String,
    /// Source support state.
    pub state: ShopFamilyState,
    /// Number of shop definitions in the manifest.
    pub definition_count: usize,
}

/// Relative permissiveness of a visibility label for follow-up leak checks.
///
/// A record may not target a definition that is more restricted than the record reaching it.
pub(super) const fn visibility_rank(visibility: ShopVisibility) -> u8 {
    match visibility {
        ShopVisibility::Hidden | ShopVisibility::Unknown => 0,
        ShopVisibility::OwnerOnly => 1,
        ShopVisibility::Visible => 2,
    }
}
