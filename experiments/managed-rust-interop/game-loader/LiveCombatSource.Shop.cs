// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using MegaCrit.Sts2.Core.Entities.Merchant;
using MegaCrit.Sts2.Core.Nodes.Rooms;
using MegaCrit.Sts2.Core.Nodes.Screens.Shops;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private static NMerchantSlot[] ShopSlots(NMerchantRoom room) => room.Inventory.IsOpen
        && room.Inventory.IsVisibleInTree() && room.Inventory.Inventory != null ? room.Inventory.GetAllSlots()
            .Where(slot => slot.IsVisibleInTree() && Clickable(slot.Hitbox)
                && slot.Entry is { IsStocked: true } && SupportedShopEntry(slot.Entry)).ToArray()
            : Array.Empty<NMerchantSlot>();

    private static string ShopEntryId(NMerchantRoom room, MerchantEntry entry) =>
        $"shop:{Array.IndexOf((room.Inventory.Inventory ?? throw new InvalidOperationException("shop inventory unavailable"))
            .AllEntries.ToArray(), entry)}:{entry.GetType().Name}";

    private static string ShopEntryName(MerchantEntry entry) => entry switch
    {
        MerchantCardEntry { CreationResult: { } creation } => creation.Card.Title,
        MerchantRelicEntry { Model: { } relic } => relic.Title.GetFormattedText(),
        MerchantPotionEntry { Model: { } potion } => potion.Title.GetFormattedText(),
        MerchantCardRemovalEntry => "Remove a card",
        _ => throw new InvalidOperationException("unsupported merchant entry")
    };

    private static bool SupportedShopEntry(MerchantEntry entry) => entry is MerchantCardEntry { CreationResult: not null }
        or MerchantRelicEntry { Model: not null } or MerchantPotionEntry { Model: not null } or MerchantCardRemovalEntry;

    private static RuntimeV3GameplayObservation ProjectShop(RuntimeV3GameplayObservation observation,
        NMerchantRoom room) => Surface(observation, RuntimeV3GameplayState.Shop, Array.Empty<string>(),
            MegaCrit.Sts2.Core.Nodes.CommonUi.NModalContainer.Instance?.OpenModal == null) with
        {
            ShopItems = ShopSlots(room).Where(slot => SupportedShopEntry(slot.Entry) && slot.Entry.Cost >= 0)
                .Select(slot => new RuntimeV3GameplayShopItem(ShopEntryId(room, slot.Entry),
                    ShopEntryName(slot.Entry), checked((uint)slot.Entry.Cost))).ToArray()
        };

    private LegalActionReference[] ShopActions(RuntimeV3GameplayObservation observation, NMerchantRoom room)
    {
        var actions = new List<LegalActionReference>();
        foreach (var slot in ShopSlots(room).Where(slot => slot.Entry.EnoughGold && slot.Entry.Cost >= 0))
        {
            if (slot.Entry is MerchantCardRemovalEntry { Used: false } && CurrentPlayer() is { } player)
            {
                foreach (var card in player.Deck.Cards.Where(card => card.IsRemovable))
                {
                    string id = CardId(card);
                    actions.Add(new($"shop_remove:{observation.Generation}:{id}", "shop_remove", id,
                        null, observation.Generation));
                }
            }
            else if (slot.Entry is MerchantCardEntry or MerchantRelicEntry
                || slot.Entry is MerchantPotionEntry && CurrentPlayer()?.PotionSlots.Any(potion => potion == null) == true)
            {
                string id = ShopEntryId(room, slot.Entry);
                actions.Add(new($"shop_purchase:{observation.Generation}:{id}", "shop_purchase", id,
                    null, observation.Generation));
            }
        }
        if (room.Inventory.IsOpen ? ShopBack(room) != null : Clickable(room.ProceedButton))
            actions.Add(new($"proceed:{observation.Generation}", "proceed", null, null, observation.Generation));
        return actions.ToArray();
    }
}
