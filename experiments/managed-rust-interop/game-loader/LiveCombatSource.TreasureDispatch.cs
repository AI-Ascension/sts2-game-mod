// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Threading.Tasks;
using MegaCrit.Sts2.Core.Nodes.Screens;
using MegaCrit.Sts2.Core.Nodes.Screens.Map;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private static bool PrepareTreasure(LegalActionReference action, out Func<Task> invoke,
        out Func<bool> postcondition, out string effect)
    {
        invoke = () => Task.CompletedTask;
        postcondition = () => false;
        effect = "";
        if (CurrentTreasure() is not { } room || RewardOverlay() != null
            || CurrentPlayer() is not { } player) return false;
        if (action.Kind == "choose_reward" && action.Value == TreasureChestId
            && TreasureChest(room) is { } chest)
        {
            invoke = () => { chest.ForceClick(); return Task.CompletedTask; };
            postcondition = () => !Clickable(chest) && (TreasureRelics(room).Length > 0
                || RewardOverlay() is NRewardsScreen || Clickable(room.ProceedButton));
            effect = "treasure_chest_opened";
            return true;
        }
        if (action.Kind == "choose_reward")
        {
            var matches = TreasureRelics(room).Where(holder => TreasureRelicId(holder) == action.Value).ToArray();
            if (matches.Length != 1) return false;
            var holder = matches[0];
            var relic = holder.Relic.Model;
            // Native inventory acquisition can clone the displayed model. Retain its
            // host identity and require a newly owned instance, not reference equality
            // with the display model or an unrelated inventory change.
            var previouslyOwned = player.Relics.ToArray();
            invoke = () => { holder.ForceClick(); return Task.CompletedTask; };
            postcondition = () => !Clickable(holder) && player.Relics.Any(owned =>
                owned.Id == relic.Id && !previouslyOwned.Contains(owned));
            effect = "treasure_relic_obtained";
            return true;
        }
        if (action.Kind == "proceed" && Clickable(room.ProceedButton))
        {
            invoke = () => { room.ProceedButton.ForceClick(); return Task.CompletedTask; };
            postcondition = () => NMapScreen.Instance is { IsOpen: true } map && map.IsVisibleInTree();
            effect = "treasure_proceed_opened_map";
            return true;
        }
        return false;
    }
}
