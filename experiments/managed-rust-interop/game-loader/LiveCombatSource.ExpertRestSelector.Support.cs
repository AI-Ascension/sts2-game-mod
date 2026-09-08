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
