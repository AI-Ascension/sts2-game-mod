// SPDX-License-Identifier: MIT

use super::REFERENCE_TEXT_MAX_IDENTITY_BYTES;

/// Owner-defined family of non-gameplay reference text.
///
/// The families are the documented inventory: tutorial explanations, help material, lore,
/// credits, and the public UI text a screen currently displays.  A family the owner reports but
/// this producer cannot project stays [`ReferenceFamily::Unknown`] and is counted as explicit
/// unsupported scope rather than dropped from the inventory.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReferenceFamily {
    /// Step-by-step tutorial explanations.
    Tutorial,
    /// Help and rules reference material.
    Help,
    /// Lore and background text.
    Lore,
    /// Credits and attribution text.
    Credits,
    /// Public UI text a screen currently displays.
    UiText,
    /// A family the owner reported that this producer does not project.
    Unknown,
}

impl ReferenceFamily {
    /// Number of distinct families, including the unsupported one.
    pub const COUNT: usize = 6;

    /// Every family in canonical order.
    pub const ALL: [Self; Self::COUNT] = [
        Self::Tutorial,
        Self::Help,
        Self::Lore,
        Self::Credits,
        Self::UiText,
        Self::Unknown,
    ];

    /// Returns the canonical token for this family.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tutorial => "tutorial",
            Self::Help => "help",
            Self::Lore => "lore",
            Self::Credits => "credits",
            Self::UiText => "ui_text",
            Self::Unknown => "unknown",
        }
    }

    /// Parses an owner-registry family token.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "tutorial" => Some(Self::Tutorial),
            "help" => Some(Self::Help),
            "lore" => Some(Self::Lore),
            "credits" => Some(Self::Credits),
            "ui_text" => Some(Self::UiText),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    /// Returns whether this family is an inventoried, projectable reference family.
    #[must_use]
    pub const fn is_inventoried(self) -> bool {
        !matches!(self, Self::Unknown)
    }
}

/// How much of one reference document this consumer may read.
///
/// The policy is carried for every document so a locked or spoiler-bearing reference is answered
/// with an explicit reason instead of being silently omitted, which would be indistinguishable
/// from a family the owner never authored.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReferenceDiscovery {
    /// The document is readable in full.
    Open,
    /// The document is readable because the run has already discovered it.
    Discovered,
    /// The document is withheld until the run discovers it.
    Locked,
    /// The owner did not report a discovery state.
    Unknown,
}

impl ReferenceDiscovery {
    /// Returns the canonical token for this policy.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Discovered => "discovered",
            Self::Locked => "locked",
            Self::Unknown => "unknown",
        }
    }

    /// Parses an owner-registry discovery token.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "open" => Some(Self::Open),
            "discovered" => Some(Self::Discovered),
            "locked" => Some(Self::Locked),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    /// Returns whether this policy permits reading the document's text.
    #[must_use]
    pub const fn is_readable(self) -> bool {
        matches!(self, Self::Open | Self::Discovered)
    }
}

/// Owner-declared reason a stored value is unavailable.
///
/// The reason is carried instead of substituting an empty string or an invented description for
/// text the owner could not supply.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReferenceUnavailableReason {
    /// The owner has authored no text for this reference.
    NotAuthored,
    /// The reference is withheld by its discovery policy.
    Locked,
    /// The reference belongs to a family this producer does not project.
    UnsupportedScope,
    /// The value is a private input field and is deliberately withheld.
    PrivateInput,
    /// The owner could not determine availability for this reference.
    Unknown,
}

impl ReferenceUnavailableReason {
    /// Returns the canonical token for this reason.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotAuthored => "not_authored",
            Self::Locked => "locked",
            Self::UnsupportedScope => "unsupported_scope",
            Self::PrivateInput => "private_input",
            Self::Unknown => "unknown",
        }
    }
}

/// Language-independent identity of one referenced definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ReferenceEntityReference {
    /// Stable owner-defined entity family.
    pub entity_kind: String,
    /// Opaque ID scoped by `entity_kind`.
    pub namespaced_id: String,
}

/// One ordered piece of stored reference text.
///
/// Formatting is retained as inert structure: an emphasis run states that the owner rendered a
/// run of text with emphasis, which is presentation metadata and never agent authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReferenceTextSegment {
    /// Literal reference text, carried verbatim.
    Text(String),
    /// A run the owner rendered with emphasis, carried verbatim.
    Emphasis(String),
    /// An explicit line break in the owner's text.
    LineBreak,
    /// A typed stable reference to another definition.
    Reference(ReferenceEntityReference),
    /// A value the owner could not supply, with the explicit reason.
    Unavailable(ReferenceUnavailableReason),
}

impl ReferenceTextSegment {
    /// Returns the aggregate retained byte length of one segment.
    #[must_use]
    pub(super) fn retained_len(&self) -> usize {
        match self {
            Self::Text(value) | Self::Emphasis(value) => value.len(),
            Self::Reference(reference) => {
                reference.entity_kind.len() + reference.namespaced_id.len()
            }
            Self::LineBreak | Self::Unavailable(_) => 0,
        }
    }
}

/// One source-owned section of a reference document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceSectionInput {
    /// Stable owner-defined section ID, unique within its document.
    pub section_id: String,
    /// Heading the owner renders for this section.
    pub heading: String,
    /// Ordered section text.
    pub segments: Vec<ReferenceTextSegment>,
    /// Searchable keywords the owner associated with this section.
    pub keywords: Vec<String>,
}

/// One source-owned reference document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceDocumentInput {
    /// Reference family this document belongs to.
    pub family: ReferenceFamily,
    /// Stable owner-defined document ID.
    pub namespaced_id: String,
    /// Locale this document's text is authored in.
    pub locale: String,
    /// How much of this document the consumer may read.
    pub discovery: ReferenceDiscovery,
    /// Revision of this document's text, independent of the semantic content revision.
    pub revision: String,
    /// Ordered document title.
    pub title: Vec<ReferenceTextSegment>,
    /// Ordered sections.
    pub sections: Vec<ReferenceSectionInput>,
    /// Searchable keywords the owner associated with the whole document.
    pub keywords: Vec<String>,
    /// References from this document to other definitions.
    pub related: Vec<ReferenceEntityReference>,
}

/// Validates one opaque identity token used anywhere in this module.
pub(super) fn validate_identity(
    value: &str,
    field: &'static str,
) -> Result<(), super::ReferenceTextError> {
    if value.is_empty()
        || value.len() > REFERENCE_TEXT_MAX_IDENTITY_BYTES
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric() && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-')
        })
    {
        return Err(super::ReferenceTextError::InvalidInput(field));
    }
    Ok(())
}
