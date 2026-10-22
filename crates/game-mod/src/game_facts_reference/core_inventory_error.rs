// SPDX-License-Identifier: MIT

use super::GameFactsError;

/// Local signature-admission failures, without source tokens or host details.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoreFactsAdmissionError {
    /// The ordinary bounded inventory or manifest checks refused the supplied value.
    Inventory(GameFactsError),
    /// The pinned Core inventory does not match the accepted version-two profile.
    UnsupportedVersion,
    /// A supplied rule is absent from Core's pinned inventory.
    UnknownRule { rule_index: usize },
    /// A rule omitted a declared input or supplied an additional one.
    InputCount { rule_index: usize },
    /// An input name or its position disagrees with Core's declaration.
    InputName {
        rule_index: usize,
        input_index: usize,
    },
    /// An input unit disagrees with Core's declaration; no conversion is guessed.
    InputUnit {
        rule_index: usize,
        input_index: usize,
    },
    /// An input's required/conditional status disagrees with Core's declaration.
    InputAvailability {
        rule_index: usize,
        input_index: usize,
    },
}

impl std::fmt::Display for CoreFactsAdmissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Inventory(_) => "the supplied facts inventory is invalid",
            Self::UnsupportedVersion => "the Core rules-reference version is unsupported",
            Self::UnknownRule { .. } => "the supplied rule is absent from the Core inventory",
            Self::InputCount { .. } => "the input count differs from the Core rule signature",
            Self::InputName { .. } => {
                "the input name or order differs from the Core rule signature"
            }
            Self::InputUnit { .. } => "the input unit differs from the Core rule signature",
            Self::InputAvailability { .. } => {
                "the input availability differs from the Core rule signature"
            }
        })
    }
}

impl std::error::Error for CoreFactsAdmissionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Inventory(error) => Some(error),
            _ => None,
        }
    }
}
