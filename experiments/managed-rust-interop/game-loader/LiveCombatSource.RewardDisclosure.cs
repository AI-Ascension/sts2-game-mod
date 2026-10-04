// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Nodes.Rewards;
using MegaCrit.Sts2.Core.Rewards;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    /// <summary>
    /// Describes the rewards on a rewards screen, disclosing what taking each one would present.
    ///
    /// <para>
    /// This is the set that carried the livelock in <c>sts2-game-mod#171</c>: it was the only
    /// offered set a model had to rank blind, and a model told nothing but
    /// <c>reward:5:CardReward</c> cannot rank it, so it skips. A card reward therefore discloses
    /// the cards it holds as <c>contents</c>, and the decision to open it can be made knowing what
    /// is inside.
    /// </para>
    ///
    /// <para>
    /// The cards come from <c>CardReward.Cards</c>, a public <c>IEnumerable&lt;CardModel&gt;</c> the
    /// host already holds and already shows on the very next overlay. Reading it is not a
    /// content-catalog read and not a peek at an ungenerated reward: it is the same list the player
    /// is being shown. A reward whose pending choice the host does not expose here discloses nothing
    /// and stays the bare identity, which is what the harness already admits. No probability,
    /// weight, reroll rule, or unrevealed outcome is read or inferred; the private
    /// <c>Options</c>/<c>RerollOptions</c> and <c>CanReroll</c> are deliberately left alone.
    /// </para>
    ///
    /// <para>
    /// Entries are filtered by the same rule <see cref="RewardActions"/> applies to the legal
    /// catalog: an identity more than one button claims is dropped rather than published twice. The
    /// offered set and the action catalog must agree on which offers exist — a described option the
    /// catalog holds no action for is a decision the model cannot act on, which is the blind-choice
    /// problem this change exists to remove.
    /// </para>
    /// </summary>
    private static RuntimeV3GameplayOfferedSet RewardSet(Node screen)
    {
        NRewardButton[] buttons = RewardButtons(screen);
        var pairs = new (string Identity, NRewardButton Button)[buttons.Length];
        for (int index = 0; index < buttons.Length; index++)
        {
            pairs[index] = (RewardId(buttons[index]), buttons[index]);
        }
        var entries = new List<RuntimeV3GameplayOfferedEntry>(pairs.Length);
        foreach (IGrouping<string, (string Identity, NRewardButton Button)> group in pairs
            .GroupBy(pair => pair.Identity, StringComparer.Ordinal))
        {
            if (group.Count() != 1) continue;
            entries.Add(new RuntimeV3GameplayOfferedEntry(group.Key,
                Contents: RewardContents(group.Single().Button)));
        }
        return new RuntimeV3GameplayOfferedSet(entries);
    }

    /// <summary>
    /// Discloses what taking one reward would present, for the reward kinds whose pending choice the
    /// host already holds here, and discloses nothing for the kinds it does not.
    /// </summary>
    private static List<RuntimeV3GameplayOfferedContent>? RewardContents(NRewardButton button)
    {
        if (button.Reward is not CardReward cardReward) return null;
        var contents = new List<RuntimeV3GameplayOfferedContent>();
        int index = 0;
        foreach (CardModel card in cardReward.Cards)
        {
            contents.Add(RewardCardContent(card, index++));
        }
        return contents.Count > 0 ? contents : null;
    }

    /// <summary>
    /// Describes one disclosed card, reading only what the host card itself carries.
    ///
    /// <para>
    /// This deliberately does not reuse <see cref="RewardCardEntry"/>'s identity. That identity is
    /// the <c>select_card</c> action identity minted on the *next* overlay from the per-instance
    /// <c>_cardIds</c> mapping; publishing it here would couple two screens' catalogs and would
    /// offer an identity the model cannot select yet. A disclosed card therefore carries the host's
    /// own stable card id and is documentation of the offer, not a new action.
    /// </para>
    /// </summary>
    private static RuntimeV3GameplayOfferedContent RewardCardContent(CardModel card, int index)
    {
        return RuntimeV3GameplayOfferedContent.FromCard(
            DisclosedCardId(card, index),
            card.Title,
            card.EnergyCost.GetResolved(),
            card.IsUpgraded,
            PublicText(card, "Description"),
            PublicText(card, "Rarity"));
    }

    /// <summary>
    /// A disclosed card's identity, built from the host's own stable card id.
    ///
    /// <para>
    /// <c>Card.Id.Entry</c> is the host card identity this loader already publishes for live cards
    /// in <c>LiveCombatSource.LiveCards.cs</c>. It is sanitised through the same identity builder
    /// the offered set uses, so an unusual card id cannot emit an identity the harness refuses. When
    /// the host id cannot be sanitised at all, the disclosure falls back to the card's own position
    /// in the host's list rather than dropping the card or inventing an id; the fallback is
    /// positional rather than a fixed string because two cards sharing one identity inside a
    /// disclosed set would be a duplicate.
    /// </para>
    /// </summary>
    private static string DisclosedCardId(CardModel card, int index)
    {
        string entry = card.Id.Entry;
        if (RuntimeV3GameplayContract.IsIdentity(entry))
        {
            return RuntimeV3GameplayChoiceIdentity.Card(entry, card.Title);
        }
        return "disclosed:" + index;
    }
}
