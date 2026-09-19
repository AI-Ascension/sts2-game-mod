// SPDX-License-Identifier: MIT

//! Source-only settings reference catalog.
//!
//! This module copies allowlisted game, profile, and addon setting identities, localized labels and
//! descriptions, declared value shapes, shipped defaults, stored preferences, runtime-effective
//! values, declared bounds and options, and the owner read seam each value came through into an
//! immutable catalog fenced by the existing content manifest, locale, and selected profile.  Stored
//! preferences stay distinct from effective runtime values, and run-affecting settings link to the
//! separate effective run configuration instead of restating it.
//!
//! The slice is read-only by construction: it exposes no setter, cannot address a config-file path
//! or reflect over a generic preference store, and returns every value as an explicit availability
//! state rather than a substituted default.  Credentials, private endpoints, and operator secrets
//! are excluded from public discovery and their values are withheld even in the owner scope.  No
//! live profile change, locale change, resolution change, key-binding change, or native host
//! compatibility is claimed here.

mod catalog;
mod catalog_reader;
mod definition;
mod error;
mod model;
mod validation;

pub use catalog::{SettingsCatalogProducer, SettingsCatalogSnapshot, SettingsCatalogSource};
pub use catalog_reader::{
    SettingsCatalog, SettingsCatalogReader, SettingsContinuation, SettingsDefinitionPage,
    SettingsDefinitionSummary, SettingsListQuery,
};
pub use definition::{SettingDefinition, SettingDefinitionInput};
pub use error::{SettingsReferenceError, SettingsSourceError};
pub use model::*;
