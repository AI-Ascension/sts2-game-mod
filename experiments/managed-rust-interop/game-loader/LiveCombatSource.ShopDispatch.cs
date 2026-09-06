// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Threading.Tasks;
using MegaCrit.Sts2.Core.Entities.Merchant;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Nodes.Screens.Map;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private bool PrepareShop(LegalActionReference action, out Func<Task> invoke,
        out Func<bool> postcondition, out string effect)
    {
        invoke = () => Task.CompletedTask;
        postcondition = () => false;
        effect = "";
        if (CurrentShop() is not { } room || RewardOverlay() != null
            || CurrentPlayer() is not { } player) return false;
        if (action.Kind == "proceed"
            && (room.Inventory.IsOpen ? ShopBack(room) != null : Clickable(room.ProceedButton)))
        {
            invoke = () => ProceedShopAsync(room);
            postcondition = () => NMapScreen.Instance is { IsOpen: true } map && map.IsVisibleInTree();
            effect = "shop_proceed_opened_map";
            return true;
        }
        if (action.Kind == "shop_remove")
        {
            var removal = ShopSlots(room).Select(slot => slot.Entry)
                .OfType<MerchantCardRemovalEntry>().SingleOrDefault(entry => !entry.Used && entry.EnoughGold);
            var card = player.Deck.Cards.SingleOrDefault(card => CardId(card) == action.Value && card.IsRemovable);
            if (removal == null || card == null) return false;
            invoke = () => RemoveShopCardAsync(room, removal, card);
            postcondition = () => removal.Used && !player.Deck.Cards.Contains(card);
            effect = "shop_card_removed";
            return true;
        }
        if (action.Kind != "shop_purchase") return false;
        var matches = ShopSlots(room).Where(slot => ShopEntryId(room, slot.Entry) == action.Value
            && slot.Entry.EnoughGold).ToArray();
        if (matches.Length != 1 || PurchaseOwnership(matches[0].Entry, player) is not { } owned) return false;
        var entry = matches[0].Entry;
        invoke = async () =>
        {
            if (!await entry.OnTryPurchaseWrapper(room.Inventory.Inventory))
                throw new InvalidOperationException("native shop purchase did not complete");
            RequireThread();
        };
        postcondition = owned;
        effect = "shop_item_obtained";
        return true;
    }

    private static Func<bool>? PurchaseOwnership(MerchantEntry entry, Player player)
    {
        if (entry is MerchantCardEntry { CreationResult: { } creation })
        {
            var previous = player.Deck.Cards.ToArray();
            var id = creation.Card.Id;
            return () => player.Deck.Cards.Any(owned => owned.Id == id && !previous.Contains(owned));
        }
        if (entry is MerchantRelicEntry { Model: { } relic })
        {
            var previous = player.Relics.ToArray();
            var id = relic.Id;
            return () => player.Relics.Any(owned => owned.Id == id && !previous.Contains(owned));
        }
        if (entry is MerchantPotionEntry { Model: { } potion } && player.PotionSlots.Any(slot => slot == null))
        {
            var previous = player.Potions.ToArray();
            var id = potion.Id;
            return () => player.Potions.Any(owned => owned.Id == id && !previous.Contains(owned));
        }
        return null;
    }
}
