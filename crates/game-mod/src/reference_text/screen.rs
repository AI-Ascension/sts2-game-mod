// SPDX-License-Identifier: MIT

use super::model::{ReferenceTextSegment, ReferenceUnavailableReason};

/// Semantic kind of one supported public screen.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PublicScreenKind {
    /// A modal dialog.
    Modal,
    /// A dismissible tutorial overlay.
    TutorialOverlay,
    /// A message that blocks progress until it is acknowledged.
    BlockingMessage,
    /// A confirmation prompt.
    Confirmation,
    /// A screen the owner reported that this producer does not classify.
    Unknown,
}

impl PublicScreenKind {
    /// Number of distinct screen kinds, including the unclassified one.
    pub const COUNT: usize = 5;

    /// Returns the canonical token for this screen kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Modal => "modal",
            Self::TutorialOverlay => "tutorial_overlay",
            Self::BlockingMessage => "blocking_message",
            Self::Confirmation => "confirmation",
            Self::Unknown => "unknown",
        }
    }

    /// Parses an owner-registry screen-kind token.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "modal" => Some(Self::Modal),
            "tutorial_overlay" => Some(Self::TutorialOverlay),
            "blocking_message" => Some(Self::BlockingMessage),
            "confirmation" => Some(Self::Confirmation),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    /// Returns whether this producer classifies the screen kind.
    #[must_use]
    pub const fn is_classified(self) -> bool {
        !matches!(self, Self::Unknown)
    }
}

/// Semantic kind of one described control on a supported public screen.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PublicControlKind {
    /// A button.
    Button,
    /// A choice among listed candidates.
    Choice,
    /// A free-text input whose value is always withheld.
    TextInput,
    /// A control kind the owner reported that this producer does not classify.
    Unknown,
}

impl PublicControlKind {
    /// Returns the canonical token for this control kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Choice => "choice",
            Self::TextInput => "text_input",
            Self::Unknown => "unknown",
        }
    }

    /// Parses an owner-registry control-kind token.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "button" => Some(Self::Button),
            "choice" => Some(Self::Choice),
            "text_input" => Some(Self::TextInput),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    /// Returns whether this control carries a value that must never be read.
    #[must_use]
    pub const fn carries_private_input(self) -> bool {
        matches!(self, Self::TextInput)
    }
}

/// One described control on a supported public screen.
///
/// Only semantic description crosses this boundary: an ID, a kind, a label, availability and an
/// unavailable reason.  A raw scene node, an unrelated OS window and an arbitrary host object
/// property have no representation here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicControlInput {
    /// Stable owner-defined control ID.
    pub control_id: String,
    /// Semantic control kind.
    pub kind: PublicControlKind,
    /// Label the owner renders for this control.
    pub label: String,
    /// Whether the owner reports the control as available.
    pub available: bool,
    /// Why the control is unavailable, when the owner reported it.
    pub unavailable_reason: Option<ReferenceUnavailableReason>,
}

/// One source-owned supported public screen.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicScreenInput {
    /// Semantic screen kind.
    pub screen_kind: PublicScreenKind,
    /// Stable owner-defined screen ID.
    pub screen_id: String,
    /// Locale this screen's text is authored in.
    pub locale: String,
    /// Whether the screen blocks progress until acknowledged.
    pub blocking: bool,
    /// Whether the screen can be dismissed.
    pub dismissible: bool,
    /// Ordered visible text.
    pub visible_text: Vec<ReferenceTextSegment>,
    /// Described controls.
    pub controls: Vec<PublicControlInput>,
}
