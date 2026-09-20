// SPDX-License-Identifier: MIT

//! The immutable catalog and the non-clonable reader that fences every bounded read.

mod catalog;
mod page;
mod projection;
mod reader;

pub use catalog::AssetCatalog;
pub use page::{AssetContinuation, AssetListQuery, AssetPage, AssetSummary};
pub use reader::AssetCatalogReader;
