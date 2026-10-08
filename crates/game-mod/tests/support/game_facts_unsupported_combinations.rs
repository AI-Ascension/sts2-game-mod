// SPDX-License-Identifier: MIT

use sts2_game_mod::FactsUnsupportedCombination;

pub(super) fn unsupported(rule_ids: &[&str], reason: &str) -> FactsUnsupportedCombination {
    FactsUnsupportedCombination {
        rule_ids: rule_ids.iter().map(|id| (*id).to_owned()).collect(),
        reason: reason.to_owned(),
    }
}
