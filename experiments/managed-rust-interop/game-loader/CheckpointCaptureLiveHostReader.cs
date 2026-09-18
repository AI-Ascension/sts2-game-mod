// SPDX-License-Identifier: MIT

using System.Threading.Tasks;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Context;
using MegaCrit.Sts2.Core.Entities.Actions;
using MegaCrit.Sts2.Core.Entities.Creatures;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.Screens.Map;
using MegaCrit.Sts2.Core.Rooms;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Saves;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Host-bound reader for <see cref="CheckpointCaptureSource"/>. Every member it touches is a
/// public, metadata-observed row of <c>docs/evidence/checkpoint-coverage-inventory-20260917</c>;
/// it reads on the calling (host) thread, copies values, and retains nothing. Nothing constructs
/// this reader in production: the seam is unwired until exact-host evidence exists.
/// </summary>
internal sealed partial class CheckpointCaptureLiveHostReader : ICheckpointCaptureHostReader
{
    public CheckpointHostSettlementWitness ReadSettlement()
    {
        RunManager run = RunManager.Instance;
        bool inProgress = run.IsInProgress;
        RunState? state = inProgress ? run.DebugOnlyGetState() : null;
        bool singlePlayer = state is not null && state.Players.Count == 1
            && run.NetService.Type == NetGameType.Singleplayer;
        Player? player = state is null ? null : LocalContext.GetMe(state);
        PlayerCombatState? playerCombat = player?.PlayerCombatState;

        Task? saveTask = SaveManager.Instance.CurrentRunSaveTask;
        ActionExecutor executor = run.ActionExecutor;
        GameAction? current = executor.CurrentlyRunningAction;
        CheckpointHostActionState action = current is null
            ? CheckpointHostActionState.None
            : current.State switch
            {
                GameActionState.Executing => CheckpointHostActionState.Executing,
                GameActionState.GatheringPlayerChoice =>
                    CheckpointHostActionState.GatheringPlayerChoice,
                GameActionState.Finished => CheckpointHostActionState.Finished,
                _ => CheckpointHostActionState.Other
            };

        CombatManager combat = CombatManager.Instance;
        bool combatActive = false;
        bool enemySide = false;
        bool performingMove = false;
        if (combat.DebugOnlyGetState() is { } combatState && playerCombat is not null
            && player is not null)
        {
            combatActive = combat.IsInProgress;
            enemySide = combatState.CurrentSide != player.Creature.Side;
            foreach (Creature enemy in combatState.Enemies)
                performingMove |= enemy.Monster?.IsPerformingMove == true;
        }

        bool mapOpen = NMapScreen.Instance is { IsOpen: true } map && map.IsVisibleInTree();
        CheckpointHostRoomPhase room = state is null
            ? CheckpointHostRoomPhase.Unknown
            : state.CurrentRoom is CombatRoom { IsPreFinished: true }
                ? CheckpointHostRoomPhase.CombatReward
                : combatActive
                    ? CheckpointHostRoomPhase.Combat
                    : mapOpen
                        ? CheckpointHostRoomPhase.MapChoice
                        : state.CurrentRoom is null
                            ? CheckpointHostRoomPhase.Unknown
                            : CheckpointHostRoomPhase.Other;

        CheckpointHostTurnPhase turnPhase = playerCombat is null
            ? CheckpointHostTurnPhase.None
            : playerCombat.Phase == PlayerTurnPhase.Play
                ? CheckpointHostTurnPhase.Play
                : playerCombat.Phase == PlayerTurnPhase.End
                    ? CheckpointHostTurnPhase.End
                    : CheckpointHostTurnPhase.Other;

        return new CheckpointHostSettlementWitness(
            RunInProgress: inProgress,
            RunStateAvailable: state is not null,
            SinglePlayer: singlePlayer,
            GameOver: run.IsGameOver,
            Abandoned: run.IsAbandoned,
            SavePending: saveTask is { IsCompleted: false },
            ActionQueueEmpty: run.ActionQueueSet.IsEmpty,
            ActionExecutorRunning: executor.IsRunning,
            ActionExecutorPaused: executor.IsPaused,
            CurrentAction: action,
            ModalOpen: NModalContainer.Instance?.OpenModal is not null,
            Room: room,
            CombatInProgress: combatActive,
            CombatStarting: combat.IsStarting,
            CombatEnding: combat.IsEnding,
            CombatPaused: combat.IsPaused,
            EnemyTurnStarted: combat.IsEnemyTurnStarted,
            PlayerTurnEnding: combat.EndingPlayerTurnPhaseOne || combat.EndingPlayerTurnPhaseTwo,
            PlayerActionsDisabled: combat.PlayerActionsDisabled,
            EnemySideActive: enemySide,
            AnyMonsterPerformingMove: performingMove,
            TurnPhase: turnPhase,
            TurnNumber: playerCombat?.TurnNumber ?? 0);
    }
}
