// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Entities.Cards;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Models;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private RuntimeV3GameplayPlayer ProjectPlayer(Player? player, PlayerCombatState? combat) => new(
        U16(player?.Creature.CurrentHp ?? 0), U16(player?.Creature.MaxHp ?? 0),
        (byte)Math.Clamp(combat?.Energy ?? 0, 0, 255), (uint)Math.Max(player?.Gold ?? 0, 0),
        ProjectCards(combat?.Hand.Cards), ProjectCards(player?.Deck.Cards),
        ProjectCards(combat?.DiscardPile.Cards), ProjectCards(combat?.ExhaustPile.Cards),
        ProjectRelics(player), ProjectPotions(player), PotionSlotCapacity(player), MaxPotionCount(player));

    private RuntimeV3GameplayCard[] ProjectCards(IEnumerable<CardModel>? cards) =>
        cards?.Take(RuntimeV3GameplayContract.MaxEntities + 1)
            .Select(card => new RuntimeV3GameplayCard(
                CardId(card), card.Title,
                (byte)Math.Clamp(card.EnergyCost.GetResolved(), 0, byte.MaxValue),
                card.IsUpgraded,
                PublicText(card, "Description")))
            .ToArray()
        ?? Array.Empty<RuntimeV3GameplayCard>();

    private static RuntimeV3GameplayRelic[]? ProjectRelics(Player? player)
    {
        if (player is null) return null;
        try
        {
            var result = new List<RuntimeV3GameplayRelic>();
            var identities = new HashSet<string>(StringComparer.Ordinal);
            foreach (RelicModel relic in player.Relics)
            {
                string relicId = relic.Id.Entry;
                string? name = PublicText(relic, "Title");
                if (!RuntimeV3GameplayContract.IsIdentity(relicId) || !identities.Add(relicId)
                    || name is null
                    || !RuntimeV3GameplayContract.IsPublishableText(name)
                    || result.Count >= RuntimeV3GameplayContract.MaxEntities)
                    return null;

                result.Add(new RuntimeV3GameplayRelic(
                    relicId, name, PublicText(relic, "DynamicDescription")));
            }
            return result.ToArray();
        }
        catch (Exception)
        {
            // A host transition or unavailable model field means this inventory is unavailable.
            return null;
        }
    }

    private static RuntimeV3GameplayPotion[]? ProjectPotions(Player? player)
    {
        if (player is null) return null;
        try
        {
            var result = new List<RuntimeV3GameplayPotion>();
            var slots = new HashSet<int>();
            foreach (PotionModel potion in player.Potions)
            {
                int slot = player.GetPotionSlotIndex(potion);
                if (slot < 0 || slot > byte.MaxValue || !slots.Add(slot)
                    || result.Count >= RuntimeV3GameplayContract.MaxEntities)
                    return null;

                string? name = PublicText(potion, "Title");
                string potionId = PotionId(potion, slot);
                if (name is null || !RuntimeV3GameplayContract.IsPublishableText(name)
                    || !RuntimeV3GameplayContract.IsIdentity(potionId))
                    return null;

                string? hostTargetType = PublicText(potion, "TargetType");
                string? targetMode = hostTargetType switch
                {
                    "Self" => "self",
                    "AnyEnemy" => "any_enemy",
                    "AllEnemies" => "all_enemies",
                    "None" => "none",
                    null => null,
                    _ => "unknown"
                };
                result.Add(new RuntimeV3GameplayPotion(
                    potionId,
                    name,
                    (byte)slot,
                    PotionUsable(player, potion, slot),
                    targetMode,
                    PublicText(potion, "DynamicDescription")));
            }
            return result.ToArray();
        }
        catch (Exception)
        {
            // Never emit a partially enumerated inventory as if it were complete.
            return null;
        }
    }

    private static byte? PotionSlotCapacity(Player? player)
    {
        if (player is null) return null;
        try
        {
            // PotionSlots is the host's full belt array, including empty slots. The contract field
            // is capacity, not the occupied-potion count.
            int count = player.PotionSlots.Count;
            return count is >= 0 and <= byte.MaxValue ? (byte)count : null;
        }
        catch (Exception)
        {
            return null;
        }
    }
}
