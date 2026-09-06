// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Nodes.Cards;
using MegaCrit.Sts2.Core.Nodes.Cards.Holders;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.Screens.CardSelection;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    // The native deck selector renders selected cards with this typed control's width uniform.
    // Unknown controls or uniforms remain unavailable.
    private static float? EventSelectionWidth(NCardHolder holder)
    {
        if (!GodotObject.IsInstanceValid(holder)) return null;
        NCardHighlight[] highlights = Descendants(holder).OfType<NCardHighlight>().ToArray();
        if (highlights.Length != 1 || !highlights[0].IsVisibleInTree()
            || highlights[0].Material is not ShaderMaterial material) return null;
        Variant width = material.GetShaderParameter("width");
        return width.VariantType == Variant.Type.Float && float.IsFinite(width.AsSingle())
            ? width.AsSingle() : null;
    }

    private static NCardHolder[] AvailableRewardCards(Node screen) => RewardCards(screen)
        .Where(holder => screen is not NDeckCardSelectScreen || EventSelectionWidth(holder) == 0f)
        .ToArray();

    private bool PrepareEventRemoval(LegalActionReference action, out Func<Task> invoke,
        out Func<bool> postcondition, out string effect)
    {
        invoke = () => Task.CompletedTask;
        postcondition = () => false;
        effect = "";
        if (action.Kind != "select_card" || RewardOverlay() is not NDeckCardSelectScreen screen
            || EventChoiceParent() is not { } parent || CurrentPlayer() is not { } player) return false;
        NCardHolder? holder = AvailableRewardCards(screen).SingleOrDefault(card => RewardCardId(card) == action.Value);
        if (holder?.CardModel is not { } card) return false;
        var selected = RewardCards(screen).Where(candidate => EventSelectionWidth(candidate) > 0f)
            .Select(candidate => candidate.CardModel!).Append(card).ToArray();
        bool partial = false;
        invoke = async () =>
        {
            if (holder.EmitSignal(NCardHolder.SignalName.Pressed, holder) != Error.Ok)
                throw new InvalidOperationException("native event card selection failed");
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
                if (!confirmed && controls.Length == 0 && EventSelectionWidth(holder) > 0f)
                {
                    partial = true;
                    return;
                }
            }
            throw new InvalidOperationException("native event selection did not settle");
        };
        postcondition = () => partial
            ? RewardOverlay() == screen && EventSelectionWidth(holder) > 0f
                && player.Deck.Cards.Contains(card) && parent.Work is { IsCompleted: false }
            : RewardOverlay() != screen && selected.All(selectedCard => !player.Deck.Cards.Contains(selectedCard))
                && parent.Work?.IsCompletedSuccessfully == true && parent.Postcondition();
        effect = "event_card_selection_applied";
        return true;
    }

}
