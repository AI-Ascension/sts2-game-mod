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
        bool captured = false;
        _liveCardFence = null;
        try
        {
            LiveCardHostFence fence = ReadLiveCardFence(contentManifest);
            LiveCardCapturedSnapshot snapshot = _liveCardRegistry.Capture(
                instanceId, fence.Run, fence.RunKey, contentManifest, fence.Generation,
                fence.Cards) with
            {
                Energy = fence.Energy,
                ModeId = fence.ModeId,
                Locale = fence.Locale,
                ManifestWitness = fence.ManifestWitness
            };
            captured = snapshot.Available;
            if (captured) _liveCardFence = fence;
            return snapshot;
        }
        catch (Exception)
        {
            return LiveCardCapturedSnapshot.Unavailable("host_snapshot_unavailable");
        }
        finally
        {
            if (!captured) InvalidateLiveCardSnapshot();
        }
    }

    /// <summary>
    /// Restore/session owners call this on the host thread before exposing a new live snapshot.
    /// No restore implementation is implied by this invalidation hook.
    /// </summary>
    internal void InvalidateLiveCardSnapshot()
    {
        RequireThread();
        _liveCardFence = null;
        _liveCardRegistry.Invalidate();
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
            LiveCardField<int> cost = ReadFixedCardCost(
                card, out LiveCardField<int> rawCost, out LiveCardField<bool> costX);
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
                RawResolvedCost = rawCost,
                HasEnergyCostX = costX
            });
            position++;
        }
        return true;
    }

    private static LiveCardField<int> ReadFixedCardCost(
        CardModel card, out LiveCardField<int> rawCost, out LiveCardField<bool> costX)
    {
        rawCost = LiveCardField<int>.Unknown();
        costX = LiveCardField<bool>.Unknown();
        try
        {
            object? value = NativeContentCatalogManifestSourceHelpers.ReadOptionalValue(
                card, "HasEnergyCostX");
            if (value is not bool isX) return LiveCardField<int>.Unknown();
            costX = LiveCardField<bool>.Available(isX);
            if (isX) return LiveCardField<int>.Unsupported();
            int resolved = card.EnergyCost.GetResolved();
            rawCost = LiveCardField<int>.Available(resolved);
            return resolved >= 0 ? rawCost : LiveCardField<int>.Unknown();
        }
        catch (Exception)
        {
            rawCost = LiveCardField<int>.Failed();
            return LiveCardField<int>.Failed();
        }
    }
}
