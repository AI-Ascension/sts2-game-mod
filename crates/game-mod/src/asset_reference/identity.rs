// SPDX-License-Identifier: MIT

//! Catalog identity and the revision every asset query must name to be answered.

use crate::ContentCursorBinding;

/// Static catalog identity: manifest, locale, content revision, generation and producer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized asset value.
    pub locale: String,
    /// Content-set revision the assets were read at.
    pub content_revision: String,
    /// Catalog generation the assets were read at.
    pub generation: u64,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

impl AssetCatalogBinding {
    /// Returns the revision every query must name to be answered.
    #[must_use]
    pub fn revision(&self) -> AssetRevision {
        AssetRevision {
            content_revision: self.content_revision.clone(),
            generation: self.generation,
        }
    }
}

/// Documented freshness of one asset read.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AssetRevision {
    /// Content-set revision this read is bound to.
    pub content_revision: String,
    /// Catalog generation this read is bound to.
    pub generation: u64,
}

/// Returns whether one string is an opaque handle rather than a path or a URL.
///
/// A locator always contains a separator or a scheme, so refusing `..`, both path separators and
/// the scheme colon is enough to keep a handle from naming a file or a remote resource.
#[must_use]
pub fn is_opaque_handle(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains(':')
        && !value.contains("..")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'#'))
}
