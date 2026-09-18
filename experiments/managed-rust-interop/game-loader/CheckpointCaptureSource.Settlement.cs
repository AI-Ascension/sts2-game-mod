// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Settlement plus the stable detail token that explains a non-quiescent result.</summary>
internal readonly record struct CheckpointSettlementClassification(
    CheckpointSettlement Settlement,
    string Detail);

internal sealed partial class CheckpointCaptureSource
{
    /// <summary>
    /// Maps one host witness to <c>Quiescent / MidEffect / EnemyExecution /
    /// PendingSelectionTransition / Unknown</c>. Checks run from the least to the most specific
    /// so an unsafe host phase is never reported as a mere boundary mismatch. Only the first
    /// failing check is reported; the classifier reads nothing from the host itself.
    /// </summary>
    internal static CheckpointSettlementClassification Classify(
        CheckpointHostSettlementWitness witness, CheckpointCaptureBoundary boundary)
    {
        if (witness is null)
            return Unknown("witness_missing");
        if (!witness.RunInProgress || !witness.RunStateAvailable)
            return Unknown("run_not_active");
        if (!witness.SinglePlayer)
            return Unknown("not_single_player");
        if (witness.GameOver)
            return Unknown("game_over");
        if (witness.Abandoned)
            return Unknown("run_abandoned");
        if (witness.ActionExecutorPaused)
            return Unknown("action_executor_paused");
        if (witness.CombatPaused)
            return Unknown("combat_paused");

        if (witness.CurrentAction == CheckpointHostActionState.GatheringPlayerChoice)
            return Pending("player_choice_gathering");
        if (witness.ModalOpen)
            return Pending("modal_open");

        if (witness.EnemyTurnStarted)
            return Enemy("enemy_turn_started");
        if (witness.EnemySideActive)
            return Enemy("enemy_side_active");
        if (witness.AnyMonsterPerformingMove)
            return Enemy("monster_performing_move");

        if (witness.ActionExecutorRunning
            || witness.CurrentAction == CheckpointHostActionState.Executing)
            return Mid("action_executing");
        if (witness.CurrentAction == CheckpointHostActionState.Other)
            return Unknown("action_state_unclassified");
        if (!witness.ActionQueueEmpty)
            return Mid("action_queue_pending");
        if (witness.SavePending)
            return Mid("run_save_pending");
        if (witness.CombatStarting)
            return Mid("combat_starting");
        if (witness.CombatEnding)
            return Mid("combat_ending");
        if (witness.PlayerTurnEnding || witness.TurnPhase == CheckpointHostTurnPhase.End)
            return Mid("player_turn_ending");

        return boundary switch
        {
            CheckpointCaptureBoundary.SettledMapChoice => ClassifyMapChoice(witness),
            CheckpointCaptureBoundary.StablePlayerTurnCombat => ClassifyCombat(witness, 1),
            CheckpointCaptureBoundary.LaterTurnCombat => ClassifyCombat(witness, 2),
            _ => Unknown("boundary_without_settlement_rule")
        };
    }

    private static CheckpointSettlementClassification ClassifyMapChoice(
        CheckpointHostSettlementWitness witness)
    {
        if (witness.CombatInProgress || witness.TurnPhase != CheckpointHostTurnPhase.None)
            return Unknown("host_phase_mismatch:combat_active");
        if (witness.Room != CheckpointHostRoomPhase.MapChoice)
            return Unknown("host_phase_mismatch:map_not_open");
        return Quiescent();
    }

    private static CheckpointSettlementClassification ClassifyCombat(
        CheckpointHostSettlementWitness witness, long minimumTurn)
    {
        if (!witness.CombatInProgress || witness.Room != CheckpointHostRoomPhase.Combat)
            return Unknown("host_phase_mismatch:combat_not_active");
        if (witness.TurnPhase != CheckpointHostTurnPhase.Play)
            return Unknown("host_phase_mismatch:not_player_play_phase");
        if (witness.PlayerActionsDisabled)
            return Mid("player_actions_disabled");
        if (witness.TurnNumber < minimumTurn)
            return Unknown("turn_witness_below_minimum");
        return Quiescent();
    }

    private static CheckpointSettlementClassification Quiescent() =>
        new(CheckpointSettlement.Quiescent, string.Empty);

    private static CheckpointSettlementClassification Mid(string detail) =>
        new(CheckpointSettlement.MidEffect, detail);

    private static CheckpointSettlementClassification Enemy(string detail) =>
        new(CheckpointSettlement.EnemyExecution, detail);

    private static CheckpointSettlementClassification Pending(string detail) =>
        new(CheckpointSettlement.PendingSelectionTransition, detail);

    private static CheckpointSettlementClassification Unknown(string detail) =>
        new(CheckpointSettlement.Unknown, detail);
}
