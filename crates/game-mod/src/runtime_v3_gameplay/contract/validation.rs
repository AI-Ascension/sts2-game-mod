// SPDX-License-Identifier: MIT

use super::*;

impl RuntimeV3GameplayCard {
    fn validate(&self) -> Result<(), RuntimeV3GameplayValidationError> {
        if !valid_identity(&self.card_id) {
            return Err(RuntimeV3GameplayValidationError::InvalidIdentity);
        }
        if !valid_text(&self.name) {
            return Err(RuntimeV3GameplayValidationError::InvalidText);
        }
        Ok(())
    }
}

impl RuntimeV3GameplayEnemy {
    fn validate(&self) -> Result<(), RuntimeV3GameplayValidationError> {
        if !valid_identity(&self.enemy_id) {
            return Err(RuntimeV3GameplayValidationError::InvalidIdentity);
        }
        if !valid_text(&self.name) || self.hp > self.max_hp {
            return Err(RuntimeV3GameplayValidationError::InvalidText);
        }
        if let RuntimeV3GameplayEnemyIntent::Attack { hits, .. } = &self.intent
            && *hits == 0
        {
            return Err(RuntimeV3GameplayValidationError::ObservationShape);
        }
        Ok(())
    }
}

impl RuntimeV3GameplayPlayer {
    fn validate(&self) -> Result<(), RuntimeV3GameplayValidationError> {
        if self.hp > self.max_hp {
            return Err(RuntimeV3GameplayValidationError::ObservationShape);
        }
        for cards in [&self.hand, &self.deck, &self.discard, &self.exhaust] {
            if cards.len() > RUNTIME_V3_GAMEPLAY_MAX_ENTITIES {
                return Err(RuntimeV3GameplayValidationError::CollectionBounds);
            }
            for card in cards {
                card.validate()?;
            }
        }
        Ok(())
    }
}

impl RuntimeV3GameplayObservation {
    /// Validates only bounded, player-visible representation facts.
    pub fn validate(&self) -> Result<(), RuntimeV3GameplayValidationError> {
        if !valid_identity(&self.state_id) {
            return Err(RuntimeV3GameplayValidationError::InvalidIdentity);
        }
        if self.generation > RUNTIME_V3_GAMEPLAY_MAX_GENERATION {
            return Err(RuntimeV3GameplayValidationError::GenerationBounds);
        }
        if self
            .visible_seed
            .as_deref()
            .is_some_and(|seed| !valid_text(seed))
        {
            return Err(RuntimeV3GameplayValidationError::InvalidText);
        }
        self.player.validate()?;
        validate_state(&self.state)?;
        Ok(())
    }
}

impl RuntimeV3GameplayShopItem {
    fn validate(&self) -> Result<(), RuntimeV3GameplayValidationError> {
        if !valid_identity(&self.item_id) {
            return Err(RuntimeV3GameplayValidationError::InvalidIdentity);
        }
        if !valid_text(&self.name) {
            return Err(RuntimeV3GameplayValidationError::InvalidText);
        }
        Ok(())
    }
}

impl RuntimeV3GameplayLegalAction {
    /// Validates the bounded typed action, without claiming that it is legal in the current state.
    pub fn validate(&self) -> Result<(), RuntimeV3GameplayValidationError> {
        if !valid_identity(&self.action_id) {
            return Err(RuntimeV3GameplayValidationError::InvalidIdentity);
        }
        let identifiers = match &self.action {
            RuntimeV3GameplayAction::StartRun { character_id }
            | RuntimeV3GameplayAction::SelectMapNode {
                node_id: character_id,
            }
            | RuntimeV3GameplayAction::ChooseReward {
                reward_id: character_id,
            }
            | RuntimeV3GameplayAction::ShopPurchase {
                item_id: character_id,
            }
            | RuntimeV3GameplayAction::ShopRemove {
                card_id: character_id,
            }
            | RuntimeV3GameplayAction::Smith {
                card_id: character_id,
            }
            | RuntimeV3GameplayAction::EventChoice {
                choice_id: character_id,
            }
            | RuntimeV3GameplayAction::SelectCard {
                card_id: character_id,
            } => [Some(character_id.as_str()), None],
            RuntimeV3GameplayAction::PlayCard { card_id, target_id } => {
                [Some(card_id.as_str()), target_id.as_deref()]
            }
            RuntimeV3GameplayAction::EndTurn
            | RuntimeV3GameplayAction::SkipReward
            | RuntimeV3GameplayAction::Rest
            | RuntimeV3GameplayAction::ConfirmVictory
            | RuntimeV3GameplayAction::SaveQuit => [None, None],
        };
        if identifiers
            .into_iter()
            .flatten()
            .any(|identifier| !valid_identity(identifier))
        {
            return Err(RuntimeV3GameplayValidationError::ActionShape);
        }
        Ok(())
    }
}

