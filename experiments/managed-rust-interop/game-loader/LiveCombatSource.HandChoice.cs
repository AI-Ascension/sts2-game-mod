// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Entities.Actions;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.Cards.Holders;
using MegaCrit.Sts2.Core.Nodes.Combat;
using MegaCrit.Sts2.Core.Nodes.CommonUi;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private string? _lastHandDiagnostic;
    private static NPlayerHand? SelectingHand() => NPlayerHand.Instance is { } hand
        && GodotObject.IsInstanceValid(hand) && hand.IsVisibleInTree() && hand.IsInCardSelection
            ? hand : null;

    private NHandCardHolder[] HandUpgradeChoices(NPlayerHand hand) =>
        hand.CurrentMode == NPlayerHand.Mode.UpgradeSelect && CombatChoiceParent() != null
            ? hand.ActiveHolders.Where(holder => GodotObject.IsInstanceValid(holder)
                && holder.IsVisibleInTree() && holder.CardModel != null
                && Clickable(holder.Hitbox)).ToArray()
            : Array.Empty<NHandCardHolder>();

    private RuntimeV3GameplayObservation ProjectHandChoice(RuntimeV3GameplayObservation before,
        NPlayerHand hand)
    {
        string diagnostic = $"mode={hand.CurrentMode}; parent={CombatChoiceParent() != null}; "
            + $"holders={hand.ActiveHolders.Count}; visible={hand.ActiveHolders.Count(h => h.IsVisibleInTree())}; "
            + $"select_mode={hand.ActiveHolders.Count(h => h.InSelectMode)}; "
            + $"clickable={hand.ActiveHolders.Count(h => Clickable(h.Hitbox))}";
        if (_lastHandDiagnostic != diagnostic)
        {
            _lastHandDiagnostic = diagnostic;
            GD.Print("[AI-ASCENSION LIVE] hand selection " + diagnostic);
        }
        string[] choices = HandUpgradeChoices(hand).Select(RewardCardId).ToArray();
        return Surface(before, RuntimeV3GameplayState.Selection, choices,
            choices.Length > 0 && NModalContainer.Instance?.OpenModal == null);
    }

    private LegalActionReference[] HandChoiceActions(RuntimeV3GameplayObservation observation,
        NPlayerHand hand) => HandUpgradeChoices(hand).Select(holder =>
        {
            string value = RewardCardId(holder);
            return new LegalActionReference($"select_card:{observation.Generation}:{value}",
                "select_card", value, null, observation.Generation);
        }).ToArray();

    private bool PrepareHandChoice(LegalActionReference action, out Func<Task> invoke,
        out Func<bool> postcondition, out string effect)
    {
        invoke = () => Task.CompletedTask;
        postcondition = () => false;
        effect = "";
        if (action.Kind != "select_card" || RewardOverlay() != null
            || SelectingHand() is not { } hand || CombatChoiceParent() is not { } parent)
            return false;
        NHandCardHolder? holder = HandUpgradeChoices(hand)
            .SingleOrDefault(candidate => RewardCardId(candidate) == action.Value);
        if (holder == null) return false;
        var card = holder.CardModel!;
        int previousLevel = card.CurrentUpgradeLevel;
        invoke = () => ChooseHandUpgradeAsync(hand, holder);
        postcondition = () => !hand.IsInCardSelection
            && card.CurrentUpgradeLevel > previousLevel
            && parent.HostAction.State == GameActionState.Finished
            && parent.HostAction.CompletionTask.IsCompletedSuccessfully
            && parent.HostAction.Exception == null;
        effect = "combat_hand_upgrade_completed";
        return true;
    }

    private async Task ChooseHandUpgradeAsync(NPlayerHand hand, NHandCardHolder holder)
    {
        if (holder.EmitSignal(NHandCardHolder.SignalName.HolderMouseClicked, holder) != Error.Ok)
            throw new InvalidOperationException("native hand upgrade selection failed");
        bool confirmed = false;
        for (int frame = 0; frame < 600; frame++)
        {
            await WaitCampaignFrameAsync();
            RequireThread();
            if (!hand.IsInCardSelection) return;
            if (hand.CurrentMode != NPlayerHand.Mode.UpgradeSelect || RewardOverlay() != null)
                throw new InvalidOperationException("native hand upgrade surface changed");
            NConfirmButton[] controls = NGame.Instance is { } game
                ? Descendants(game).OfType<NConfirmButton>().Where(Clickable).ToArray()
                : Array.Empty<NConfirmButton>();
            if (!confirmed && controls.Length == 1)
            {
                controls[0].ForceClick();
                confirmed = true;
            }
        }
        throw new InvalidOperationException("native hand upgrade selection did not close");
    }
}
