// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Entities.RestSite;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.Cards.Holders;
using MegaCrit.Sts2.Core.Nodes.Combat;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.RestSite;
using MegaCrit.Sts2.Core.Nodes.Rooms;
using MegaCrit.Sts2.Core.Nodes.Screens.CardSelection;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private sealed class ExpertRestSelector
    {
        internal ExpertRestSelector(
            RuntimeV4ExpertRestOperation operation,
            RuntimeV4ExpertRestActionReference parentAction,
            RuntimeV4ExpertGameplayObservation before,
            NRestSiteButton button,
            RestSiteOption option,
            string selectionId,
            string selectionKind,
            int requiredCount)
        {
            Operation = operation;
            ParentAction = parentAction;
            Before = before;
            Button = button;
            Option = option;
            SelectionId = selectionId;
            SelectionKind = selectionKind;
            RequiredCount = requiredCount;
        }

        internal RuntimeV4ExpertRestOperation Operation { get; }
        internal RuntimeV4ExpertRestActionReference ParentAction { get; }
        internal RuntimeV4ExpertGameplayObservation Before { get; }
        internal NRestSiteButton Button { get; }
        internal RestSiteOption Option { get; }
        internal string SelectionId { get; }
        internal string SelectionKind { get; }
        internal int RequiredCount { get; }
        internal ulong Generation { get; set; }
        internal NDeckUpgradeSelectScreen? SmithScreen { get; set; }
        internal List<CardModel> SelectedCards { get; } = new();
        internal List<NRestSiteCharacter> MendTargets { get; } = new();
        internal Player? SelectedPlayer { get; set; }
        internal RuntimeV4ExpertRestOperation? ChildOperation { get; set; }
    }

    private RuntimeV4ExpertRestHostProjection SelectorProjection(
        RuntimeV4ExpertGameplayObservation observation,
        ExpertRestSelector selector)
    {
        selector.Generation = observation.Generation;
        RuntimeV4ExpertRestSelector wire = BuildSelector(selector, observation.Generation);
        RuntimeV4ExpertGameplayChoice[] choices = selector.SelectionKind == "card"
            ? RewardCardsForRest(selector).Select(holder => new RuntimeV4ExpertGameplayChoice(
                CardId(holder.CardModel!), holder.CardModel!.Title, "card", null)).ToArray()
            : selector.MendTargets.Select(character => new RuntimeV4ExpertGameplayChoice(
                RestPlayerId(character.Player), RestPlayerLabel(character.Player), "player", null)).ToArray();
        RuntimeV4ExpertGameplayObservation projected = observation with
        {
            State = new RuntimeV4ExpertGameplayState("selection") { Choices = choices },
            LegalActions = Array.Empty<RuntimeV4ExpertGameplayAction>()
        };
        return new RuntimeV4ExpertRestHostProjection(projected, wire.LegalActions, wire);
    }

    private RuntimeV4ExpertRestSelector BuildSelector(
        ExpertRestSelector selector,
        ulong generation)
    {
        var actions = new List<RuntimeV4ExpertRestActionReference>();
        if (selector.SelectionKind == "card")
        {
            foreach (NCardHolder holder in RewardCardsForRest(selector))
            {
                CardModel card = holder.CardModel!;
                string cardId = CardId(card);
                if (selector.SelectedCards.Contains(card)) continue;
                actions.Add(SelectorAction(generation, selector, "select_card", cardId));
            }
            if (selector.SelectedCards.Count == selector.RequiredCount)
                actions.Add(SelectorAction(generation, selector, "confirm_selection", null));
        }
        else
        {
            foreach (NRestSiteCharacter character in selector.MendTargets)
            {
                if (selector.SelectedPlayer is not null) break;
                actions.Add(SelectorAction(generation, selector, "select_player",
                    RestPlayerId(character.Player)));
            }
        }
        actions.Add(SelectorAction(generation, selector, "cancel_selection", null));
        string[] selected = selector.SelectionKind == "card"
            ? selector.SelectedCards.Select(CardId).ToArray()
            : selector.SelectedPlayer is null
                ? Array.Empty<string>()
                : new[] { RestPlayerId(selector.SelectedPlayer) };
        return new RuntimeV4ExpertRestSelector(selector.SelectionId, selector.SelectionKind,
            selector.RequiredCount, selected, selector.RequiredCount - selected.Length, actions);
    }

    private static RuntimeV4ExpertRestActionReference SelectorAction(
        ulong generation,
        ExpertRestSelector selector,
        string kind,
        string? choice)
    {
        string suffix = choice is null ? kind : $"{kind}:{choice}";
        RuntimeV4ExpertRestAction action = kind switch
        {
            "select_card" => new(kind, "smith", selector.SelectionId, CardId: choice),
            "select_player" => new(kind, "mend", selector.SelectionId, PlayerId: choice),
            _ => new(kind, selector.SelectionKind == "card" ? "smith" : "mend",
                selector.SelectionId)
        };
        return new RuntimeV4ExpertRestActionReference(
            $"rest-selection:{generation}:{selector.SelectionId}:{suffix}", action);
    }

    private bool DispatchSelectorAction(
        RuntimeV4ExpertRestOperation operation,
        RuntimeV4ExpertRestActionReference action,
        RuntimeV4ExpertRestHostProjection current)
    {
        RequireThread();
        RuntimeV4ExpertRestSelector? wire = current.Selector;
        if (wire is null || action.Action.SelectionId != wire.SelectionId
            || !_expertRestSelectors.TryGetValue(wire.SelectionId, out ExpertRestSelector? selector)
            || selector.ChildOperation is not null
            || !wire.LegalActions.Contains(action))
            return false;

        var pending = new ExpertRestPending(operation, action, current.Observation,
            selector.Button, selector.Option) { Selector = selector };
        _expertRestPending.Add(operation, pending);
        selector.ChildOperation = operation;
        try
        {
            pending.Work = action.Action.Kind switch
            {
                "select_card" => SelectSmithCardAsync(selector, action.Action.CardId!),
                "select_player" => SelectMendPlayerAsync(selector, action.Action.PlayerId!),
                "confirm_selection" => ConfirmSmithSelectionAsync(selector),
                "cancel_selection" => CancelSelectorAsync(selector),
                _ => throw new InvalidOperationException("unsupported rest selector action")
            };
            return true;
        }
        catch
        {
            selector.ChildOperation = null;
            _expertRestPending.Remove(operation);
            throw;
        }
    }

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
        targetManager.OnNodeHovered(matches[0]);
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
                    selector.SelectedPlayer = matches[0].Player;
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

    private static NCardHolder[] RewardCardsForRest(ExpertRestSelector selector) =>
        selector.SmithScreen is not { } screen || CurrentPlayer() is not { } player
            ? Array.Empty<NCardHolder>() : Descendants(screen)
            .OfType<NCardHolder>()
            .Where(holder => holder.IsVisibleInTree() && holder.CardModel is not null
                && player.Deck.Cards.Contains(holder.CardModel) && Clickable(holder.Hitbox))
            .ToArray();

    private static string RestPlayerId(Player player) => $"player:{player.NetId}";

    private static string RestPlayerLabel(Player player) =>
        ReadNestedIdentity(player, "Character", "Id") ?? RestPlayerId(player);

    private string RestSelectionFingerprint()
    {
        if (_expertRestSelectors.Count == 0) return "none";
        return string.Join("|", _expertRestSelectors.Values.OrderBy(selector => selector.SelectionId)
            .Select(selector => selector.SelectionId + ":" + selector.SelectionKind + ":"
                + selector.RequiredCount + ":" + string.Join(",", selector.SelectedCards.Select(CardId))
                + ":" + (selector.SelectedPlayer is null ? "" : RestPlayerId(selector.SelectedPlayer))));
    }

    private static bool Clickable(NBackButton control) =>
        GodotObject.IsInstanceValid(control) && control.IsVisibleInTree() && control.IsEnabled;
}
