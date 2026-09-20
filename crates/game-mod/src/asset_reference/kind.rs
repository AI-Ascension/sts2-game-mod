// SPDX-License-Identifier: MIT

//! Media kinds, media classes, retrieval states and visibility.

use super::AssetField;

/// Media kind an asset reference declares.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AssetMediaKind {
    /// A small content-linked icon.
    Icon,
    /// Content-linked card, character, relic or background art.
    Art,
    /// Content-linked music, ambience or a sound effect.
    Audio,
}

impl AssetMediaKind {
    /// Returns every media kind, so coverage can be stated rather than inferred.
    #[must_use]
    pub const fn all() -> [Self; 3] {
        [Self::Icon, Self::Art, Self::Audio]
    }

    /// Returns the media class this kind belongs to.
    #[must_use]
    pub const fn media_class(self) -> AssetMediaClass {
        match self {
            Self::Icon | Self::Art => AssetMediaClass::Image,
            Self::Audio => AssetMediaClass::Audio,
        }
    }
}

/// Coarse media class, so per-class support can be stated once rather than per asset.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AssetMediaClass {
    /// Raster or vector image media.
    Image,
    /// Audio media.
    Audio,
}

impl AssetMediaClass {
    /// Returns every class, so coverage can be stated rather than inferred.
    #[must_use]
    pub const fn all() -> [Self; 2] {
        [Self::Image, Self::Audio]
    }
}

/// Source support state for one media class.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AssetClassState {
    /// Source projects every asset in this class.
    Projected,
    /// The class exists but no typed source adapter is available.
    Unsupported,
    /// The class is known but currently unavailable.
    Unavailable,
}

/// Declared support state for one media class with the count the source reports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetClassCoverage {
    /// Media class this row states.
    pub class: AssetMediaClass,
    /// Source support state.
    pub state: AssetClassState,
    /// Number of assets the source declares in this class.
    pub asset_count: usize,
    /// Fields this class does not project, stated rather than omitted.
    pub unsupported_fields: Vec<AssetField>,
}

/// What the source reports about retrieving one asset's bytes.
///
/// The two non-retrievable states are deliberately separate: collapsed into one "not available"
/// value, a retrieval this boundary cannot serve is reported as an asset the installed build does
/// not contain, and a genuinely missing asset is reported as a policy refusal.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AssetRetrievalState {
    /// The asset exists and its bytes may be served within the owner-controlled bounds.
    Retrievable,
    /// The asset exists and its metadata is known, but bytes are withheld at this scope.
    MetadataOnly,
    /// The asset exists but no supported adapter can serve its bytes here.
    RetrievalUnavailable,
    /// The installed build contains no such asset, though content links it.
    AssetMissing,
}

impl AssetRetrievalState {
    /// Returns whether bytes may be published for this state.
    #[must_use]
    pub const fn is_retrievable(self) -> bool {
        matches!(self, Self::Retrievable)
    }

    /// Returns whether the installed build lacks the asset entirely.
    #[must_use]
    pub const fn is_absent_from_build(self) -> bool {
        matches!(self, Self::AssetMissing)
    }

    /// Returns whether the state names a reason rather than a bare absence.
    #[must_use]
    pub const fn is_stated(self) -> bool {
        true
    }

    /// Returns whether metadata and byte size may be published for this state.
    #[must_use]
    pub const fn has_observable_media(self) -> bool {
        !matches!(self, Self::AssetMissing)
    }
}

/// Owner-defined visibility of one asset reference.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AssetVisibility {
    /// Visible on the supported public reference surface.
    Visible,
    /// Visible only in an explicitly owner-authorized scope.
    OwnerOnly,
    /// The source knows the asset exists but must not reveal it.
    Hidden,
}

/// Scope requested by an asset query.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AssetVisibilityScope {
    /// Publicly visible assets only.
    Public,
    /// Public assets plus owner-only assets.
    Reference,
    /// Explicit owner-authorized scope, including hidden assets.
    Owner,
}

impl AssetVisibilityScope {
    /// Returns whether this scope may observe one visibility class.
    #[must_use]
    pub const fn observes(self, visibility: AssetVisibility) -> bool {
        match visibility {
            AssetVisibility::Visible => true,
            AssetVisibility::OwnerOnly => !matches!(self, Self::Public),
            AssetVisibility::Hidden => matches!(self, Self::Owner),
        }
    }
}
