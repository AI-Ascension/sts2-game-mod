// SPDX-License-Identifier: MIT

//! Causality as a stated fact, never as a difference between two snapshots.

/// Whether a recorded event states its causal parent.
///
/// The distinction exists so "no parent" and "not yet known" cannot be confused. A history that
/// inferred a parent by diffing snapshots would manufacture causal claims the host never made, so this
/// vocabulary carries only what was stated, and says plainly when nothing was.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SemanticCausalProvenance {
    /// The host named the parent event explicitly.
    Stated,
    /// No parent was named; this is a disclosure, not an inference.
    NotStated,
}

impl SemanticCausalProvenance {
    /// Every provenance, in a stable order.
    pub const ALL: [Self; 2] = [Self::Stated, Self::NotStated];

    /// The stable lowercase name used in owner-defined text and diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Stated => "stated",
            Self::NotStated => "not_stated",
        }
    }
}

/// The causal parent of one event, paired with whether it was stated.
///
/// The two fields must agree: a named parent is `Stated` and an unnamed parent is `NotStated`. Any
/// other pairing is a refusal, so a consumer can rely on `provenance` alone to decide whether the
/// absence of a parent means anything.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticCausalParent {
    /// The parent event identity, present only when one was stated.
    pub parent_event_id: Option<String>,
    /// Whether the parent above was stated by the host.
    pub provenance: SemanticCausalProvenance,
}

impl SemanticCausalParent {
    /// The explicit disclosure that no parent was named.
    #[must_use]
    pub const fn not_stated() -> Self {
        Self {
            parent_event_id: None,
            provenance: SemanticCausalProvenance::NotStated,
        }
    }

    /// A parent the host named explicitly.
    #[must_use]
    pub fn stated(parent_event_id: &str) -> Self {
        Self {
            parent_event_id: Some(parent_event_id.to_owned()),
            provenance: SemanticCausalProvenance::Stated,
        }
    }

    /// Returns the stated parent identity, if one was stated.
    #[must_use]
    pub fn stated_parent(&self) -> Option<&str> {
        match self.provenance {
            SemanticCausalProvenance::Stated => self.parent_event_id.as_deref(),
            SemanticCausalProvenance::NotStated => None,
        }
    }
}
