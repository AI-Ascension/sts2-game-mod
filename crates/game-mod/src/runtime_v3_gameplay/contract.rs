// SPDX-License-Identifier: MIT

pub const RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION: &str = "runtime-v3-gameplay";
pub const RUNTIME_V3_GAMEPLAY_ARTIFACT: &str = "sts2-protocol/runtime-v3-gameplay";
pub const RUNTIME_V3_GAMEPLAY_SCHEMA_SOURCE: &str = "schemas/runtime-v3-gameplay.schema.json";
pub const RUNTIME_V3_GAMEPLAY_GENERATOR: &str = "hand-authored";
pub const RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST: &str =
    "fbfb18279b0c7ebb350ef0ce0d56547fa11e83985b13380cb2b0f1dba4cb56e9";
pub const RUNTIME_V3_GAMEPLAY_MAX_GENERATION: u64 = 9_007_199_254_740_991;
pub const RUNTIME_V3_GAMEPLAY_MAX_LEGAL_ACTIONS: usize = 256;
pub const RUNTIME_V3_GAMEPLAY_MAX_ENTITIES: usize = 256;
pub const RUNTIME_V3_GAMEPLAY_MAX_TEXT_BYTES: usize = 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeV3GameplayStateKind {
    Setup,
    Map,
    Combat,
    Reward,
    Shop,
    Event,
    Rest,
    Selection,
    Victory,
    Defeat,
    Recovery,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeV3GameplayEnemyIntent {
    Attack { damage: u16, hits: u8 },
    Defend,
    Buff,
    Debuff,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayCard {
    pub card_id: String,
    pub name: String,
    pub cost: u8,
    pub upgraded: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayEnemy {
    pub enemy_id: String,
    pub name: String,
    pub hp: u16,
    pub max_hp: u16,
    pub intent: RuntimeV3GameplayEnemyIntent,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayPlayer {
    pub hp: u16,
    pub max_hp: u16,
    pub energy: u8,
    pub gold: u32,
    pub hand: Vec<RuntimeV3GameplayCard>,
    pub deck: Vec<RuntimeV3GameplayCard>,
    pub discard: Vec<RuntimeV3GameplayCard>,
    pub exhaust: Vec<RuntimeV3GameplayCard>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum RuntimeV3GameplayState {
    Setup {
        characters: Vec<String>,
    },
    Map {
        node_id: Option<String>,
        options: Vec<String>,
    },
    Combat {
        turn_index: u16,
        enemies: Vec<RuntimeV3GameplayEnemy>,
    },
    Reward {
        options: Vec<String>,
    },
    Shop {
        items: Vec<RuntimeV3GameplayShopItem>,
    },
    Event {
        choices: Vec<String>,
    },
    Rest {
        options: Vec<String>,
    },
    Selection {
        choices: Vec<String>,
    },
    Victory,
    Defeat {
        reason: Option<String>,
    },
    Recovery {
        code: String,
    },
}

impl RuntimeV3GameplayState {
    #[must_use]
    pub const fn kind(&self) -> RuntimeV3GameplayStateKind {
        match self {
            Self::Setup { .. } => RuntimeV3GameplayStateKind::Setup,
            Self::Map { .. } => RuntimeV3GameplayStateKind::Map,
            Self::Combat { .. } => RuntimeV3GameplayStateKind::Combat,
            Self::Reward { .. } => RuntimeV3GameplayStateKind::Reward,
            Self::Shop { .. } => RuntimeV3GameplayStateKind::Shop,
            Self::Event { .. } => RuntimeV3GameplayStateKind::Event,
            Self::Rest { .. } => RuntimeV3GameplayStateKind::Rest,
            Self::Selection { .. } => RuntimeV3GameplayStateKind::Selection,
            Self::Victory => RuntimeV3GameplayStateKind::Victory,
            Self::Defeat { .. } => RuntimeV3GameplayStateKind::Defeat,
            Self::Recovery { .. } => RuntimeV3GameplayStateKind::Recovery,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayShopItem {
    pub item_id: String,
    pub name: String,
    pub price: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayObservation {
    pub state_id: String,
    pub generation: u64,
    pub visible_seed: Option<String>,
    pub player: RuntimeV3GameplayPlayer,
    pub state: RuntimeV3GameplayState,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeV3GameplayAction {
    StartRun {
        character_id: String,
    },
    SelectMapNode {
        node_id: String,
    },
    PlayCard {
        card_id: String,
        target_id: Option<String>,
    },
    EndTurn,
    ChooseReward {
        reward_id: String,
    },
    SkipReward,
    ShopPurchase {
        item_id: String,
    },
    ShopRemove {
        card_id: String,
    },
    Rest,
    Smith {
        card_id: String,
    },
    EventChoice {
        choice_id: String,
    },
    SelectCard {
        card_id: String,
    },
    ConfirmVictory,
    SaveQuit,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayLegalAction {
    pub action_id: String,
    pub action: RuntimeV3GameplayAction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeV3GameplayRecoveryKind {
    Reobserve,
    Reconcile,
    ReleaseLease,
    StopEpisode,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayRecovery {
    pub kind: RuntimeV3GameplayRecoveryKind,
    pub operation_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeV3GameplayWaitOutcome {
    Successor,
    SameStateMutation,
    Timeout,
    RecoveryRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeV3GameplayStatus {
    Accepted,
    Settled,
    Rejected,
    Unknown,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeV3GameplayMessageKind {
    StateRequest,
    StateResponse,
    LegalActionsRequest,
    LegalActionsResponse,
    DispatchActionRequest,
    DispatchActionResponse,
    WaitRequest,
    WaitResponse,
    ReobserveRequest,
    ReobserveResponse,
    RecoverRequest,
    RecoverResponse,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayProvenance {
    pub artifact: String,
    pub source: String,
    pub generator: String,
}

impl Default for RuntimeV3GameplayProvenance {
    fn default() -> Self {
        Self {
            artifact: RUNTIME_V3_GAMEPLAY_ARTIFACT.to_owned(),
            source: RUNTIME_V3_GAMEPLAY_SCHEMA_SOURCE.to_owned(),
            generator: RUNTIME_V3_GAMEPLAY_GENERATOR.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeV3GameplayContext {
    pub correlation_id: String,
    pub instance_id: String,
    pub session_id: String,
    pub lease_id: String,
    pub lease_epoch: u64,
}

impl RuntimeV3GameplayContext {
    #[must_use]
    pub fn new(
        correlation_id: impl Into<String>,
        instance_id: impl Into<String>,
        session_id: impl Into<String>,
        lease_id: impl Into<String>,
        lease_epoch: u64,
    ) -> Self {
        Self {
            correlation_id: correlation_id.into(),
            instance_id: instance_id.into(),
            session_id: session_id.into(),
            lease_id: lease_id.into(),
            lease_epoch,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeV3GameplayIdentity {
    pub instance_id: String,
    pub session_id: String,
    pub lease_id: String,
    pub lease_epoch: u64,
}

impl RuntimeV3GameplayIdentity {
    #[must_use]
    pub fn new(
        instance_id: impl Into<String>,
        session_id: impl Into<String>,
        lease_id: impl Into<String>,
        lease_epoch: u64,
    ) -> Self {
        Self {
            instance_id: instance_id.into(),
            session_id: session_id.into(),
            lease_id: lease_id.into(),
            lease_epoch,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayTransitionWitness {
    pub from_generation: u64,
    pub to_generation: u64,
    pub state_id: String,
    pub effect_kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayMessage {
    pub protocol_version: String,
    pub schema_digest: String,
    pub provenance: RuntimeV3GameplayProvenance,
    pub correlation_id: String,
    pub instance_id: String,
    pub session_id: String,
    pub lease_id: String,
    pub lease_epoch: u64,
    pub generation: u64,
    pub kind: RuntimeV3GameplayMessageKind,
    pub state_id: Option<String>,
    pub operation_id: Option<String>,
    pub observation: Option<RuntimeV3GameplayObservation>,
    pub legal_actions: Option<Vec<RuntimeV3GameplayLegalAction>>,
    pub action: Option<RuntimeV3GameplayLegalAction>,
    pub status: Option<RuntimeV3GameplayStatus>,
    pub transition: Option<RuntimeV3GameplayTransitionWitness>,
    pub error_code: Option<String>,
    pub wait_for_millis: Option<u32>,
    pub wait_outcome: Option<RuntimeV3GameplayWaitOutcome>,
    pub recovery: Option<RuntimeV3GameplayRecovery>,
}

impl RuntimeV3GameplayMessage {
    #[must_use]
    pub fn base(
        context: RuntimeV3GameplayContext,
        generation: u64,
        kind: RuntimeV3GameplayMessageKind,
    ) -> Self {
        Self {
            protocol_version: RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION.to_owned(),
            schema_digest: RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST.to_owned(),
            provenance: RuntimeV3GameplayProvenance::default(),
            correlation_id: context.correlation_id,
            instance_id: context.instance_id,
            session_id: context.session_id,
            lease_id: context.lease_id,
            lease_epoch: context.lease_epoch,
            generation,
            kind,
            state_id: None,
            operation_id: None,
            observation: None,
            legal_actions: None,
            action: None,
            status: None,
            transition: None,
            error_code: None,
            wait_for_millis: None,
            wait_outcome: None,
            recovery: None,
        }
    }

    #[must_use]
    pub fn state_request(context: RuntimeV3GameplayContext, generation: u64) -> Self {
        Self::base(
            context,
            generation,
            RuntimeV3GameplayMessageKind::StateRequest,
        )
    }

    #[must_use]
    pub fn reobserve_request(context: RuntimeV3GameplayContext, generation: u64) -> Self {
        Self::base(
            context,
            generation,
            RuntimeV3GameplayMessageKind::ReobserveRequest,
        )
    }

    #[must_use]
    pub fn legal_actions_request(
        context: RuntimeV3GameplayContext,
        generation: u64,
        state_id: impl Into<String>,
    ) -> Self {
        Self {
            state_id: Some(state_id.into()),
            ..Self::base(
                context,
                generation,
                RuntimeV3GameplayMessageKind::LegalActionsRequest,
            )
        }
    }

    #[must_use]
    pub fn dispatch_action_request(
        context: RuntimeV3GameplayContext,
        generation: u64,
        state_id: impl Into<String>,
        operation_id: impl Into<String>,
        action: RuntimeV3GameplayLegalAction,
    ) -> Self {
        Self {
            state_id: Some(state_id.into()),
            operation_id: Some(operation_id.into()),
            action: Some(action),
            ..Self::base(
                context,
                generation,
                RuntimeV3GameplayMessageKind::DispatchActionRequest,
            )
        }
    }

    #[must_use]
    pub fn wait_request(
        context: RuntimeV3GameplayContext,
        generation: u64,
        operation_id: impl Into<String>,
        wait_for_millis: u32,
    ) -> Self {
        Self {
            operation_id: Some(operation_id.into()),
            wait_for_millis: Some(wait_for_millis),
            ..Self::base(
                context,
                generation,
                RuntimeV3GameplayMessageKind::WaitRequest,
            )
        }
    }

    #[must_use]
    pub fn recover_request(
        context: RuntimeV3GameplayContext,
        generation: u64,
        recovery: RuntimeV3GameplayRecovery,
    ) -> Self {
        Self {
            recovery: Some(recovery),
            ..Self::base(
                context,
                generation,
                RuntimeV3GameplayMessageKind::RecoverRequest,
            )
        }
    }

    pub fn validate(&self) -> Result<(), RuntimeV3GameplayValidationError> {
        if self.protocol_version != RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION
            || self.schema_digest != RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST
            || self.provenance != RuntimeV3GameplayProvenance::default()
            || self.lease_epoch > RUNTIME_V3_GAMEPLAY_MAX_GENERATION
            || self.generation > RUNTIME_V3_GAMEPLAY_MAX_GENERATION
            || !valid_identity(&self.correlation_id)
            || !valid_identity(&self.instance_id)
            || !valid_identity(&self.session_id)
            || !valid_identity(&self.lease_id)
        {
            return Err(RuntimeV3GameplayValidationError::Metadata);
        }
        for value in [&self.state_id, &self.operation_id, &self.error_code]
            .into_iter()
            .flatten()
        {
            if !valid_identity(value) {
                return Err(RuntimeV3GameplayValidationError::InvalidIdentity);
            }
        }
        if let Some(observation) = &self.observation {
            observation.validate()?;
        }
        if let Some(actions) = &self.legal_actions {
            validate_actions(actions)?;
        }
        if let Some(action) = &self.action {
            action.validate()?;
        }
        if let Some(transition) = &self.transition {
            transition.validate()?;
        }
        if let Some(recovery) = &self.recovery {
            recovery.validate()?;
        }
        validate_shape(self)
    }

    pub fn validate_request(&self) -> Result<(), RuntimeV3GameplayValidationError> {
        self.validate()?;
        if matches!(
            self.kind,
            RuntimeV3GameplayMessageKind::StateRequest
                | RuntimeV3GameplayMessageKind::LegalActionsRequest
                | RuntimeV3GameplayMessageKind::DispatchActionRequest
                | RuntimeV3GameplayMessageKind::WaitRequest
                | RuntimeV3GameplayMessageKind::ReobserveRequest
                | RuntimeV3GameplayMessageKind::RecoverRequest
        ) {
            Ok(())
        } else {
            Err(RuntimeV3GameplayValidationError::ResultShape)
        }
    }
}

impl RuntimeV3GameplayObservation {
    pub fn validate(&self) -> Result<(), RuntimeV3GameplayValidationError> {
        if !valid_identity(&self.state_id)
            || self.generation > RUNTIME_V3_GAMEPLAY_MAX_GENERATION
            || self.player.hp > self.player.max_hp
            || self
                .visible_seed
                .as_deref()
                .is_some_and(|value| !valid_text(value))
        {
            return Err(RuntimeV3GameplayValidationError::ObservationShape);
        }
        validate_cards(&self.player.hand)?;
        validate_cards(&self.player.deck)?;
        validate_cards(&self.player.discard)?;
        validate_cards(&self.player.exhaust)?;
        match &self.state {
            RuntimeV3GameplayState::Setup { characters }
            | RuntimeV3GameplayState::Reward {
                options: characters,
            }
            | RuntimeV3GameplayState::Rest {
                options: characters,
            }
            | RuntimeV3GameplayState::Event {
                choices: characters,
            }
            | RuntimeV3GameplayState::Selection {
                choices: characters,
            } => validate_ids(characters),
            RuntimeV3GameplayState::Map { node_id, options } => {
                if node_id
                    .as_deref()
                    .is_some_and(|value| !valid_identity(value))
                {
                    return Err(RuntimeV3GameplayValidationError::InvalidIdentity);
                }
                validate_ids(options)
            }
            RuntimeV3GameplayState::Combat { enemies, .. } => {
                if enemies.len() > RUNTIME_V3_GAMEPLAY_MAX_ENTITIES {
                    return Err(RuntimeV3GameplayValidationError::CollectionBounds);
                }
                for enemy in enemies {
                    if !valid_identity(&enemy.enemy_id)
                        || !valid_text(&enemy.name)
                        || enemy.hp > enemy.max_hp
                    {
                        return Err(RuntimeV3GameplayValidationError::ObservationShape);
                    }
                    if let RuntimeV3GameplayEnemyIntent::Attack { hits, .. } = &enemy.intent {
                        if *hits == 0 {
                            return Err(RuntimeV3GameplayValidationError::ObservationShape);
                        }
                    }
                }
                Ok(())
            }
            RuntimeV3GameplayState::Shop { items } => {
                if items.len() > RUNTIME_V3_GAMEPLAY_MAX_ENTITIES {
                    return Err(RuntimeV3GameplayValidationError::CollectionBounds);
                }
                for item in items {
                    if !valid_identity(&item.item_id) || !valid_text(&item.name) {
                        return Err(RuntimeV3GameplayValidationError::ObservationShape);
                    }
                }
                Ok(())
            }
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
}

impl RuntimeV3GameplayLegalAction {
    pub fn validate(&self) -> Result<(), RuntimeV3GameplayValidationError> {
        if !valid_identity(&self.action_id) {
            return Err(RuntimeV3GameplayValidationError::InvalidIdentity);
        }
        let values = match &self.action {
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
        if values
            .into_iter()
            .flatten()
            .any(|value| !valid_identity(value))
        {
            Err(RuntimeV3GameplayValidationError::ActionShape)
        } else {
            Ok(())
        }
    }
}

impl RuntimeV3GameplayTransitionWitness {
    fn validate(&self) -> Result<(), RuntimeV3GameplayValidationError> {
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
    fn validate(&self) -> Result<(), RuntimeV3GameplayValidationError> {
        if (self.kind == RuntimeV3GameplayRecoveryKind::Reconcile) != self.operation_id.is_some() {
            return Err(RuntimeV3GameplayValidationError::RecoveryShape);
        }
        if self
            .operation_id
            .as_deref()
            .is_some_and(|value| !valid_identity(value))
        {
            return Err(RuntimeV3GameplayValidationError::InvalidIdentity);
        }
        Ok(())
    }
}

fn validate_shape(
    message: &RuntimeV3GameplayMessage,
) -> Result<(), RuntimeV3GameplayValidationError> {
    match message.kind {
        RuntimeV3GameplayMessageKind::StateRequest
        | RuntimeV3GameplayMessageKind::ReobserveRequest => {
            if payload_is_empty(message) {
                Ok(())
            } else {
                Err(RuntimeV3GameplayValidationError::ResultShape)
            }
        }
        RuntimeV3GameplayMessageKind::StateResponse
        | RuntimeV3GameplayMessageKind::ReobserveResponse => {
            if message.state_id.is_some()
                && message.observation.is_some()
                && message.legal_actions.is_some()
                && no_state_result_fields(message)
                && observation_matches_envelope(message)
            {
                Ok(())
            } else {
                Err(RuntimeV3GameplayValidationError::ResultShape)
            }
        }
        RuntimeV3GameplayMessageKind::LegalActionsRequest => {
            if message.state_id.is_some() && only_state_id_fields(message) {
                Ok(())
            } else {
                Err(RuntimeV3GameplayValidationError::ResultShape)
            }
        }
        RuntimeV3GameplayMessageKind::LegalActionsResponse => {
            if message.state_id.is_some()
                && message.legal_actions.is_some()
                && no_action_result_fields(message)
            {
                Ok(())
            } else {
                Err(RuntimeV3GameplayValidationError::ResultShape)
            }
        }
        RuntimeV3GameplayMessageKind::DispatchActionRequest => {
            if message.state_id.is_some()
                && message.operation_id.is_some()
                && message.action.is_some()
                && no_observation_result_fields(message)
            {
                Ok(())
            } else {
                Err(RuntimeV3GameplayValidationError::ResultShape)
            }
        }
        RuntimeV3GameplayMessageKind::DispatchActionResponse => non_wait_result_shape(message),
        RuntimeV3GameplayMessageKind::WaitRequest => {
            if message.operation_id.is_some()
                && message
                    .wait_for_millis
                    .is_some_and(|value| (1..=120_000).contains(&value))
                && message.state_id.is_none()
                && no_wait_request_fields(message)
            {
                Ok(())
            } else {
                Err(RuntimeV3GameplayValidationError::ResultShape)
            }
        }
        RuntimeV3GameplayMessageKind::WaitResponse => {
            if message.wait_outcome.is_some() {
                result_shape(message).and_then(|()| validate_wait_outcome(message))
            } else {
                Err(RuntimeV3GameplayValidationError::ResultShape)
            }
        }
        RuntimeV3GameplayMessageKind::RecoverRequest => {
            if message.recovery.is_some()
                && message.state_id.is_none()
                && message.operation_id.is_none()
                && no_recovery_request_fields(message)
            {
                Ok(())
            } else {
                Err(RuntimeV3GameplayValidationError::ResultShape)
            }
        }
        RuntimeV3GameplayMessageKind::RecoverResponse => non_wait_result_shape(message),
    }
}

fn payload_is_empty(message: &RuntimeV3GameplayMessage) -> bool {
    message.state_id.is_none()
        && message.operation_id.is_none()
        && message.observation.is_none()
        && message.legal_actions.is_none()
        && message.action.is_none()
        && message.status.is_none()
        && message.transition.is_none()
        && message.error_code.is_none()
        && message.wait_for_millis.is_none()
        && message.wait_outcome.is_none()
        && message.recovery.is_none()
}

fn no_observation_result_fields(message: &RuntimeV3GameplayMessage) -> bool {
    message.observation.is_none()
        && message.legal_actions.is_none()
        && message.status.is_none()
        && message.transition.is_none()
        && message.error_code.is_none()
        && message.wait_for_millis.is_none()
        && message.wait_outcome.is_none()
        && message.recovery.is_none()
}

fn no_action_result_fields(message: &RuntimeV3GameplayMessage) -> bool {
    message.operation_id.is_none()
        && message.observation.is_none()
        && message.action.is_none()
        && message.status.is_none()
        && message.transition.is_none()
        && message.error_code.is_none()
        && message.wait_for_millis.is_none()
        && message.wait_outcome.is_none()
        && message.recovery.is_none()
}

fn no_state_result_fields(message: &RuntimeV3GameplayMessage) -> bool {
    message.operation_id.is_none()
        && message.action.is_none()
        && message.status.is_none()
        && message.transition.is_none()
        && message.error_code.is_none()
        && message.wait_for_millis.is_none()
        && message.wait_outcome.is_none()
        && message.recovery.is_none()
}

fn no_recovery_request_fields(message: &RuntimeV3GameplayMessage) -> bool {
    message.observation.is_none()
        && message.legal_actions.is_none()
        && message.action.is_none()
        && message.status.is_none()
        && message.transition.is_none()
        && message.error_code.is_none()
        && message.wait_for_millis.is_none()
        && message.wait_outcome.is_none()
}

fn no_wait_request_fields(message: &RuntimeV3GameplayMessage) -> bool {
    message.state_id.is_none()
        && message.observation.is_none()
        && message.legal_actions.is_none()
        && message.action.is_none()
        && message.status.is_none()
        && message.transition.is_none()
        && message.error_code.is_none()
        && message.wait_outcome.is_none()
        && message.recovery.is_none()
}

fn only_state_id_fields(message: &RuntimeV3GameplayMessage) -> bool {
    message.operation_id.is_none()
        && message.observation.is_none()
        && message.legal_actions.is_none()
        && message.action.is_none()
        && message.status.is_none()
        && message.transition.is_none()
        && message.error_code.is_none()
        && message.wait_for_millis.is_none()
        && message.wait_outcome.is_none()
        && message.recovery.is_none()
}

fn observation_matches_envelope(message: &RuntimeV3GameplayMessage) -> bool {
    let Some(observation) = &message.observation else {
        return false;
    };
    message.state_id.as_deref() == Some(observation.state_id.as_str())
        && observation.generation == message.generation
}

fn result_shape(
    message: &RuntimeV3GameplayMessage,
) -> Result<(), RuntimeV3GameplayValidationError> {
    if message.operation_id.is_none()
        || message.action.is_some()
        || message.wait_for_millis.is_some()
        || message.recovery.is_some()
    {
        return Err(RuntimeV3GameplayValidationError::ResultShape);
    }
    match message.status {
        Some(RuntimeV3GameplayStatus::Settled) => {
            if message.observation.is_some()
                && message.legal_actions.is_some()
                && message.transition.is_some()
                && message.error_code.is_none()
                && observation_matches_envelope(message)
                && transition_matches_envelope(message)
            {
                Ok(())
            } else {
                Err(RuntimeV3GameplayValidationError::ResultShape)
            }
        }
        Some(RuntimeV3GameplayStatus::Accepted) => {
            if message.observation.is_some()
                && message.legal_actions.is_some()
                && message.transition.is_none()
                && message.error_code.is_none()
                && observation_matches_envelope(message)
            {
                Ok(())
            } else {
                Err(RuntimeV3GameplayValidationError::ResultShape)
            }
        }
        Some(RuntimeV3GameplayStatus::Rejected | RuntimeV3GameplayStatus::Cancelled) => {
            if message.observation.is_some()
                && message.legal_actions.is_some()
                && message.transition.is_none()
                && message.error_code.is_some()
                && observation_matches_envelope(message)
            {
                Ok(())
            } else {
                Err(RuntimeV3GameplayValidationError::ResultShape)
            }
        }
        Some(RuntimeV3GameplayStatus::Unknown) => {
            if message.observation.is_none()
                && message.legal_actions.is_none()
                && message.transition.is_none()
                && message.error_code.is_some()
            {
                Ok(())
            } else {
                Err(RuntimeV3GameplayValidationError::ResultShape)
            }
        }
        None => Err(RuntimeV3GameplayValidationError::ResultShape),
    }
}

fn non_wait_result_shape(
    message: &RuntimeV3GameplayMessage,
) -> Result<(), RuntimeV3GameplayValidationError> {
    result_shape(message)?;
    if message.wait_outcome.is_none() {
        Ok(())
    } else {
        Err(RuntimeV3GameplayValidationError::ResultShape)
    }
}

fn transition_matches_envelope(message: &RuntimeV3GameplayMessage) -> bool {
    let Some(transition) = &message.transition else {
        return false;
    };
    transition.to_generation == message.generation
        && message.state_id.as_deref() == Some(transition.state_id.as_str())
}

fn validate_wait_outcome(
    message: &RuntimeV3GameplayMessage,
) -> Result<(), RuntimeV3GameplayValidationError> {
    match message.wait_outcome {
        Some(RuntimeV3GameplayWaitOutcome::Successor)
        | Some(RuntimeV3GameplayWaitOutcome::SameStateMutation) => {
            if message.status == Some(RuntimeV3GameplayStatus::Settled) {
                Ok(())
            } else {
                Err(RuntimeV3GameplayValidationError::ResultShape)
            }
        }
        Some(RuntimeV3GameplayWaitOutcome::Timeout)
        | Some(RuntimeV3GameplayWaitOutcome::RecoveryRequired) => {
            if message.status == Some(RuntimeV3GameplayStatus::Unknown) {
                Ok(())
            } else {
                Err(RuntimeV3GameplayValidationError::ResultShape)
            }
        }
        None => Err(RuntimeV3GameplayValidationError::ResultShape),
    }
}

fn validate_cards(cards: &[RuntimeV3GameplayCard]) -> Result<(), RuntimeV3GameplayValidationError> {
    if cards.len() > RUNTIME_V3_GAMEPLAY_MAX_ENTITIES {
        return Err(RuntimeV3GameplayValidationError::CollectionBounds);
    }
    for card in cards {
        if !valid_identity(&card.card_id) || !valid_text(&card.name) {
            return Err(RuntimeV3GameplayValidationError::ObservationShape);
        }
    }
    Ok(())
}

fn validate_ids(values: &[String]) -> Result<(), RuntimeV3GameplayValidationError> {
    if values.len() > RUNTIME_V3_GAMEPLAY_MAX_ENTITIES {
        return Err(RuntimeV3GameplayValidationError::CollectionBounds);
    }
    if values.iter().any(|value| !valid_identity(value)) {
        Err(RuntimeV3GameplayValidationError::InvalidIdentity)
    } else {
        Ok(())
    }
}

fn validate_actions(
    actions: &[RuntimeV3GameplayLegalAction],
) -> Result<(), RuntimeV3GameplayValidationError> {
    if actions.len() > RUNTIME_V3_GAMEPLAY_MAX_LEGAL_ACTIONS {
        return Err(RuntimeV3GameplayValidationError::CollectionBounds);
    }
    for (index, action) in actions.iter().enumerate() {
        action.validate()?;
        if actions[..index]
            .iter()
            .any(|previous| previous.action_id == action.action_id)
        {
            return Err(RuntimeV3GameplayValidationError::DuplicateAction);
        }
    }
    Ok(())
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= RUNTIME_V3_GAMEPLAY_MAX_TEXT_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
}

fn valid_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= RUNTIME_V3_GAMEPLAY_MAX_TEXT_BYTES
        && !value.chars().any(char::is_control)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeV3GameplayValidationError {
    Metadata,
    InvalidIdentity,
    InvalidText,
    GenerationBounds,
    CollectionBounds,
    ObservationShape,
    ActionShape,
    DuplicateAction,
    TransitionShape,
    RecoveryShape,
    ResultShape,
}

impl std::fmt::Display for RuntimeV3GameplayValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Metadata => "runtime-v3-gameplay metadata is unsupported",
            Self::InvalidIdentity => "runtime-v3-gameplay identity is invalid",
            Self::InvalidText => "runtime-v3-gameplay visible text is invalid",
            Self::GenerationBounds => "runtime-v3-gameplay generation is out of bounds",
            Self::CollectionBounds => "runtime-v3-gameplay collection exceeds its bound",
            Self::ObservationShape => "runtime-v3-gameplay observation is malformed",
            Self::ActionShape => "runtime-v3-gameplay action is malformed",
            Self::DuplicateAction => "runtime-v3-gameplay action IDs are duplicated",
            Self::TransitionShape => "runtime-v3-gameplay transition witness is malformed",
            Self::RecoveryShape => "runtime-v3-gameplay recovery is malformed",
            Self::ResultShape => "runtime-v3-gameplay message shape is malformed",
        })
    }
}

impl std::error::Error for RuntimeV3GameplayValidationError {}
