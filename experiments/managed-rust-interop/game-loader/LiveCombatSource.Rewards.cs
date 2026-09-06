// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using MegaCrit.Sts2.Core.Nodes.Cards.Holders;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.GodotExtensions;
using MegaCrit.Sts2.Core.Nodes.Rewards;
using MegaCrit.Sts2.Core.Nodes.Screens;
using MegaCrit.Sts2.Core.Nodes.Screens.CardSelection;
using MegaCrit.Sts2.Core.Nodes.Screens.Overlays;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private string? _lastOverlayDiagnostic;
    private static Node? RewardOverlay()
    {
        Node? screen = NOverlayStack.Instance?.ScreenCount > 0
            ? NOverlayStack.Instance.Peek() as Node : null;
        return screen is CanvasItem canvas && canvas.IsVisibleInTree() ? screen : null;
    }

    private static bool Clickable(NClickableControl control) =>
        GodotObject.IsInstanceValid(control) && control.IsVisibleInTree() && control.IsEnabled;

    private static NRewardButton[] RewardButtons(Node screen) => Descendants(screen)
        .OfType<NRewardButton>().Where(button => Clickable(button)
            && button.Reward is { SuccessfullySelected: false }).ToArray();

    private static string RewardId(NRewardButton button) =>
        button.Reward is { } reward ? $"reward:{reward.RewardsSetIndex}:{reward.GetType().Name}"
            : throw new InvalidOperationException("reward control is unbound");

    private static NCardHolder[] RewardCards(Node screen) => Descendants(screen)
        .OfType<NCardHolder>().Where(holder => holder.IsVisibleInTree()
            && holder.CardModel != null && Clickable(holder.Hitbox)).ToArray();

    private string RewardCardId(NCardHolder holder) => RuntimeV3GameplayChoiceIdentity.Card(
        CardId(holder.CardModel!), holder.CardModel!.Title);

    private RuntimeV3GameplayObservation ProjectRewardOverlay(RuntimeV3GameplayObservation observation)
    {
        Node? screen = RewardOverlay();
        bool modal = MegaCrit.Sts2.Core.Nodes.CommonUi.NModalContainer.Instance?.OpenModal != null;
        if (screen is NRewardsScreen)
            return Surface(observation, RuntimeV3GameplayState.Reward,
                RewardButtons(screen).Select(RewardId).GroupBy(value => value)
                    .Where(group => group.Count() == 1).Select(group => group.Key).ToArray(), !modal);
        if (screen is NCardRewardSelectionScreen)
            return Surface(observation, RuntimeV3GameplayState.Selection,
                RewardCards(screen).Select(RewardCardId).ToArray(), !modal);
        return observation with { IsActionable = false, InputEnabled = false, ModalBlocking = true };
    }

    private void TraceCampaignSurface(Node? screen)
    {
        string diagnostic = $"overlay={screen?.GetType().Name ?? "none"}; "
            + $"map_open={MegaCrit.Sts2.Core.Nodes.Screens.Map.NMapScreen.Instance?.IsOpen}; "
            + $"map_visible={MegaCrit.Sts2.Core.Nodes.Screens.Map.NMapScreen.Instance?.IsVisibleInTree()}";
        if (_lastOverlayDiagnostic != diagnostic)
        {
            _lastOverlayDiagnostic = diagnostic;
            GD.Print("[AI-ASCENSION LIVE] campaign surface " + diagnostic);
        }
    }

    private LegalActionReference[] RewardActions(RuntimeV3GameplayObservation observation)
    {
        Node? screen = RewardOverlay();
        var actions = new List<LegalActionReference>();
        string? kind = screen is NRewardsScreen ? "choose_reward"
            : screen is NCardRewardSelectionScreen ? "select_card" : null;
        if (kind == null) return Array.Empty<LegalActionReference>();
        IEnumerable<string> values = screen is NCardRewardSelectionScreen
            ? RewardCards(screen).Select(RewardCardId) : observation.StateValues;
        foreach (string value in values)
            actions.Add(new($"{kind}:{observation.Generation}:{value}", kind, value, null,
                observation.Generation));
        if (screen is NRewardsScreen && Descendants(screen).OfType<NProceedButton>()
            .Count(Clickable) == 1)
            actions.Add(new($"proceed:{observation.Generation}", "proceed", null, null,
                observation.Generation));
        return actions.ToArray();
    }
}
