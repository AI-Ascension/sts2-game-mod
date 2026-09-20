// SPDX-License-Identifier: MIT

use super::{
    field::{ActionField, ActionText},
    kind::{ActionEligibilityState, ActionRefusalReason},
    model::ActionSemanticReference,
};

/// Resolved availability of one action or one of its targets with an explicit host reason.
///
/// A refused subject must carry a reason code and the host's own reason text, and an available
/// subject must carry neither, so a disabled option is always explained and an enabled one never
/// carries a stale refusal. The reason text is the host's statement, not an invented description.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionEligibility {
    /// Resolved availability state.
    pub state: ActionEligibilityState,
    /// Explicit refusal reason code, present only for a refused subject.
    pub reason: ActionField<ActionRefusalReason>,
    /// Localized host reason text, present only for a refused subject.
    pub reason_text: ActionText,
    /// Definitions the availability depends on.
    pub references: Vec<ActionSemanticReference>,
}

impl ActionEligibility {
    /// Returns whether the subject may be acted on now.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        matches!(self.state, ActionEligibilityState::Available)
    }

    /// Returns the refusal reason code, when the subject is refused for a stated reason.
    #[must_use]
    pub fn refusal(&self) -> Option<&ActionRefusalReason> {
        match self.state {
            ActionEligibilityState::Available => None,
            ActionEligibilityState::Unavailable | ActionEligibilityState::Unknown => {
                self.reason.value()
            }
        }
    }
}
