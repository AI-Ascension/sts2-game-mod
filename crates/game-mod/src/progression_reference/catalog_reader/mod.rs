// SPDX-License-Identifier: MIT

//! The immutable catalog and the non-clonable reader that fences every bounded read.

mod catalog;
mod page;
mod reader;

pub use catalog::ProgressionCatalog;
pub use page::{
    ProgressionContinuation, ProgressionEntryPage, ProgressionEntrySummary, ProgressionListQuery,
};
pub use reader::ProgressionCatalogReader;
