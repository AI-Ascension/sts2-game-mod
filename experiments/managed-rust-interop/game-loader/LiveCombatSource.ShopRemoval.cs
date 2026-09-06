// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Entities.Merchant;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Nodes.Cards.Holders;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.Rooms;
using MegaCrit.Sts2.Core.Nodes.Screens.CardSelection;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private async Task RemoveShopCardAsync(NMerchantRoom room, MerchantCardRemovalEntry removal, CardModel card)
    {
        Task<bool> purchase = removal.OnTryPurchaseWrapper(room.Inventory.Inventory);
        bool selected = false;
        bool confirmed = false;
        for (int frame = 0; frame < 600; frame++)
        {
            RequireThread();
            if (purchase.IsCompleted)
            {
                if (!await purchase) throw new InvalidOperationException("native card removal did not complete");
                return;
            }
            if (RewardOverlay() is NDeckCardSelectScreen screen)
            {
                if (!selected)
                {
                    var holder = RewardCards(screen).SingleOrDefault(holder => ReferenceEquals(holder.CardModel, card));
                    if (holder != null)
                    {
                        if (holder.EmitSignal(NCardHolder.SignalName.Pressed, holder) != Error.Ok)
                            throw new InvalidOperationException("native removal card selection failed");
                        selected = true;
                    }
                }
                else if (!confirmed)
                {
                    var controls = Descendants(screen).OfType<NConfirmButton>().Where(Clickable).ToArray();
                    if (controls.Length == 1)
                    {
                        controls[0].ForceClick();
                        confirmed = true;
                    }
                }
            }
            await WaitCampaignFrameAsync();
        }
        throw new InvalidOperationException("native card removal exceeded its frame bound");
    }
}
