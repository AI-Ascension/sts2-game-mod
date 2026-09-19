// SPDX-License-Identifier: MIT

use sha2::{Digest, Sha256};

use crate::ContentCursorBinding;

/// Domain separation tag for the run-configuration fingerprint.
pub const RUN_CONFIGURATION_FINGERPRINT_DOMAIN: &str = "sts2.run-configuration.v1";

/// Kind of profile the run was admitted under.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunProfileKind {
    /// The default profile.
    Default,
    /// A named profile.
    Named,
    /// The host could not classify the profile.
    Unknown,
}

/// Profile the run was admitted under.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunProfile {
    /// Exact profile identity.
    pub profile_id: String,
    /// Kind of profile.
    pub kind: RunProfileKind,
}

/// Catalog witness every static part of a definition is bound to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunConfigurationBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale for localized labels.
    pub locale: String,
    /// Profile the run was admitted under.
    pub profile: RunProfile,
    /// Exact source producer compatibility.
    pub producer_version: String,
}

/// Live witness: run identity and the monotonic configuration revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunConfigurationLiveBinding {
    /// Run identity settled at admission.
    pub run_id: String,
    /// Live instance identity for the selected run.
    pub instance_id: String,
    /// Monotonic snapshot epoch.
    pub epoch: u64,
    /// Monotonic configuration revision.
    pub revision: u64,
}

/// Key that fences preview and cache entries by configuration revision and fingerprint.
///
/// The fingerprint covers the settled mode, difficulty, character, loadout, act sequence, active
/// modifiers, multiplayer scaling, active content, and unlock rules.  It never covers an
/// unavailable seed; a key produced without seed material carries `seed_blind`, so a seed-blind
/// entry can never be reused for a seed-aware query or the reverse.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunConfigurationCacheKey {
    /// Configuration revision the entry was produced for.
    pub revision: u64,
    /// Domain-separated fingerprint of the settled configuration.
    pub fingerprint: String,
    /// Whether the entry was keyed without seed material.
    pub seed_blind: bool,
}

impl RunConfigurationCacheKey {
    /// Creates a key for one revision and fingerprint.
    #[must_use]
    pub const fn new(revision: u64, fingerprint: String, seed_blind: bool) -> Self {
        Self {
            revision,
            fingerprint,
            seed_blind,
        }
    }

    /// Returns whether two runs may share one preview or cache entry.
    #[must_use]
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self == other
    }
}

/// Computes the domain-separated fingerprint of the supplied configuration parts.
#[must_use]
pub(super) fn configuration_fingerprint(parts: &[String]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(RUN_CONFIGURATION_FINGERPRINT_DOMAIN.as_bytes());
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part.as_bytes());
    }
    let output = hasher.finalize();
    let mut hex = String::with_capacity(output.len() * 2);
    for byte in output {
        hex.push_str(&format!("{byte:02x}"));
    }
    format!("sha256:{hex}")
}
