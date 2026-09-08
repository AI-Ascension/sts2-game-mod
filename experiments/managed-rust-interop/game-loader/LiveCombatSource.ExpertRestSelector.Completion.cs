// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using Godot;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.Cards.Holders;
using MegaCrit.Sts2.Core.Nodes.CommonUi;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private RuntimeV4ExpertRestHostCompletion? CompleteSelectorAction(
        ExpertRestPending pending)
    {
        if (pending.Selector is not { } selector)
            return null;

        if (pending.Action.Action.Kind == "rest_option")
        {
            RuntimeV4ExpertGameplayObservation after = ObserveExpert();
            if (after.Generation <= pending.Before.Generation) return null;
            RuntimeV4ExpertRestSelector wire = BuildSelector(selector, after.Generation);
            var transition = new RuntimeV4ExpertRestSelectionRequestedTransition(
                WireRestOptionId(selector.Option.OptionId)!, pending.Before.Generation,
                after.Generation, wire);
            var completion = new RuntimeV4ExpertRestHostCompletion(
                "settled", SelectorProjection(after, selector).Observation, transition, null, null);
            _expertRestPending.Remove(pending.Operation);
            pending.Completion = completion;
            return completion;
        }

        selector.ChildOperation = null;
        if (pending.Action.Action.Kind == "cancel_selection")
        {
            _expertRestSelectors.Remove(selector.SelectionId);
            var cancelled = new RuntimeV4ExpertRestHostCompletion(
                "cancelled", null, null, null, "sts2.game-mod/selection_cancelled");
            _expertRestPending.Remove(pending.Operation);
            pending.Completion = cancelled;
            return cancelled;
        }

        RuntimeV4ExpertGameplayObservation before = pending.Before;
        if (pending.Action.Action.Kind == "select_player")
        {
            if (selector.SelectedPlayer is null) return null;
            _expertRestSelectors.Remove(selector.SelectionId);
            RuntimeV4ExpertGameplayObservation after = ObserveExpert();
            if (after.Generation <= before.Generation) return null;
            string targetId = RestPlayerId(selector.SelectedPlayer);
            var witness = new RuntimeV4ExpertRestEffectWitness(
                "mend_applied", pending.Operation, "mend", after.Generation,
                new RuntimeV4ExpertRestNativeEvidence(
                    $"mend:{pending.Operation.OperationId}:{targetId}", after.StateId), targetId);
            var transition = new RuntimeV4ExpertRestSelectionCompletedTransition(
                "mend", before.Generation, after.Generation, selector.SelectionId, "player", 1,
                new[] { targetId }, witness);
            var completed = new RuntimeV4ExpertRestHostCompletion(
                "settled", after, transition, witness, null);
            _expertRestPending.Remove(pending.Operation);
            pending.Completion = completed;
            return completed;
        }

        RuntimeV4ExpertGameplayObservation smithAfter = ObserveExpert();
        if (pending.Action.Action.Kind == "select_card")
        {
            if (smithAfter.Generation <= before.Generation) return null;
            RuntimeV4ExpertRestSelector wire = BuildSelector(selector, smithAfter.Generation);
            var transition = new RuntimeV4ExpertRestSelectionProgressedTransition(
                "smith", before.Generation, smithAfter.Generation, wire);
            var progressed = new RuntimeV4ExpertRestHostCompletion(
                "settled", SelectorProjection(smithAfter, selector).Observation,
                transition, null, null);
            _expertRestPending.Remove(pending.Operation);
            pending.Completion = progressed;
            return progressed;
        }

        if (pending.Action.Action.Kind != "confirm_selection"
            || selector.SelectedCards.Count != selector.RequiredCount)
            return null;
        _expertRestSelectors.Remove(selector.SelectionId);
        RuntimeV4ExpertGameplayObservation afterSmith = ObserveExpert();
        if (afterSmith.Generation <= before.Generation) return null;
        RuntimeV4ExpertRestCardEvidence? evidence = CardEvidence(
            before.Player.Deck, afterSmith.Player.Deck, false, false, true);
        if (evidence is null) return null;
        var smithWitness = new RuntimeV4ExpertRestEffectWitness(
            "smith_applied", pending.Operation, "smith", afterSmith.Generation, evidence);
        var smithTransition = new RuntimeV4ExpertRestSelectionCompletedTransition(
            "smith", before.Generation, afterSmith.Generation, selector.SelectionId, "card",
            selector.RequiredCount, selector.SelectedCards.Select(CardId).ToArray(), smithWitness);
        var smithCompletion = new RuntimeV4ExpertRestHostCompletion(
            "settled", afterSmith, smithTransition, smithWitness, null);
        _expertRestPending.Remove(pending.Operation);
        pending.Completion = smithCompletion;
        return smithCompletion;
    }
}
