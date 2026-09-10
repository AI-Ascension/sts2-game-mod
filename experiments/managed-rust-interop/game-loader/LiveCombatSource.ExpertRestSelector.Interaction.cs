// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Entities.RestSite;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.Cards.Holders;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.Combat;
using MegaCrit.Sts2.Core.Nodes.RestSite;
using MegaCrit.Sts2.Core.Nodes.Rooms;
using MegaCrit.Sts2.Core.Nodes.Screens.CardSelection;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private async Task BeginSmithSelectorAsync(
        ExpertRestPending pending,
        SmithRestSiteOption option)
    {
        pending.Button.ForceClick();
        NDeckUpgradeSelectScreen? screen = null;
        for (int frame = 0; frame < 600; frame++)
        {
            await WaitCampaignFrameAsync();
            RequireThread();
            if (RewardOverlay() is NDeckUpgradeSelectScreen candidate)
            {
                screen = candidate;
                break;
            }
        }
        if (screen is null)
            throw new InvalidOperationException("smith selector did not become visible");
        if (CurrentPlayer() is not { } player || option.SmithCount is < 1 or > RuntimeV4ExpertRestActionContract.MaxChoices)
            throw new InvalidOperationException("smith selector has no bounded local deck");

        var selector = new ExpertRestSelector(pending.Operation, pending.Action, pending.Before,
            pending.Button, option, $"selection:{pending.Operation.OperationId}", "card",
            option.SmithCount)
        {
            SmithScreen = screen
        };
        pending.Selector = selector;
        _expertRestSelectors.Add(selector.SelectionId, selector);
        RuntimeV4ExpertGameplayObservation observed = ObserveExpert();
        selector.Generation = observed.Generation;
        if (selector.Generation <= pending.Before.Generation)
            throw new InvalidOperationException("smith selector did not fence its boundary");
    }

    private async Task BeginMendSelectorAsync(
        ExpertRestPending pending,
        MendRestSiteOption option)
    {
        pending.Button.ForceClick();
        NTargetManager targetManager = NTargetManager.Instance;
        for (int frame = 0; frame < 600; frame++)
        {
            await WaitCampaignFrameAsync();
            RequireThread();
            if (targetManager.IsInSelection) break;
        }
        if (!targetManager.IsInSelection)
            throw new InvalidOperationException("mend selector did not enter targeting");
        if (CurrentRestSite() is not { } room)
            throw new InvalidOperationException("mend selector lost its rest room");
        NRestSiteCharacter[] targets = room.Characters
            .Where(character => GodotObject.IsInstanceValid(character)
                && character.IsVisibleInTree() && targetManager.AllowedToTargetNode(character))
            .ToArray();
        if (targets.Length == 0)
            throw new InvalidOperationException("mend selector exposed no legal player target");

        var selector = new ExpertRestSelector(pending.Operation, pending.Action, pending.Before,
            pending.Button, option, $"selection:{pending.Operation.OperationId}", "player", 1);
        selector.MendTargets.AddRange(targets);
        pending.Selector = selector;
        _expertRestSelectors.Add(selector.SelectionId, selector);
        RuntimeV4ExpertGameplayObservation observed = ObserveExpert();
        selector.Generation = observed.Generation;
        if (selector.Generation <= pending.Before.Generation)
            throw new InvalidOperationException("mend selector did not fence its boundary");
    }

    private async Task SelectSmithCardAsync(ExpertRestSelector selector, string cardId)
    {
        if (selector.SmithScreen is not { } screen || RewardOverlay() != screen)
            throw new InvalidOperationException("smith selector screen is unavailable");
        NCardHolder[] matches = RewardCardsForRest(selector)
            .Where(holder => CardId(holder.CardModel!) == cardId).ToArray();
        if (matches.Length != 1 || selector.SelectedCards.Contains(matches[0].CardModel!))
            throw new InvalidOperationException("smith card choice is not unique");
        if (matches[0].EmitSignal(NCardHolder.SignalName.Pressed, matches[0]) != Error.Ok)
            throw new InvalidOperationException("native smith card selection failed");
        selector.SelectedCards.Add(matches[0].CardModel!);
        await WaitCampaignFrameAsync();
        RequireThread();
        if (RewardOverlay() != screen)
            throw new InvalidOperationException("smith selector closed before confirmation");
    }

    private async Task ConfirmSmithSelectionAsync(ExpertRestSelector selector)
    {
        if (selector.SmithScreen is not { } screen || RewardOverlay() != screen
            || selector.SelectedCards.Count != selector.RequiredCount)
            throw new InvalidOperationException("smith confirmation is not legal");
        NConfirmButton[] controls = Descendants(screen).OfType<NConfirmButton>()
            .Where(Clickable).ToArray();
        if (controls.Length != 1)
            throw new InvalidOperationException("smith confirmation control is ambiguous");
        controls[0].ForceClick();
        for (int frame = 0; frame < 600; frame++)
        {
            await WaitCampaignFrameAsync();
            RequireThread();
            if (RewardOverlay() != screen) return;
        }
        throw new InvalidOperationException("smith confirmation did not settle");
    }

    private async Task SelectMendPlayerAsync(ExpertRestSelector selector, string playerId)
    {
        NRestSiteCharacter[] matches = selector.MendTargets
            .Where(character => RestPlayerId(character.Player) == playerId).ToArray();
        if (matches.Length != 1 || !NTargetManager.Instance.IsInSelection)
            throw new InvalidOperationException("mend player choice is not unique");
        NTargetManager targetManager = NTargetManager.Instance;
        NRestSiteCharacter target = matches[0];
        targetManager.OnNodeHovered(target);
        Player player = target.Player;
        selector.MendTargetSnapshot = (U16(player.Creature.CurrentHp),
            U16(player.Creature.MaxHp));
        targetManager._Input(new InputEventMouseButton
        {
            ButtonIndex = MouseButton.Left,
            Pressed = false
        });
        for (int frame = 0; frame < 600; frame++)
        {
            await WaitCampaignFrameAsync();
            RequireThread();
            if (!targetManager.IsInSelection && CurrentRestSite() is { } room)
            {
                NRestSiteButton? current = room.GetButtonForOption(selector.Option);
                if ((current is null || !ReferenceEquals(current, selector.Button))
                    && Clickable(room.ProceedButton))
                {
                    selector.SelectedPlayer = player;
                    return;
                }
            }
        }
        throw new InvalidOperationException("mend player selection did not settle");
    }

    private async Task CancelSelectorAsync(ExpertRestSelector selector)
    {
        if (selector.SelectionKind == "player")
        {
            NTargetManager.Instance.CancelTargeting();
            for (int frame = 0; frame < 120; frame++)
            {
                await WaitCampaignFrameAsync();
                RequireThread();
                if (!NTargetManager.Instance.IsInSelection) return;
            }
            throw new InvalidOperationException("mend cancellation did not settle");
        }
        if (selector.SmithScreen is not { } screen || RewardOverlay() != screen)
            throw new InvalidOperationException("smith selector screen is unavailable");
        NBackButton[] controls = Descendants(screen).OfType<NBackButton>()
            .Where(Clickable).ToArray();
        if (controls.Length == 0)
            throw new InvalidOperationException("smith cancellation control is unavailable");
        controls[0].ForceClick();
        for (int frame = 0; frame < 120; frame++)
        {
            await WaitCampaignFrameAsync();
            RequireThread();
            if (RewardOverlay() != screen) return;
        }
        throw new InvalidOperationException("smith cancellation did not settle");
    }
}
