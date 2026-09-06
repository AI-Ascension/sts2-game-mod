// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Entities.Actions;
using MegaCrit.Sts2.Core.Entities.Cards;
using MegaCrit.Sts2.Core.Nodes.Cards.Holders;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.Screens.CardSelection;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private PendingAction? CombatChoiceParent()
    {
        PendingAction[] matches = _pending.Values.Where(pending => pending.Action.Kind == "play_card"
            && pending.Card != null && pending.Card.Pile?.Type != PileType.Hand
            && pending.HostAction.State == GameActionState.GatheringPlayerChoice
            && pending.HostAction.Exception == null).ToArray();
        return matches.Length == 1 ? matches[0] : null;
    }

    private bool HasCombatChoice(Node? screen) => screen is NCombatPileCardSelectScreen
        && CombatChoiceParent() != null && RewardCards(screen).Length > 0;

    private RuntimeV3HostCompletion? CombatChoiceBoundary(RuntimeV3OperationKey operation,
        PendingAction pending)
    {
        if (!ReferenceEquals(CombatChoiceParent(), pending) || !HasCombatChoice(RewardOverlay()))
            return null;
        RuntimeV3GameplayObservation after = Observe();
        if (after.State != RuntimeV3GameplayState.Selection || !after.InputEnabled
            || after.Generation <= pending.Before.Generation) return null;
        var witness = new RuntimeV3TransitionWitness(operation, pending.Action,
            pending.Before.Generation, after.Generation, after.StateId, "card_play_choice_requested");
        return new(after, witness, LegalActions(after));
    }

    private bool PrepareCombatChoice(LegalActionReference action, out Func<Task> invoke,
        out Func<bool> postcondition, out string effect)
    {
        invoke = () => Task.CompletedTask;
        postcondition = () => false;
        effect = "";
        if (action.Kind != "select_card" || RewardOverlay() is not NCombatPileCardSelectScreen screen
            || CombatChoiceParent() is not { } parent) return false;
        NCardHolder? holder = RewardCards(screen).SingleOrDefault(card => RewardCardId(card) == action.Value);
        if (holder == null) return false;
        invoke = () => ChooseCombatCardAsync(screen, holder);
        postcondition = () => RewardOverlay() != screen
            && parent.HostAction.State == GameActionState.Finished
            && parent.HostAction.CompletionTask.IsCompletedSuccessfully
            && parent.HostAction.Exception == null;
        effect = "combat_card_choice_completed";
        return true;
    }

    private async Task ChooseCombatCardAsync(NCombatPileCardSelectScreen screen, NCardHolder holder)
    {
        if (holder.EmitSignal(NCardHolder.SignalName.Pressed, holder) != Error.Ok)
            throw new InvalidOperationException("native combat card selection failed");
        bool confirmed = false;
        for (int frame = 0; frame < 600; frame++)
        {
            await WaitCampaignFrameAsync();
            RequireThread();
            if (RewardOverlay() != screen) return;
            NConfirmButton[] controls = Descendants(screen).OfType<NConfirmButton>().Where(Clickable).ToArray();
            if (!confirmed && controls.Length == 1)
            {
                controls[0].ForceClick();
                confirmed = true;
            }
        }
        throw new InvalidOperationException("native combat card selection did not close");
    }
}
