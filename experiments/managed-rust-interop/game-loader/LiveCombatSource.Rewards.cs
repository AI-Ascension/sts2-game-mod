// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Nodes.Cards.Holders;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.GodotExtensions;
using MegaCrit.Sts2.Core.Nodes.Rewards;
using MegaCrit.Sts2.Core.Nodes.Screens;
using MegaCrit.Sts2.Core.Nodes.Screens.CardSelection;
using MegaCrit.Sts2.Core.Nodes.Screens.Overlays;
using MegaCrit.Sts2.Core.Rewards;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private string? _lastOverlayDiagnostic;
    private static Node? RewardOverlay()
    {
        Node? screen = NOverlayStack.Instance?.ScreenCount > 0
            ? NOverlayStack.Instance.Peek() as Node : null;
        if (screen is CanvasItem canvas && canvas.IsVisibleInTree()) return screen;
        // Some native event selectors are attached outside the overlay stack. Admit only
        // one visible typed selector; an ambiguous scene remains unavailable.
        if (MegaCrit.Sts2.Core.Nodes.NGame.Instance is not { } game) return null;
        NCardGridSelectionScreen[] selectors = Descendants(game).OfType<NCardGridSelectionScreen>()
            .Where(candidate => GodotObject.IsInstanceValid(candidate) && candidate.IsVisibleInTree()).ToArray();
        return selectors.Length == 1 ? selectors[0] : null;
    }

    private static bool Clickable(NClickableControl control) =>
        GodotObject.IsInstanceValid(control) && control.IsVisibleInTree() && control.IsEnabled;

    private static NRewardButton[] RewardButtons(Node screen) => Descendants(screen)
        .OfType<NRewardButton>().Where(button => Clickable(button)
            && button.Reward is { SuccessfullySelected: false }).ToArray();

    private static string RewardId(NRewardButton button) =>
        button.Reward is { } reward ? $"reward:{reward.RewardsSetIndex}:{reward.GetType().Name}"
            : throw new InvalidOperationException("reward control is unbound");

    private static bool CanClaimReward(NRewardButton button) => button.Reward is { } reward
        && (reward is not PotionReward || reward.Player.PotionSlots.Any(slot => slot == null));

    private static NCardHolder[] RewardCards(Node screen) => Descendants(screen)
        .OfType<NCardHolder>().Where(holder => holder.IsVisibleInTree()
            && holder.CardModel != null && Clickable(holder.Hitbox)).ToArray();

    private string RewardCardId(NCardHolder holder) => RuntimeV3GameplayChoiceIdentity.Card(
        CardId(holder.CardModel!), holder.CardModel!.Title);

    /// <summary>
    /// Describes one offered card from what the host card itself exposes.
    ///
    /// <para>
    /// Every attribute read here is one this loader already reads off the same <c>CardModel</c>
    /// elsewhere: title and the upgraded flag in <c>LiveCombatSource.LiveCards.cs</c>, and rarity
    /// and rules text through <c>PublicText</c> in
    /// <c>LiveCombatSource.ExpertProfileReflection.cs</c>. That is what makes these host-supplied
    /// rather than synthesized: each is a value the host object carries, read by a reader the
    /// shipped composition already owns, and it is published only when the read returns a value.
    /// A <c>null</c> read means the host did not supply it and the attribute is omitted — never
    /// filled with a plausible value, because a defaulted rarity would read downstream as an
    /// observed fact about this run.
    /// </para>
    ///
    /// <para>
    /// The <c>choice_id</c> is the same identity <see cref="RewardCardId"/> produces, so a provider
    /// can act on this described entry exactly as it acted on the bare identifier, and the dispatch
    /// matchers that compare against <c>RewardCardId</c> keep working unchanged.
    /// </para>
    ///
    /// <para>
    /// This is an instance method, not a static one: <c>CardId</c> assigns each card a stable
    /// per-instance identity from the <c>_cardIds</c> mapping. Reading it through
    /// <see cref="RewardCardId"/> is what guarantees the published <c>choice_id</c> is byte-for-byte
    /// the identity the action catalog and the dispatch matchers already use.
    /// </para>
    /// </summary>
    private RuntimeV3GameplayOfferedEntry RewardCardEntry(NCardHolder holder)
    {
        CardModel card = holder.CardModel!;
        return RuntimeV3GameplayOfferedEntry.FromCard(
            RewardCardId(holder),
            card.Title,
            card.EnergyCost.GetResolved(),
            card.IsUpgraded,
            PublicText(card, "Description"),
            PublicText(card, "Rarity"));
    }

    /// <summary>Describes every available reward card on a selection surface.</summary>
    private RuntimeV3GameplayOfferedSet RewardCardSet(Node screen)
    {
        NCardHolder[] holders = AvailableRewardCards(screen);
        var entries = new List<RuntimeV3GameplayOfferedEntry>(holders.Length);
        foreach (NCardHolder holder in holders)
        {
            entries.Add(RewardCardEntry(holder));
        }
        return new RuntimeV3GameplayOfferedSet(entries);
    }

    private RuntimeV3GameplayObservation ProjectRewardOverlay(RuntimeV3GameplayObservation observation)
    {
        Node? screen = RewardOverlay();
        bool modal = MegaCrit.Sts2.Core.Nodes.CommonUi.NModalContainer.Instance?.OpenModal != null;
        if (screen is NRewardsScreen)
            // Rewards stay identity-only. `NRewardButton` exposes `RewardsSetIndex` and the reward
            // type name to this loader, and nothing else player-visible: no title, no rarity, no
            // description. The game does model those on `Reward` subtypes, but reaching them would
            // mean reading the static content catalog (see NativeContentCatalogSemantic.cs), which
            // is reflection over the content manifest and is **not** attached to the shipped host
            // composition. Emitting from a table nothing in this composition reads would be a
            // source-only catalog dressed as a producer, and a rarity synthesized from it would
            // read downstream as an observed fact about this run. So the entry stays bare, which
            // is exactly what the harness already admits.
            return Surface(observation, RuntimeV3GameplayState.Reward,
                RewardButtons(screen).Select(RewardId).GroupBy(value => value)
                    .Where(group => group.Count() == 1).Select(group => group.Key).ToArray(), !modal);
        if (screen != null && (screen is NCardRewardSelectionScreen || HasCombatChoice(screen)
            || HasEventCardChoice(screen)))
            return Surface(observation, RuntimeV3GameplayState.Selection,
                RewardCardSet(screen), !modal);
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
        if (screen == null && SelectingHand() is { } hand)
            return HandChoiceActions(observation, hand);
        var actions = new List<LegalActionReference>();
        string? kind = screen is NRewardsScreen ? "choose_reward"
            : screen is NCardRewardSelectionScreen || HasCombatChoice(screen)
                || HasEventCardChoice(screen) ? "select_card" : null;
        if (kind == null || screen == null) return Array.Empty<LegalActionReference>();
        IEnumerable<string> values = kind == "select_card"
            ? AvailableRewardCards(screen).Select(RewardCardId)
            : RewardButtons(screen).Where(CanClaimReward).Select(RewardId)
                .GroupBy(value => value).Where(group => group.Count() == 1).Select(group => group.Key);
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
