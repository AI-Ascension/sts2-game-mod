// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Entities.Cards;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Saves;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private readonly LiveCardSnapshotRegistry _liveCardRegistry = new();
    private object? _liveCardProfile;
    private int? _liveCardProfileId;

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
            SaveManager saves = SaveManager.Instance;
            if (!saves.IsProfileInitialized || saves.CurrentProfileId is < 1 or > 3)
            {
                InvalidateLiveCardSnapshot();
                return LiveCardCapturedSnapshot.Unavailable("profile_source_unavailable");
            }
            object profile = saves.Progress;
            int profileId = saves.CurrentProfileId;
            if (!ReferenceEquals(profile, _liveCardProfile) || profileId != _liveCardProfileId)
                _liveCardRegistry.Invalidate();
            _liveCardProfile = profile;
            _liveCardProfileId = profileId;
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
            LiveCardField<int> energy = ReadPlayerEnergy(player);
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
            if (stateGeneration != Observe().Generation
                || !ReferenceEquals(run, manager.DebugOnlyGetState())
                || !ReferenceEquals(player, CurrentPlayer())
                || !ReferenceEquals(profile, saves.Progress) || profileId != saves.CurrentProfileId
                || !saves.IsProfileInitialized
                || energy != ReadPlayerEnergy(player))
            {
                _liveCardRegistry.Invalidate();
                return LiveCardCapturedSnapshot.Unavailable("mixed_generation");
            }
            return _liveCardRegistry.Capture(
                instanceId, run, runKey, contentManifest, stateGeneration, cards) with
            {
                PlayerEnergy = energy,
                ModeId = run.GameMode.ToString()
            };
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
        _liveCardProfile = null;
        _liveCardProfileId = null;
    }

    private static bool AddCards(
        List<LiveCardCaptureInput> output,
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
                LiveCardField<IReadOnlyDictionary<string, string>>.NotObserved())
            {
                FixedEnergyCost = ReadFixedEnergyCost(card)
            });
            position++;
        }
        return true;
    }

    private static LiveCardField<int> ReadPlayerEnergy(Player player)
    {
        if (player.PlayerCombatState is not { } combat)
            return LiveCardField<int>.NotObserved();
        int energy = combat.Energy;
        return energy >= 0 ? LiveCardField<int>.Available(energy) : LiveCardField<int>.Unknown();
    }

    private static LiveCardField<bool> ReadFixedEnergyCost(CardModel card) =>
        NativeContentCatalogManifestSourceHelpers.ReadOptionalValue(card, "HasEnergyCostX")
            is bool hasEnergyCostX
            ? LiveCardField<bool>.Available(!hasEnergyCostX)
            : LiveCardField<bool>.NotObserved();
}
