// SPDX-License-Identifier: MIT

use super::super::ProgressionReferenceError;
use super::{
    PROGRESSION_MAX_CONTENT_REFERENCES, PROGRESSION_MAX_RELATED, PROGRESSION_MAX_REQUIREMENTS,
    ProgressionDomain, ProgressionFieldValue, ProgressionQuantity, ProgressionReadState,
    ProgressionText, validate_identity,
};

/// Kind of requirement gating one progression entry.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProgressionRequirementKind {
    /// A content definition the profile must have encountered.
    Content,
    /// A count of something the profile must reach.
    Threshold,
    /// A prior entry the profile must already hold.
    Prerequisite,
    /// An owner-defined rule identity that is not a manifest definition.
    Rule,
    /// The source could not classify the requirement.
    Unknown,
}

/// Reference to a manifest definition the progression records resolve against.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProgressionContentReference {
    /// Manifest entity kind, such as `card`.
    pub entity_kind: String,
    /// Namespaced manifest identity.
    pub namespaced_id: String,
}

impl ProgressionContentReference {
    /// Creates one bounded content reference.
    pub fn new(entity_kind: &str, namespaced_id: &str) -> Result<Self, ProgressionReferenceError> {
        validate_identity(entity_kind, "content_reference_kind")?;
        validate_identity(namespaced_id, "content_reference_id")?;
        Ok(Self {
            entity_kind: entity_kind.to_owned(),
            namespaced_id: namespaced_id.to_owned(),
        })
    }
}

/// One requirement gating an entry, with its own state and progress.
///
/// A requirement is never inferred from the entry's own state: an entry may be locked by a
/// requirement the source states, and a requirement may be satisfied while the entry is not held.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressionRequirement {
    /// Stable requirement identity.
    pub requirement_id: String,
    /// Requirement kind.
    pub kind: ProgressionRequirementKind,
    /// What the source reports about this requirement for the selected profile.
    pub state: ProgressionReadState,
    /// Progress toward the requirement, or its stated absence.
    pub progress: ProgressionFieldValue<ProgressionQuantity>,
    /// Content definition the requirement resolves against, when it has one.
    pub content: Option<ProgressionContentReference>,
    /// Localized requirement description, or its stated absence.
    pub description: ProgressionText,
}

/// One related owner identity carried beside an entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressionRelatedReference {
    /// Domain the related identity belongs to.
    pub domain: ProgressionDomain,
    /// Referenced owner identity.
    pub id: String,
}

/// Validates the bounded collection limits of one entry before it is bound.
pub(crate) fn validate_entry_bounds(
    requirement_count: usize,
    content_reference_count: usize,
    related_count: usize,
) -> Result<(), ProgressionReferenceError> {
    if requirement_count > PROGRESSION_MAX_REQUIREMENTS {
        return Err(ProgressionReferenceError::InvalidInput("requirements"));
    }
    if content_reference_count > PROGRESSION_MAX_CONTENT_REFERENCES {
        return Err(ProgressionReferenceError::InvalidInput(
            "content_references",
        ));
    }
    if related_count > PROGRESSION_MAX_RELATED {
        return Err(ProgressionReferenceError::InvalidInput("related"));
    }
    Ok(())
}
