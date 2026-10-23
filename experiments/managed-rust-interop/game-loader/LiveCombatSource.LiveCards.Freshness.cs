// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Saves;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    // Only the host-thread owner retains these handles. They never enter the returned DTO.
    private LiveCardHostFence? _liveCardFence;

    private sealed record LiveCardHostFence(
        object Run, object Player, object? Combat, object Progress, int ProfileId,
        string RunKey, string ModeId, string Locale, NativeContentManifestWitness ManifestWitness,
        ulong Generation, LiveCardField<int> Energy, IReadOnlyList<LiveCardCaptureInput> Cards);

    internal bool IsLiveCardSnapshotCurrent(LiveCardCapturedSnapshot snapshot)
    {
        RequireThread();
        bool current = false;
        try
        {
            LiveCardHostFence? previous = _liveCardFence;
            if (previous is null || !_liveCardRegistry.IsCurrent(
                    snapshot, snapshot.InstanceId, snapshot.ContentManifest)) return false;
            LiveCardHostFence now = ReadLiveCardFence(snapshot.ContentManifest);
            if (!ReferenceEquals(previous.Run, now.Run)
                || !ReferenceEquals(previous.Player, now.Player)
                || !ReferenceEquals(previous.Combat, now.Combat)
                || !ReferenceEquals(previous.Progress, now.Progress)
                || previous.ProfileId != now.ProfileId
                || previous.RunKey != now.RunKey || previous.ModeId != now.ModeId
                || previous.Locale != now.Locale || previous.ManifestWitness != now.ManifestWitness
                || previous.Generation != now.Generation || previous.Energy != now.Energy
                || previous.Cards.Count != now.Cards.Count) return false;
            for (int index = 0; index < previous.Cards.Count; index++)
            {
                LiveCardCaptureInput before = previous.Cards[index];
                LiveCardCaptureInput after = now.Cards[index];
                if (!ReferenceEquals(before.HostCard, after.HostCard) || before != after)
                    return false;
            }
            current = true;
            return true;
        }
        catch (Exception) { return false; }
        finally
        {
            if (!current) InvalidateLiveCardSnapshot();
        }
    }

    private LiveCardHostFence ReadLiveCardFence(string expectedManifest)
    {
        RunManager manager = RunManager.Instance;
        if (!manager.IsInProgress || manager.DebugOnlyGetState() is not { } run)
            throw new InvalidOperationException("run unavailable");
        Player player = CurrentPlayer()
            ?? throw new InvalidOperationException("player unavailable");
        SaveManager saves = SaveManager.Instance;
        if (!saves.IsProfileInitialized || saves.Progress is not { } progress)
            throw new InvalidOperationException("profile unavailable");
        string locale = NativeContentCatalogSnapshot.CaptureKnownInputs().Locale;
        if (!NativeContentCatalogManifestSource.TryCaptureCanonicalContentIndexWithSource(
                "live-card-capture", locale, out NativeContentIndexCapture catalog)
            || catalog.Snapshot.ManifestId != expectedManifest
            || catalog.Snapshot.Locale != locale
            || catalog.Snapshot.ManifestWitness is not { } witness)
            throw new InvalidOperationException("catalog unavailable");
        ulong generation = Observe().Generation;
        var cards = new List<LiveCardCaptureInput>();
        var combat = player.PlayerCombatState;
        if (combat is not null
            && (!AddCards(cards, combat.Hand.Cards, LiveCardZone.Hand, player, expectedManifest)
                || !AddCards(cards, combat.DiscardPile.Cards, LiveCardZone.Discard, player,
                    expectedManifest)
                || !AddCards(cards, combat.ExhaustPile.Cards, LiveCardZone.Exhaust, player,
                    expectedManifest)))
            throw new InvalidOperationException("card count exceeded");
        if (!AddCards(cards, player.Deck.Cards, LiveCardZone.Deck, player, expectedManifest))
            throw new InvalidOperationException("card count exceeded");
        string mode = run.GameMode.ToString();
        string runKey = $"{mode}|{run.Rng.StringSeed}|"
            + player.NetId.ToString(CultureInfo.InvariantCulture);
        return new LiveCardHostFence(run, player, combat, progress, saves.CurrentProfileId,
            runKey, mode, locale, witness, generation,
            combat is null ? LiveCardField<int>.NotObserved()
                : LiveCardField<int>.Available(combat.Energy), cards);
    }
}
