// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Entities.Actions;
using MegaCrit.Sts2.Core.Entities.Creatures;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    // The additive v4 transport currently exposes observations only. Keep its mutation owner
    // generation fenced and host-thread bound so a future v4 action route cannot bypass the
    // existing v3 dispatch/reconciliation rules or reinterpret a stale potion identity.
    private readonly Dictionary<RuntimeV3OperationKey, ExpertPotionPending> _expertPotionPending = new();

    private sealed record ExpertPotionPending(
        RuntimeV4ExpertGameplayAction Action,
        RuntimeV4ExpertGameplayObservation Before,
        UsePotionAction HostAction,
        PotionModel Potion);

    /// <summary>
    /// Enqueues one already-admitted v4 potion action against the current native state.
    /// A v4 action endpoint can call this owner after authenticating the operation and checking
    /// its lease; the read-only expert-state endpoint deliberately does not invoke it.
    /// </summary>
    internal bool DispatchExpert(RuntimeV3OperationKey operation,
        RuntimeV4ExpertGameplayAction action)
    {
        RequireThread();
        if (_expertPotionPending.ContainsKey(operation) || _expertPotionPending.Count >= 4096)
            return false;

        RuntimeV4ExpertGameplayObservation before = ObserveExpert();
        RuntimeV4ExpertGameplayAction? current = before.LegalActions
            .SingleOrDefault(candidate => candidate == action);
        if (current is null || action.Kind != "use_potion") return false;
        Player? player = CurrentPlayer();
        if (player is null || player.PlayerCombatState is null) return false;
        PotionModel? potion = player.Potions.SingleOrDefault(candidate =>
            PotionId(candidate, player.GetPotionSlotIndex(candidate)) == action.Value);
        if (potion is null || PotionUsable(player, potion, player.GetPotionSlotIndex(potion)) != true)
            return false;

        Creature? target = ExpertPotionTarget(action.TargetId, player);
        string targetMode = PotionTargetMode(potion);
        if ((targetMode is "self" or "any_enemy") && target is null) return false;
        if (targetMode is "none" or "all_enemies")
        {
            if (target is not null) return false;
        }
        else if (!PotionAcceptsTarget(potion, target))
        {
            return false;
        }

        var queued = new UsePotionAction(potion, target, isCombatInProgress: true);
        _expertPotionPending.Add(operation, new(action, before, queued, potion));
        RunManager.Instance.ActionQueueSynchronizer.RequestEnqueue(queued);
        return true;
    }

    /// <summary>
    /// Returns settlement only after the exact queued native action finished, advanced the
    /// generation, and removed the addressed potion instance from the player's belt.
    /// </summary>
    internal RuntimeV4ExpertGameplayObservation? CompleteExpert(
        RuntimeV3OperationKey operation,
        RuntimeV4ExpertGameplayAction action)
    {
        RequireThread();
        if (!_expertPotionPending.TryGetValue(operation, out ExpertPotionPending? pending)
            || pending.Action != action)
            return null;
        UsePotionAction queued = pending.HostAction;
        if (queued.State != GameActionState.Finished
            || !queued.CompletionTask.IsCompletedSuccessfully
            || queued.Exception is not null)
            return null;

        RuntimeV4ExpertGameplayObservation after = ObserveExpert();
        // Compare the retained host instance, not only its slot-derived wire ID. A duplicate
        // potion can shift into the same slot after use and must not counterfeit settlement.
        bool removed = pending.Potion.HasBeenRemovedFromState
            || CurrentPlayer()?.Potions.Contains(pending.Potion) != true;
        if (!removed || after.Generation <= pending.Before.Generation) return null;
        _expertPotionPending.Remove(operation);
        return after;
    }

    private static Creature? ExpertPotionTarget(string? targetId, Player player)
    {
        if (targetId is null) return null;
        if (targetId == "player:local") return player.Creature;
        if (!targetId.StartsWith("enemy:", StringComparison.Ordinal)) return null;
        return CombatManager.Instance.DebugOnlyGetState()?.Enemies
            .SingleOrDefault(enemy => EnemyId(enemy) == targetId);
    }
}
