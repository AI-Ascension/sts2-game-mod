// SPDX-License-Identifier: MIT

using System;
using Godot;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Context;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static object? _runtimeV2PreviousRun;
    private static object? _runtimeV2PreviousCombat;
    private static object? _runtimeV2PreviousPlayer;
    private static bool _runtimeV2IdentityChanged;

    private static string? RuntimeV2ActionPreconditionError(RuntimeV2HostObservation observation)
    {
        if (!observation.HostReady)
        {
            return "sts2.runtime/host_not_ready";
        }

        return observation.CombatPhase switch
        {
            RuntimeV2OutsideCombat => "sts2.game-core/outside_combat",
            RuntimeV2EnemyTurn => "sts2.game-core/not_player_turn",
            RuntimeV2PlayerTurn => RuntimeV2PlayerActionsDisabled(),
            _ => "sts2.game-core/phase_unavailable"
        };
    }

    private static string? RuntimeV2PlayerActionsDisabled()
    {
        try
        {
            CombatManager? combatManager = CombatManager.Instance;
            if (combatManager == null)
            {
                return "sts2.runtime/host_not_ready";
            }

            return combatManager.PlayerActionsDisabled
                ? "sts2.game-core/player_actions_disabled"
                : null;
        }
        catch
        {
            return "sts2.runtime/host_not_ready";
        }
    }

    private static bool TryGetRuntimeV2Player(out Player? player)
    {
        player = null;
        try
        {
            if (RunManager.Instance == null || !RunManager.Instance.IsInProgress)
            {
                return false;
            }

            RunState runState = RunManager.Instance.DebugOnlyGetState()!;
            player = LocalContext.GetMe(runState);
            return player != null;
        }
        catch
        {
            return false;
        }
    }

    private static void TryFinalizePendingRuntimeV2()
    {
        RuntimeV2Operation? operation = _runtimeV2Pending;
        if (operation == null || operation.Status == "rejected") return;
        // A later player turn does not prove this queued operation completed.
        operation.Status = "unknown";
        operation.Observation = null;
        operation.ErrorCode ??= "sts2.runtime/completion_unverified";
    }

    private static RuntimeV2HostObservation ReadRuntimeV2Observation()
    {
        RuntimeV2HostObservation raw = ReadRuntimeV2HostObservation();
        if (raw.HostReady)
        {
            if (!_runtimeV2HostBaseline)
            {
                _runtimeV2HostBaseline = true;
            }
            else if (_runtimeV2IdentityChanged
                || raw.CombatPhase != _runtimeV2LastPhase
                || raw.TurnIndex != _runtimeV2LastTurnIndex)
            {
                _runtimeGeneration++;
            }

            _runtimeV2LastPhase = raw.CombatPhase;
            _runtimeV2LastTurnIndex = raw.TurnIndex;
            _runtimeV2IdentityChanged = false;
        }

        return raw with { Generation = _runtimeGeneration };
    }

    private static RuntimeV2HostObservation ReadRuntimeV2HostObservation()
    {
        try
        {
            if (Engine.GetMainLoop() is not SceneTree tree || tree.Root == null)
            {
                return new RuntimeV2HostObservation(RuntimeV2OutsideCombat, 0, false, 0);
            }

            CombatManager? combatManager = CombatManager.Instance;
            if (combatManager == null || !combatManager.IsInProgress)
            {
                return new RuntimeV2HostObservation(RuntimeV2OutsideCombat, 0, true, 0);
            }

            CombatState? combatState = combatManager.DebugOnlyGetState();
            if (combatState == null)
            {
                return new RuntimeV2HostObservation(RuntimeV2OutsideCombat, 0, false, 0);
            }

            if (!TryGetRuntimeV2Player(out Player? player) || player?.PlayerCombatState == null)
            {
                return new RuntimeV2HostObservation(RuntimeV2OutsideCombat, 0, false, 0);
            }

            ushort turnIndex = BoundedTurnIndex(player.PlayerCombatState.TurnNumber);
            object? run = RunManager.Instance.DebugOnlyGetState();
            _runtimeV2IdentityChanged = !ReferenceEquals(run, _runtimeV2PreviousRun)
                || !ReferenceEquals(combatState, _runtimeV2PreviousCombat)
                || !ReferenceEquals(player, _runtimeV2PreviousPlayer);
            _runtimeV2PreviousRun = run;
            _runtimeV2PreviousCombat = combatState;
            _runtimeV2PreviousPlayer = player;
            string phase = combatManager.IsEnemyTurnStarted ? RuntimeV2EnemyTurn : RuntimeV2PlayerTurn;
            return new RuntimeV2HostObservation(phase, turnIndex, true, 0);
        }
        catch
        {
            return new RuntimeV2HostObservation(RuntimeV2OutsideCombat, 0, false, 0);
        }
    }

    private static ushort BoundedTurnIndex(int roundNumber)
    {
        if (roundNumber <= 0)
        {
            return 0;
        }

        return roundNumber >= RuntimeV2MaxTurnIndex
            ? RuntimeV2MaxTurnIndex
            : (ushort)roundNumber;
    }
}