impl RuntimeV3GameplayTransitionWitness {
    pub(crate) fn validate(&self) -> Result<(), RuntimeV3GameplayValidationError> {
        if self.from_generation > RUNTIME_V3_GAMEPLAY_MAX_GENERATION
            || self.to_generation > RUNTIME_V3_GAMEPLAY_MAX_GENERATION
            || self.to_generation <= self.from_generation
            || !valid_identity(&self.state_id)
            || !valid_identity(&self.effect_kind)
        {
            return Err(RuntimeV3GameplayValidationError::TransitionShape);
        }
        Ok(())
    }
}

impl RuntimeV3GameplayRecovery {
    pub(crate) fn validate(&self) -> Result<(), RuntimeV3GameplayValidationError> {
        if (self.kind == RuntimeV3GameplayRecoveryKind::Reconcile) != self.operation_id.is_some() {
            return Err(RuntimeV3GameplayValidationError::RecoveryShape);
        }
        if self
            .operation_id
            .as_deref()
            .is_some_and(|operation_id| !valid_identity(operation_id))
        {
            return Err(RuntimeV3GameplayValidationError::InvalidIdentity);
        }
        Ok(())
    }
}

fn validate_state(state: &RuntimeV3GameplayState) -> Result<(), RuntimeV3GameplayValidationError> {
    match state {
        RuntimeV3GameplayState::Setup { characters } => validate_id_list(characters),
        RuntimeV3GameplayState::Map { node_id, options } => {
            validate_optional_identity(node_id)?;
            validate_id_list(options)
        }
        RuntimeV3GameplayState::Combat { enemies, .. } => {
            if enemies.len() > RUNTIME_V3_GAMEPLAY_MAX_ENTITIES {
                return Err(RuntimeV3GameplayValidationError::CollectionBounds);
            }
            for enemy in enemies {
                enemy.validate()?;
            }
            Ok(())
        }
        RuntimeV3GameplayState::Reward { options } | RuntimeV3GameplayState::Rest { options } => {
            validate_id_list(options)
        }
        RuntimeV3GameplayState::Shop { items } => {
            if items.len() > RUNTIME_V3_GAMEPLAY_MAX_ENTITIES {
                return Err(RuntimeV3GameplayValidationError::CollectionBounds);
            }
            for item in items {
                item.validate()?;
            }
            Ok(())
        }
        RuntimeV3GameplayState::Event { choices }
        | RuntimeV3GameplayState::Selection { choices } => validate_id_list(choices),
        RuntimeV3GameplayState::Victory => Ok(()),
        RuntimeV3GameplayState::Defeat { reason } => {
            if reason.as_deref().is_some_and(|value| !valid_text(value)) {
                return Err(RuntimeV3GameplayValidationError::InvalidText);
            }
            Ok(())
        }
        RuntimeV3GameplayState::Recovery { code } => {
            if valid_identity(code) {
                Ok(())
            } else {
                Err(RuntimeV3GameplayValidationError::InvalidIdentity)
            }
        }
    }
}

fn validate_id_list(values: &[String]) -> Result<(), RuntimeV3GameplayValidationError> {
    if values.len() > RUNTIME_V3_GAMEPLAY_MAX_ENTITIES {
        return Err(RuntimeV3GameplayValidationError::CollectionBounds);
    }
    if values.iter().any(|value| !valid_identity(value)) {
        return Err(RuntimeV3GameplayValidationError::InvalidIdentity);
    }
    Ok(())
}

fn validate_optional_identity(
    value: &Option<String>,
) -> Result<(), RuntimeV3GameplayValidationError> {
    if value.as_deref().is_some_and(|value| !valid_identity(value)) {
        Err(RuntimeV3GameplayValidationError::InvalidIdentity)
    } else {
        Ok(())
    }
}
