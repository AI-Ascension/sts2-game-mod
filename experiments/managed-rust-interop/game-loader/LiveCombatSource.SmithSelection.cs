// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.Cards.Holders;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.RestSite;
using MegaCrit.Sts2.Core.Nodes.Screens.CardSelection;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private async Task SmithCardAsync(NRestSiteButton button, CardModel card, int previousLevel)
    {
        button.ForceClick();
        NDeckUpgradeSelectScreen? selectedScreen = null;
        for (int frame = 0; frame < 600; frame++)
        {
            RequireThread();
            if (RewardOverlay() is NDeckUpgradeSelectScreen screen)
            {
                NCardHolder? holder = RewardCards(screen)
                    .SingleOrDefault(holder => ReferenceEquals(holder.CardModel, card));
                if (holder != null)
                {
                    selectedScreen = screen;
                    if (holder.EmitSignal(NCardHolder.SignalName.Pressed, holder) != Error.Ok)
                        throw new InvalidOperationException("native smith card selection failed");
                    break;
                }
            }
            await WaitCampaignFrameAsync();
        }
        if (selectedScreen == null) throw new InvalidOperationException("smith selection did not expose the chosen card");
        await ConfirmSmithAsync(selectedScreen, card, previousLevel);
    }

    private async Task ConfirmSmithAsync(NDeckUpgradeSelectScreen screen, CardModel card, int previousLevel)
    {
        for (int frame = 0; frame < 600; frame++)
        {
            await WaitCampaignFrameAsync();
            RequireThread();
            if (card.CurrentUpgradeLevel > previousLevel) return;
            // The host can close the screen before its queued upgrade is applied. The
            // retained card's upgrade-level postcondition must still establish completion.
            if (RewardOverlay() != screen) return;
            NConfirmButton[] controls = Descendants(screen).OfType<NConfirmButton>().Where(Clickable).ToArray();
            if (controls.Length == 1)
            {
                controls[0].ForceClick();
                return;
            }
        }
        throw new InvalidOperationException("smith confirmation did not become available");
    }

    private static async Task WaitCampaignFrameAsync()
    {
        NGame game = NGame.Instance ?? throw new InvalidOperationException("host scene is unavailable");
        await game.ToSignal(game.GetTree(), SceneTree.SignalName.ProcessFrame);
    }
}
