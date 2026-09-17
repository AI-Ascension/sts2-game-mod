// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.CheckpointCoverageReflection;

internal static partial class CoverageFamilies
{
    private const string PowerModel = "Models.PowerModel";
    private const string MonsterModel = "Models.MonsterModel";
    private const string MoveMachine = "MonsterMoves.MonsterMoveStateMachine.MonsterMoveStateMachine";

    private static CoverageFamily Combat() => new("combat", "0037", "Combat",
        "Combat state is not part of the single-player host run save (`SerializableRoom.EncounterState` is a string map and `NetFullCombatState` is a multiplayer carrier); a combat checkpoint must serialize the owned values listed here itself.",
        "`Allies/Enemies/Creatures`, the combat piles, `_powers`, `_orbs`, `_pets`, and `StateLog` are lists and keep host order; `_playersReadyToEndTurn` is a hash set.",
        "No host-owned combat restore path is observed; ADR 0042 stays `no_restore_adapter`.",
        "Capture is refused unless `CombatManager.IsInProgress` holds with `PlayerTurnPhase.Play`, an empty `ActionQueueSet`, `ActionExecutor.IsRunning` false, and no action in `GatheringPlayerChoice`; a `PowerModel._internalData` (`Object`) that cannot be typed rejects capture.",
        [
            P("turn/phase", CombatPlayer, "TurnNumber"), P("turn/phase", CombatPlayer, "Phase"),
            F("turn/phase", "Combat.PlayerTurnPhase", "Play"), F("turn/phase", "Combat.PlayerTurnPhase", "End"),
            P("turn/phase", CombatState, "RoundNumber"), P("turn/phase", CombatState, "CurrentSide"), P("turn/phase", CombatState, "Encounter"),
            P("turn/phase", "Rooms.CombatRoom", "CombatState"),
            P("turn/phase", CombatManager, "IsInProgress"), P("turn/phase", CombatManager, "IsStarting"), P("turn/phase", CombatManager, "IsEnding"),
            P("turn/phase", CombatManager, "IsEnemyTurnStarted"), P("turn/phase", CombatManager, "EndingPlayerTurnPhaseOne"),
            P("turn/phase", CombatManager, "EndingPlayerTurnPhaseTwo"), P("turn/phase", CombatManager, "IsPaused"),
            P("turn/phase", CombatManager, "PlayerActionsDisabled"), F("turn/phase", CombatManager, "_playersReadyToEndTurn"),
            F("turn/phase", CombatManager, "_deferredEndTurnTransition"),
            P("energy", CombatPlayer, "Energy"), P("energy", CombatPlayer, "MaxEnergy"), P("energy", CombatPlayer, "Stars"),
            P("energy", CombatPlayer, "OrbQueue"), F("energy", "Entities.Orbs.OrbQueue", "_orbs"), P("energy", "Entities.Orbs.OrbQueue", "Capacity"),
            P("powers", Creature, "Powers"), F("powers", Creature, "_powers"), P("powers", PowerModel, "Amount"),
            F("powers", PowerModel, "_amount"), F("powers", PowerModel, "_amountOnTurnStart"), F("powers", PowerModel, "_skipNextDurationTick"),
            F("powers", PowerModel, "_internalData"), P("powers", PowerModel, "Applier"), P("powers", PowerModel, "Target"),
            P("enemies/intents", CombatState, "Enemies"), P("enemies/intents", CombatState, "Allies"),
            P("enemies/intents", CombatState, "EscapedCreatures"), F("enemies/intents", CombatState, "_nextCreatureId"),
            P("enemies/intents", CombatPlayer, "Pets"),
            P("enemies/intents", Creature, "CombatId"), P("enemies/intents", Creature, "Monster"), P("enemies/intents", Creature, "ModelId"),
            P("enemies/intents", Creature, "Side"), P("enemies/intents", Creature, "IsAlive"), P("enemies/intents", Creature, "IsStunned"),
            P("enemies/intents", Creature, "MonsterMaxHpBeforeModification"),
            P("enemies/intents", MonsterModel, "NextMove"), P("enemies/intents", MonsterModel, "MoveStateMachine"),
            P("enemies/intents", MonsterModel, "IsPerformingMove"), P("enemies/intents", MonsterModel, "SpawnedThisTurn"),
            F("enemies/intents", MoveMachine, "_currentState"), F("enemies/intents", MoveMachine, "_performedFirstMove"),
            P("enemies/intents", MoveMachine, "StateLog"), T("enemies/intents", "MonsterMoves.Intents.AbstractIntent"),
            P("damage modifiers", CombatState, "Modifiers"), P("damage modifiers", CombatState, "BadgeModels"),
            P("damage modifiers", CombatState, "MultiplayerScalingModel"), P("damage modifiers", "Rooms.CombatRoom", "GoldProportion"),
            P("pending effects", RunManager, "ActionQueueSet"), P("pending effects", RunManager, "ActionExecutor"),
            P("pending effects", ActionQueueSet, "IsEmpty"), P("pending effects", ActionQueueSet, "NextActionId"),
            F("pending effects", ActionQueueSet, "_actionQueues"), F("pending effects", ActionQueueSet, "_actionsWaitingForResumption"),
            P("pending effects", ActionExecutor, "IsRunning"), P("pending effects", ActionExecutor, "IsPaused"),
            P("pending effects", ActionExecutor, "CurrentlyRunningAction"), P("pending effects", GameAction, "State"),
            P("pending effects", GameAction, "Id"), P("pending effects", GameAction, "OwnerId"),
            F("pending effects", "Entities.Actions.GameActionState", "Executing"), F("pending effects", "Entities.Actions.GameActionState", "Finished"),
            F("selection state", "Entities.Actions.GameActionState", "GatheringPlayerChoice"),
            P("selection state", RunManager, "PlayerChoiceSynchronizer"), F("selection state", "GameActions.Multiplayer.PlayerChoiceContext", "_modelStack"),
            P("selection state", CardModel, "CurrentTarget"), P("selection state", CardModel, "CurrentPlayIndex"), F("selection state", CombatState, "_allCards"),
            P("serialized form", SerializableRoom, "EncounterState"), P("serialized form", SerializableRoom, "IsPreFinished"),
            P("serialized form", SerializableRun, "PreFinishedRoom"), T("serialized form", "Entities.Multiplayer.NetFullCombatState"),
        ]);
}
