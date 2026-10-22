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
    /// <summary>
    /// Rechecks the current owner values without allocating a new occurrence or snapshot epoch.
    /// Raw energy and costs are compared separately from the compact gameplay fingerprint.
    /// </summary>
    internal bool IsLiveCardSnapshotCurrent(LiveCardCapturedSnapshot snapshot)
    {
        RequireThread();
        try
        {
            SaveManager saves = SaveManager.Instance;
            if (!saves.IsProfileInitialized || _liveCardProfile is null
                || _liveCardProfileId != saves.CurrentProfileId
                || !ReferenceEquals(_liveCardProfile, saves.Progress))
                return false;
            RunManager manager = RunManager.Instance;
            Player? player = CurrentPlayer();
            if (!manager.IsInProgress || manager.DebugOnlyGetState() is not { } run
                || player is null || snapshot.StateGeneration != Observe().Generation
                || snapshot.ModeId != run.GameMode.ToString()
                || snapshot.PlayerEnergy != ReadPlayerEnergy(player))
                return false;

            var cards = new List<LiveCardCaptureInput>();
            if (player.PlayerCombatState is { } combat
                && (!AddCards(cards, combat.Hand.Cards, LiveCardZone.Hand, player,
                        snapshot.ContentManifest)
                    || !AddCards(cards, combat.DiscardPile.Cards, LiveCardZone.Discard, player,
                        snapshot.ContentManifest)
                    || !AddCards(cards, combat.ExhaustPile.Cards, LiveCardZone.Exhaust, player,
                        snapshot.ContentManifest)))
                return false;
            if (!AddCards(cards, player.Deck.Cards, LiveCardZone.Deck, player,
                    snapshot.ContentManifest))
                return false;

            string runKey = $"{run.GameMode}|{run.Rng.StringSeed}|"
                + player.NetId.ToString(CultureInfo.InvariantCulture);
            return snapshot.StateGeneration == Observe().Generation
                && ReferenceEquals(run, manager.DebugOnlyGetState())
                && ReferenceEquals(player, CurrentPlayer())
                && saves.IsProfileInitialized && _liveCardProfileId == saves.CurrentProfileId
                && ReferenceEquals(_liveCardProfile, saves.Progress)
                && snapshot.PlayerEnergy == ReadPlayerEnergy(player)
                && _liveCardRegistry.MatchesCurrent(snapshot, run, runKey, cards);
        }
        catch (Exception)
        {
            return false;
        }
    }
}
