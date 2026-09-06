// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Nodes.Cards.Holders;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.Screens.CardSelection;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private bool PrepareEventEnchantment(LegalActionReference action, out Func<Task> invoke,
        out Func<bool> postcondition, out string effect)
    {
        invoke = () => Task.CompletedTask;
        postcondition = () => false;
        effect = "";
        if (action.Kind != "select_card" || RewardOverlay() is not NDeckEnchantSelectScreen screen
            || EventChoiceParent() is not { } parent || CurrentPlayer() is not { } player) return false;
        NCardHolder? holder = RewardCards(screen).SingleOrDefault(card => RewardCardId(card) == action.Value);
        if (holder?.CardModel is not { } card || !player.Deck.Cards.Contains(card)) return false;
        var before = card.Enchantment;
        int beforeAmount = before?.Amount ?? 0;
        invoke = async () =>
        {
            if (holder.EmitSignal(NCardHolder.SignalName.Pressed, holder) != Error.Ok)
                throw new InvalidOperationException("native event enchantment selection failed");
            bool confirmed = false;
            for (int frame = 0; frame < 600; frame++)
            {
                await WaitCampaignFrameAsync();
                RequireThread();
                if (RewardOverlay() != screen) return;
                NConfirmButton[] controls = Descendants(screen).OfType<NConfirmButton>().Where(Clickable).ToArray();
                if (!confirmed && controls.Length == 1)
                {
                    controls[0].ForceClick();
                    confirmed = true;
                }
            }
            throw new InvalidOperationException("native event enchantment did not close");
        };
        postcondition = () => RewardOverlay() != screen && player.Deck.Cards.Contains(card)
            && card.Enchantment is { } after && (after != before || after.Amount != beforeAmount)
            && parent.Work?.IsCompletedSuccessfully == true && parent.Postcondition();
        effect = "event_card_enchantment_completed";
        return true;
    }
}
