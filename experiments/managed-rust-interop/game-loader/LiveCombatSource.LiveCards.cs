// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Entities.Cards;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private readonly LiveCardSnapshotRegistry _liveCardRegistry = new();

    /// <summary>
    /// Copies the supported live-card surface on the same host thread as gameplay observation.
    /// The managed DTO contains no CardModel references after this method returns.
    /// </summary>
    internal LiveCardCapturedSnapshot CaptureLiveCardSnapshot(
        string instanceId,
        string contentManifest)
    {
        RequireThread();
        try
        {
            RunManager manager = RunManager.Instance;
            if (!manager.IsInProgress || manager.DebugOnlyGetState() is not { } run)
            {
                _liveCardRegistry.Invalidate();
                return LiveCardCapturedSnapshot.Unavailable("run_not_active");
            }

            Player? player = CurrentPlayer();
            if (player is null)
            {
                _liveCardRegistry.Invalidate();
                return LiveCardCapturedSnapshot.Unavailable("player_source_unavailable");
            }

            ulong stateGeneration = Observe().Generation;
            var cards = new List<LiveCardCaptureInput>();
            if (player.PlayerCombatState is { } combat)
            {
                if (!AddCards(cards, combat.Hand.Cards, LiveCardZone.Hand, player,
                        contentManifest)
                    || !AddCards(cards, combat.DiscardPile.Cards, LiveCardZone.Discard, player,
                        contentManifest)
                    || !AddCards(cards, combat.ExhaustPile.Cards, LiveCardZone.Exhaust, player,
                        contentManifest))
                    return LiveCardCapturedSnapshot.Unavailable("card_count_exceeded");
            }
            if (!AddCards(cards, player.Deck.Cards, LiveCardZone.Deck, player, contentManifest))
                return LiveCardCapturedSnapshot.Unavailable("card_count_exceeded");

            string runKey = $"{run.GameMode}|{run.Rng.StringSeed}|"
                + $"{player.NetId.ToString(CultureInfo.InvariantCulture)}";
            return _liveCardRegistry.Capture(
                instanceId, run, runKey, contentManifest, stateGeneration, cards);
        }
        catch (Exception)
        {
            _liveCardRegistry.Invalidate();
            return LiveCardCapturedSnapshot.Unavailable("host_snapshot_unavailable");
        }
    }

    /// <summary>
    /// Restore/session owners call this on the host thread before exposing a new live snapshot.
    /// No restore implementation is implied by this invalidation hook.
    /// </summary>
    internal void InvalidateLiveCardSnapshot()
    {
        RequireThread();
        _liveCardRegistry.Invalidate();
        _liveCardBinding = null;
    }

    private static bool AddCards(
        ICollection<LiveCardCaptureInput> output,
        IEnumerable<CardModel> cards,
        LiveCardZone zone,
        Player owner,
        string contentManifest)
    {
        int position = 0;
        foreach (CardModel card in cards)
        {
            if (output.Count >= LiveCardSnapshotRegistry.MaxCards)
                return false;
            int resolvedCost = card.EnergyCost.GetResolved();
            LiveCardField<int> cost = resolvedCost >= 0
                ? LiveCardField<int>.Available(resolvedCost)
                : LiveCardField<int>.Unknown();
            int upgradeLevel = card.CurrentUpgradeLevel;
            LiveCardField<ushort> upgrade = upgradeLevel is >= 0 and <= ushort.MaxValue
                ? LiveCardField<ushort>.Available((ushort)upgradeLevel)
                : LiveCardField<ushort>.Unknown();
            string ownerId = owner.NetId.ToString(CultureInfo.InvariantCulture);
            output.Add(new LiveCardCaptureInput(
                card,
                LiveCardField<string>.Available(card.Id.Entry),
                string.IsNullOrEmpty(contentManifest)
                    ? LiveCardField<string>.NotObserved()
                    : LiveCardField<string>.Available(contentManifest),
                LiveCardField<string>.Available(ownerId),
                LiveCardField<LiveCardLocation>.Available(new LiveCardLocation(zone, position)),
                upgrade,
                LiveCardField<string>.NotObserved(),
                LiveCardField<string>.NotObserved(),
                LiveCardField<string>.Available(card.Title),
                LiveCardField<bool>.Available(card.IsUpgraded),
                cost,
                LiveCardField<int>.NotObserved(),
                LiveCardField<int>.NotObserved(),
                LiveCardField<IReadOnlyList<string>>.NotObserved(),
                LiveCardField<IReadOnlyList<string>>.NotObserved(),
                LiveCardField<IReadOnlyDictionary<string, string>>.NotObserved()));
            position++;
        }
        return true;
    }
}
