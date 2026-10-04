// SPDX-License-Identifier: MIT

using System.Collections.Generic;
using MegaCrit.Sts2.Core.Entities.Cards;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Models;

namespace AiAscension.Sts2GameMod.Runtime;

// The compatibility probe supplies only helpers that are private to other producer partials;
// the player/card/relic/potion model APIs in LiveCombatSource.PlayerObservation.cs are the actual
// installed host assembly types.
internal sealed partial class LiveCombatSource
{
    private readonly Dictionary<CardModel, string> _cardIds = new();

    private ushort U16(int value) => (ushort)System.Math.Clamp(value, 0, ushort.MaxValue);

    private string CardId(CardModel card)
    {
        if (!_cardIds.TryGetValue(card, out string? id))
        {
            id = $"card:{_cardIds.Count + 1}";
            _cardIds.Add(card, id);
        }
        return id;
    }

    private static string PotionId(PotionModel potion, int slot) =>
        $"potion:{slot}:{potion.Id.Entry}";

    private static bool? PotionUsable(Player player, PotionModel potion, int slot)
    {
        _ = player;
        _ = potion;
        _ = slot;
        return null;
    }

    private static byte? MaxPotionCount(Player? player)
    {
        if (player is null) return null;
        try
        {
            int count = player.MaxPotionCount;
            return count is >= 0 and <= byte.MaxValue ? (byte)count : null;
        }
        catch (System.Exception)
        {
            return null;
        }
    }
}
