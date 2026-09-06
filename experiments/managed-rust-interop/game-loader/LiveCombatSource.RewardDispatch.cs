// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Nodes.Cards.Holders;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.Screens;
using MegaCrit.Sts2.Core.Nodes.Screens.CardSelection;
using MegaCrit.Sts2.Core.Nodes.Screens.Map;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private bool PrepareReward(LegalActionReference action, out Func<Task> invoke,
        out Func<bool> postcondition, out string effect)
    {
        invoke = () => Task.CompletedTask;
        postcondition = () => false;
        effect = "";
        Node? screen = RewardOverlay();
        if (action.Kind == "choose_reward" && screen is NRewardsScreen)
        {
            var matches = RewardButtons(screen).Where(button => RewardId(button) == action.Value
                && CanClaimReward(button)).ToArray();
            if (matches.Length != 1) return false;
            var button = matches[0];
            var reward = button.Reward;
            if (reward == null) return false;
            invoke = () => { button.ForceClick(); return Task.CompletedTask; };
            postcondition = () => reward.SuccessfullySelected
                || RewardOverlay() is NCardRewardSelectionScreen;
            effect = "reward_collected_or_selection_opened";
            return true;
        }
        if (action.Kind == "proceed" && screen is NRewardsScreen rewards)
        {
            var buttons = Descendants(screen).OfType<NProceedButton>().Where(Clickable).ToArray();
            if (buttons.Length != 1) return false;
            invoke = () => { buttons[0].ForceClick(); return Task.CompletedTask; };
            postcondition = () => NMapScreen.Instance is { IsOpen: true } map
                && map.IsVisibleInTree() && !Clickable(buttons[0]);
            effect = "reward_proceed_opened_map";
            return true;
        }
        if (action.Kind == "select_card" && screen is NCardRewardSelectionScreen)
        {
            var holder = RewardCards(screen).SingleOrDefault(card => RewardCardId(card) == action.Value);
            var player = CurrentPlayer();
            if (holder?.CardModel == null || player == null) return false;
            var card = holder.CardModel;
            invoke = () =>
            {
                holder.EmitSignal(NCardHolder.SignalName.Pressed, holder);
                return Task.CompletedTask;
            };
            postcondition = () => RewardOverlay() != screen && player.Deck.Cards.Contains(card);
            effect = "reward_card_added_to_deck";
            return true;
        }
        return false;
    }
}
