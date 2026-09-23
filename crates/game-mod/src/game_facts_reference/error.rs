// SPDX-License-Identifier: MIT

//! Sanitized failures while validating or reading a facts-handoff inventory.

/// Sanitized failures while building or reading a source-only facts-handoff inventory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameFactsError {
    /// A present collection is empty, so it states nothing.
    EmptyPresentCollection(&'static str),
    /// An identity contains a control byte, a path separator or a traversal segment.
    NonOpaqueIdentity(&'static str),
    /// An input field is invalid or exceeds a local bound.
    InvalidInput(&'static str),
    /// Two entries repeat one opaque rule id.
    DuplicateRule(String),
    /// One rule repeats an input name.
    DuplicateInput {
        /// Rule the repeated input belongs to.
        rule_id: String,
        /// Repeated input name.
        name: String,
    },
    /// A combination names a rule the inventory does not declare.
    UnknownRuleReference {
        /// Combination this reference belongs to, by index.
        combination: usize,
        /// Rule id that is absent from the inventory.
        rule_id: String,
    },
    /// An inventory exceeds a local count or byte bound.
    InventoryTooLarge {
        /// Configured bound.
        limit: usize,
        /// Measured size.
        actual: usize,
    },
}

impl std::fmt::Display for GameFactsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for GameFactsError {}
