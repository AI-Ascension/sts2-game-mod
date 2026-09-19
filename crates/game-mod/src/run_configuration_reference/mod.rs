// SPDX-License-Identifier: MIT

//! Source-only run-configuration reference catalog.
//!
//! This module copies every exact-build field that affects a run into an immutable catalog fenced
//! by the existing content-manifest cursor, locale, and selected profile, and binds each definition
//! to a live witness (instance, run identity, epoch, and a monotonic configuration revision).
//! Requested setup stays distinct from the value the host settled, and settled values carry their
//! provenance and mutability so a mutable value never reads as fixed-at-start.
//!
//! Mode-specific optional fields, profile-specific rules, and modifiers the host cannot classify
//! are represented explicitly instead of being dropped: a field with no meaning for the admitted
//! mode reports `NotApplicable`, a kind with no supported extractor reports `Unsupported`, and an
//! unknown future modifier is retained with an explicit `Unknown` state. Seed material is exposed
//! only under the declared seed policy, and a seed-blind scope both refuses the seed field and
//! declines to reuse a seed-aware entry.
//!
//! Ordinary configuration reads never carry RNG state: identities in the `rng_state` namespace are
//! rejected outright, so a caller must use the dedicated RNG surfaces instead. The slice is
//! read-only by construction: it exposes no setter, cannot admit or restart a run, and reuses the
//! existing seeded-run profile and admission metadata rather than adding another run-start API. No
//! native extraction, live host change, or transport route is claimed here.

mod binding;
mod catalog;
mod definition;
mod error;
mod field;
mod identity;
mod model;
mod page;
mod projection;
mod reader;
mod source;
mod validation;
mod value;

pub use binding::{
    RUN_CONFIGURATION_FINGERPRINT_DOMAIN, RunConfigurationBinding, RunConfigurationCacheKey,
    RunConfigurationLiveBinding, RunProfile, RunProfileKind,
};
pub use catalog::{
    RunConfigurationCatalogProducer, RunConfigurationCatalogSnapshot, RunConfigurationCatalogSource,
};
pub use definition::{
    RunConfigurationCatalog, RunConfigurationCompleteness, RunConfigurationDefinition,
    RunConfigurationDefinitionReference,
};
pub use error::{RunConfigurationError, RunConfigurationSourceError};
pub use field::{
    RunConfigurationField, RunConfigurationFieldStatus, RunConfigurationUnavailableReason, RunText,
};
pub use model::*;
pub use page::{
    RunConfigurationContinuation, RunConfigurationListQuery, RunConfigurationPage,
    RunConfigurationSummary,
};
pub use reader::RunConfigurationReader;
pub use source::{
    FailingRunConfigurationSource, FixtureRunConfigurationFailure, FixtureRunConfigurationSource,
};
pub use value::RunValue;
