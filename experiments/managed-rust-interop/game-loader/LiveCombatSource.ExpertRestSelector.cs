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
        internal (ushort Hp, ushort MaxHp)? MendTargetSnapshot { get; set; }
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

}
