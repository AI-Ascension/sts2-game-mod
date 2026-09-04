// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text;
using Godot;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Context;
using MegaCrit.Sts2.Core.Entities.Cards;
using MegaCrit.Sts2.Core.Entities.Creatures;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static string? RuntimeV3GameplayPreconditionError(RuntimeV3GameplayHostObservation observation)
    {
        if (!observation.HostReady)
        {
            return "sts2.runtime/host_not_ready";
        }

        return observation.CombatPhase switch
        {
            RuntimeV3GameplayOutsideCombat => "sts2.game-core/outside_combat",
            RuntimeV3GameplayEnemyTurn => "sts2.game-core/not_player_turn",
            RuntimeV3GameplayPlayerTurn => RuntimeV2PlayerActionsDisabled(),
            _ => "sts2.game-core/phase_unavailable"
        };
    }

    private static bool TryGetRuntimeV3GameplayPlayer(out Player? player)
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

    private static bool TryQueueRuntimeV3GameplayAction(
        RuntimeV3GameplayRequest request,
        out string error)
    {
        error = "sts2.runtime/host_action_unavailable";
        if (!TryGetRuntimeV3GameplayPlayer(out Player? player) || player?.PlayerCombatState == null)
        {
            error = "sts2.game-mod/local_player_unavailable";
            return false;
        }

        CardPile hand = player.PlayerCombatState.Hand;
        if (hand == null || request.CardIndex >= hand.Cards.Count)
        {
            error = "sts2.game-core/card_not_in_hand";
            return false;
        }

        CardModel card = hand.Cards[request.CardIndex];
        if (!card.CanPlay())
        {
            error = "sts2.game-core/card_unplayable";
            return false;
        }

        CombatState? combatState = CombatManager.Instance?.DebugOnlyGetState();
        if (combatState == null)
        {
            error = "sts2.runtime/host_not_ready";
            return false;
        }

        Creature? target = ResolveRuntimeV3GameplayTarget(combatState, player, request.TargetId);
        if (target == null)
        {
            error = "sts2.game-core/target_not_found";
            return false;
        }
        if (request.TargetId != null && !card.CanPlayTargeting(target))
        {
            error = "sts2.game-core/target_illegal";
            return false;
        }

        try
        {
            if (RunManager.Instance?.ActionQueueSynchronizer == null)
            {
                error = "sts2.runtime/action_queue_unavailable";
                return false;
            }

            RunManager.Instance.ActionQueueSynchronizer.RequestEnqueue(new PlayCardAction(card, target));
            return true;
        }
        catch (Exception exception)
        {
            GD.PrintErr($"{LogPrefix} Runtime-v3 gameplay card dispatch failed: {exception.GetType().Name}");
            error = "sts2.runtime/host_action_exception";
            return false;
        }
    }

    private static Creature? ResolveRuntimeV3GameplayTarget(
        CombatState combatState,
        Player player,
        string? targetId)
    {
        if (targetId == null)
        {
            return player.Creature;
        }

        foreach (Creature enemy in combatState.Enemies)
        {
            if (enemy.CombatId is uint combatId
                && combatId.ToString(CultureInfo.InvariantCulture) == targetId)
            {
                return enemy;
            }
        }

        return null;
    }

    private static void TryFinalizePendingRuntimeV3Gameplay()
    {
        RuntimeV3GameplayOperation? operation = _runtimeV3GameplayPending;
        if (operation == null || operation.Status == "rejected") return;
        // RequestEnqueue provides no operation-bound completion evidence in this candidate.
        // A count/energy change can belong to another action; never manufacture a witness.
        operation.Status = "unknown";
        operation.Observation = null;
        operation.ErrorCode ??= "sts2.runtime/completion_unverified";
    }

}
