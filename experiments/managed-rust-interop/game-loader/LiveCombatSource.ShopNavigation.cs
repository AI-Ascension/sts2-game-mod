// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.Rooms;
using MegaCrit.Sts2.Core.Rooms;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private static NMerchantRoom? CurrentShop() =>
        RunManager.Instance.DebugOnlyGetState()?.CurrentRoom is MerchantRoom
        && NMerchantRoom.Instance is { } room && GodotObject.IsInstanceValid(room)
        && room.IsVisibleInTree() ? room : null;

    internal static async Task OpenEnteredShopAsync()
    {
        int thread = System.Environment.CurrentManagedThreadId;
        if (RunManager.Instance.DebugOnlyGetState()?.CurrentRoom is not MerchantRoom) return;
        bool invoked = false;
        for (int frame = 0; frame < 600; frame++)
        {
            if (System.Environment.CurrentManagedThreadId != thread)
                throw new InvalidOperationException("merchant navigation left the host thread");
            if (CurrentShop() is { } room)
            {
                if (room.Inventory.IsOpen && room.Inventory.IsVisibleInTree()) return;
                if (!invoked && Clickable(room.MerchantButton))
                {
                    room.MerchantButton.ForceClick();
                    invoked = true;
                }
            }
            await WaitCampaignFrameAsync();
        }
        throw new InvalidOperationException("native merchant inventory did not open");
    }

    private static NBackButton? ShopBack(NMerchantRoom room)
    {
        var controls = Descendants(room.Inventory).OfType<NBackButton>().Where(Clickable).ToArray();
        return controls.Length == 1 ? controls[0] : null;
    }

    private async Task ProceedShopAsync(NMerchantRoom room)
    {
        if (room.Inventory.IsOpen)
        {
            var back = ShopBack(room) ?? throw new InvalidOperationException("shop back control unavailable");
            back.ForceClick();
        }
        for (int frame = 0; frame < 600; frame++)
        {
            RequireThread();
            if (!room.Inventory.IsOpen && Clickable(room.ProceedButton))
            {
                room.ProceedButton.ForceClick();
                return;
            }
            await WaitCampaignFrameAsync();
        }
        throw new InvalidOperationException("shop proceed control did not become available");
    }
}
