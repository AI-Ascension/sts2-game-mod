// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Entities.Actions;
using MegaCrit.Sts2.Core.Entities.Cards;
using MegaCrit.Sts2.Core.Entities.Creatures;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private readonly Dictionary<RuntimeV3OperationKey, PendingAction> _pending = new();
    private sealed record PendingAction(GameAction HostAction, LegalActionReference Action,
        RuntimeV3GameplayObservation Before, CardModel? Card);

    public bool Dispatch(RuntimeV3OperationKey operation, LegalActionReference action)
    {
        RequireThread();
        if (_pending.ContainsKey(operation) || _pending.Count >= 4096) return false;
        RuntimeV3GameplayObservation before = Observe();
        if (action.Generation != before.Generation || !LegalActions(before).Contains(action)) return false;
        var player = CurrentPlayer();
        if (player?.PlayerCombatState == null) return false;
        GameAction queued;
        CardModel? card = null;
        if (action.Kind == "end_turn")
            queued = new EndPlayerTurnAction(player, player.PlayerCombatState.TurnNumber);
        else if (action.Kind == "play_card")
        {
            card = player.PlayerCombatState.Hand.Cards.SingleOrDefault(c => CardId(c) == action.Value);
            if (card == null) return false;
            Creature? target = action.TargetId == null
                ? null
                : CombatManager.Instance.DebugOnlyGetState()?.Enemies.SingleOrDefault(e => EnemyId(e) == action.TargetId);
            if (!card.CanPlayTargeting(target)) return false;
            queued = new PlayCardAction(card, target);
        }
        else return false;
        // Retain the exact queued object before enqueue: an exception after submission is unknown.
        _pending.Add(operation, new(queued, action, before, card));
        RunManager.Instance.ActionQueueSynchronizer.RequestEnqueue(queued);
        return true;
    }

    public RuntimeV3HostCompletion? Completion(RuntimeV3OperationKey operation, LegalActionReference action)
    {
        RequireThread();
        if (!_pending.TryGetValue(operation, out PendingAction? pending) || pending.Action != action)
            return null;
        GameAction queued = pending.HostAction;
        if (queued.State != GameActionState.Finished || !queued.CompletionTask.IsCompletedSuccessfully
            || queued.Exception != null) return null;
        RuntimeV3GameplayObservation after = Observe();
        bool effect = action.Kind == "play_card"
            ? pending.Card?.Pile?.Type != PileType.Hand
            : after.TurnIndex > pending.Before.TurnIndex || !after.InputEnabled;
        if (!effect || after.Generation <= pending.Before.Generation) return null;
        var witness = new RuntimeV3TransitionWitness(operation, action, pending.Before.Generation,
            after.Generation, after.StateId, action.Kind == "play_card" ? "play_card_settled" : "turn_end_settled");
        return new(after, witness, LegalActions(after));
    }
}
