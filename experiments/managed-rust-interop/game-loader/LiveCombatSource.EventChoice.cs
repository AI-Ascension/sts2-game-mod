// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Nodes.Cards.Holders;
using MegaCrit.Sts2.Core.Nodes.Screens.CardSelection;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private CampaignPending? EventChoiceParent()
    {
        CampaignPending[] matches = _campaignPending.Values.Where(pending =>
            pending.Action.Kind == "event_choice" && pending.Before.State == RuntimeV3GameplayState.Event
            && pending.Action.Value?.StartsWith("event:", StringComparison.Ordinal) == true
            && pending.Work is { IsCompleted: false }).ToArray();
        return matches.Length == 1 ? matches[0] : null;
    }

    private bool HasEventCardChoice(Node? screen) => screen is NDeckUpgradeSelectScreen or NDeckCardSelectScreen
        or NDeckEnchantSelectScreen or NSimpleCardSelectScreen
        && EventChoiceParent() != null && RewardCards(screen).Length > 0;

    private RuntimeV3HostCompletion? EventChoiceBoundary(RuntimeV3OperationKey operation,
        CampaignPending pending)
    {
        if (!ReferenceEquals(EventChoiceParent(), pending) || !HasEventCardChoice(RewardOverlay()))
            return null;
        RuntimeV3GameplayObservation after = Observe();
        if (after.State != RuntimeV3GameplayState.Selection || !after.InputEnabled
            || after.Generation <= pending.Before.Generation) return null;
        var witness = new RuntimeV3TransitionWitness(operation, pending.Action, pending.Before.Generation,
            after.Generation, after.StateId, "event_card_choice_requested");
        return new(after, witness, LegalActions(after));
    }

    private bool PrepareEventCardChoice(LegalActionReference action, out Func<Task> invoke,
        out Func<bool> postcondition, out string effect)
    {
        if (PrepareEventRemoval(action, out invoke, out postcondition, out effect)) return true;
        if (PrepareEventEnchantment(action, out invoke, out postcondition, out effect)) return true;
        if (PrepareEventCardAddition(action, out invoke, out postcondition, out effect)) return true;
        invoke = () => Task.CompletedTask;
        postcondition = () => false;
        effect = "";
        if (action.Kind != "select_card" || RewardOverlay() is not NDeckUpgradeSelectScreen screen
            || EventChoiceParent() is not { } parent) return false;
        NCardHolder? holder = RewardCards(screen).SingleOrDefault(card => RewardCardId(card) == action.Value);
        if (holder?.CardModel is not { } card) return false;
        int beforeLevel = card.CurrentUpgradeLevel;
        invoke = async () =>
        {
            if (holder.EmitSignal(NCardHolder.SignalName.Pressed, holder) != Error.Ok)
                throw new InvalidOperationException("native event upgrade selection failed");
            await ConfirmSmithAsync(screen, card, beforeLevel);
        };
        postcondition = () => RewardOverlay() != screen && card.CurrentUpgradeLevel > beforeLevel
            && parent.Work?.IsCompletedSuccessfully == true && parent.Postcondition();
        effect = "event_card_upgrade_completed";
        return true;
    }
}
