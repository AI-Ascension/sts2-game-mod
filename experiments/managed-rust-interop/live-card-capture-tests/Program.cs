// SPDX-License-Identifier: MIT
using System;
using System.Collections.Generic;
using System.Reflection;
using System.Threading.Tasks;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Saves;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class Program
{
    private static int _checks;
    private static void Main()
    {
        RawCosts();
        ReadsPreserveHostStateAndRng();
        Freshness();
        Failures();
        DeferredReads();
        Console.WriteLine($"PASS live-card capture: {_checks} source-linked checks");
    }

    private static (LiveCombatSource Source, LiveCardSourceReadAdapter Reader, ThreadQueue Thread,
        Player Player, FixedCard Card) Setup()
    {
        RunManager.Instance = new();
        SaveManager.Instance = new();
        NativeContentCatalogSnapshot.Language = "en-US";
        NativeContentCatalogManifestSource.Manifest = "manifest";
        NativeContentCatalogManifestSource.Available = true;
        NativeContentCatalogManifestSource.Witness = new(1, "adapter", "content", "text", "manifest");
        LiveCombatSource.Generation = 1;
        var player = new Player();
        var card = new FixedCard();
        player.PlayerCombatState!.Hand.Cards.Add(card);
        LiveCombatSource.Player = player;
        var source = new LiveCombatSource();
        var thread = new ThreadQueue();
        return (source, new(source, thread), thread, player, card);
    }

    private static LiveCardCapturedSnapshot Read(LiveCardSourceReadAdapter reader) =>
        reader.Read("instance", "manifest");

    private static LiveCardCapturedSnapshot Retained(
        LiveCardSourceReadAdapter reader, LiveCardCapturedSnapshot snapshot) =>
        reader.ReadRetained("instance", "manifest", snapshot.Epoch);

    private static void Check(bool value, string name)
    {
        if (!value) throw new InvalidOperationException(name);
        _checks++;
    }

    private static int Handles(LiveCombatSource source)
    {
        var registry = (LiveCardSnapshotRegistry)typeof(LiveCombatSource).GetField(
            "_liveCardRegistry", BindingFlags.NonPublic | BindingFlags.Instance)!.GetValue(source)!;
        return ((System.Collections.IDictionary)typeof(LiveCardSnapshotRegistry).GetField(
            "_occurrences", BindingFlags.NonPublic | BindingFlags.Instance)!.GetValue(registry)!).Count;
    }

    private static void RawCosts()
    {
        var f = Setup();
        foreach (int raw in new[] { 0, 1, 256, 10000, -1 })
        {
            f.Player.PlayerCombatState!.Energy = raw;
            f.Card.EnergyCost.Value = raw;
            LiveCardCapturedSnapshot snapshot = Read(f.Reader);
            Check(snapshot.Available && snapshot.Energy.Value == raw, "raw energy not clamped");
            Check(snapshot.ModeId == "standard" && snapshot.Locale == "en-US"
                && snapshot.ManifestWitness == NativeContentCatalogManifestSource.Witness,
                "capture owned identity");
            Check(raw < 0 ? snapshot.Cards[0].ResolvedCost.Status == LiveCardFieldStatus.Unknown
                : snapshot.Cards[0].ResolvedCost.Value == raw, "fixed cost not clamped");
            Check(Retained(f.Reader, snapshot) == snapshot, "same epoch remains current");
        }
        f.Card.X = true;
        Check(Read(f.Reader).Cards[0].ResolvedCost.Status == LiveCardFieldStatus.Unsupported,
            "X never becomes resolved fixed even for nonnegative cost");
        f.Card.X = false;
        f.Card.Throws = true;
        Check(Read(f.Reader).Cards[0].ResolvedCost.Status == LiveCardFieldStatus.Failed,
            "throwing X signal unavailable");
        f.Card.Throws = false;
        f.Card.EnergyCost.Throws = true;
        Check(Read(f.Reader).Cards[0].ResolvedCost.Status == LiveCardFieldStatus.Failed,
            "throwing resolved getter unavailable");
        f.Player.PlayerCombatState!.Hand.Cards.Clear();
        f.Player.PlayerCombatState.Hand.Cards.Add(new CardModel());
        Check(Read(f.Reader).Cards[0].ResolvedCost.Status == LiveCardFieldStatus.Unknown,
            "absent X signal unavailable");
        f.Player.PlayerCombatState = null;
        Check(Read(f.Reader).Energy.Status == LiveCardFieldStatus.NotObserved,
            "absent energy not zero");
    }

    private static void ReadsPreserveHostStateAndRng()
    {
        var f = Setup();
        var run = RunManager.Instance.State!;
        var combat = f.Player.PlayerCombatState!;
        var card = f.Card;
        var progress = SaveManager.Instance.Progress;
        combat.Energy = 7;
        card.EnergyCost.Value = 2;
        run.Rng.StringSeed = "unchanged-seed";
        run.Rng.DrawCount = 11;
        SaveManager.Instance.CurrentProfileId = 23;
        ulong generation = LiveCombatSource.Generation;

        for (int index = 0; index < 2; index++)
        {
            LiveCardCapturedSnapshot snapshot = Read(f.Reader);
            Check(snapshot.Available && snapshot.Energy.Value == 7
                && snapshot.Cards[0].ResolvedCost.Value == 2, "owned raw inputs");
            Check(Retained(f.Reader, snapshot) == snapshot, "retained read stays current");
            Check(ReferenceEquals(RunManager.Instance.State, run)
                && ReferenceEquals(f.Player.PlayerCombatState, combat)
                && ReferenceEquals(combat.Hand.Cards[0], card)
                && ReferenceEquals(SaveManager.Instance.Progress, progress)
                && SaveManager.Instance.CurrentProfileId == 23,
                "capture preserves host state and profile");
            Check(run.Rng.StringSeed == "unchanged-seed" && run.Rng.DrawCount == 11,
                "capture preserves RNG state");
            Check(combat.Energy == 7 && card.EnergyCost.Value == 2
                && LiveCombatSource.Generation == generation,
                "capture preserves observed values and generation");
        }
    }

    private static void Freshness()
    {
        Action<Player, FixedCard>[] changes =
        {
            (p, c) => p.PlayerCombatState!.Energy = 257,
            (p, c) => c.EnergyCost.Value = 257,
            (p, c) => c.X = true,
            (p, c) => c.Throws = true,
            (p, c) => c.EnergyCost.Throws = true,
            (p, c) => c.CurrentUpgradeLevel++,
            (p, c) => c.IsUpgraded = true,
            (p, c) => c.Title = "Changed",
            (p, c) => c.Id = new("card", "changed"),
            (p, c) => p.NetId++,
            (p, c) => p.PlayerCombatState!.Hand.Cards[0] = new FixedCard(),
            (p, c) => p.PlayerCombatState!.Hand.Cards.Add(new FixedCard()),
            (p, c) => { p.PlayerCombatState!.Hand.Cards.Clear(); p.Deck.Cards.Add(c); },
            (p, c) => p.PlayerCombatState = new(),
            (p, c) => LiveCombatSource.Player = new(),
            (p, c) => LiveCombatSource.Player = null,
            (p, c) => RunManager.Instance.State = new(),
            (p, c) => RunManager.Instance.State!.Rng.StringSeed = "other",
            (p, c) => RunManager.Instance.State!.GameMode = "daily",
            (p, c) => RunManager.Instance.IsInProgress = false,
            (p, c) => RunManager.Instance.State = null,
            (p, c) => SaveManager.Instance.CurrentProfileId++,
            (p, c) => SaveManager.Instance.Progress = new(),
            (p, c) => SaveManager.Instance.Progress = null,
            (p, c) => SaveManager.Instance.IsProfileInitialized = false,
            (p, c) => NativeContentCatalogSnapshot.Language = "fr-FR",
            (p, c) => NativeContentCatalogManifestSource.Manifest = "different",
            (p, c) => NativeContentCatalogManifestSource.Available = false,
            (p, c) => LiveCombatSource.Generation++,
            (p, c) => NativeContentCatalogManifestSource.Witness =
                NativeContentCatalogManifestSource.Witness with { CatalogGeneration = 2 },
            (p, c) => NativeContentCatalogManifestSource.Witness =
                NativeContentCatalogManifestSource.Witness with { AdapterCompatibility = "new" },
            (p, c) => NativeContentCatalogManifestSource.Witness =
                NativeContentCatalogManifestSource.Witness with { ContentSetRevision = "new" },
            (p, c) => NativeContentCatalogManifestSource.Witness =
                NativeContentCatalogManifestSource.Witness with { LocalizedTextRevision = "new" },
            (p, c) => NativeContentCatalogManifestSource.Witness =
                NativeContentCatalogManifestSource.Witness with { InventoryRevision = "new" }
        };
        foreach (var change in changes)
        {
            var f = Setup();
            LiveCardCapturedSnapshot snapshot = Read(f.Reader);
            Check(snapshot.Available, "initial capture");
            change(f.Player, f.Card);
            Check(!Retained(f.Reader, snapshot).Available, "changed native value fenced");
            Check(Handles(f.Source) == 0, "stale check clears native handles");
        }
        var negative = Setup();
        negative.Card.EnergyCost.Value = -1;
        var prior = Read(negative.Reader);
        negative.Card.EnergyCost.Value = -2;
        Check(!Retained(negative.Reader, prior).Available, "unknown negative raw changes fenced");
        var epoch = Setup();
        prior = Read(epoch.Reader);
        epoch.Source.CaptureLiveCardSnapshot("instance", "manifest");
        Check(!Retained(epoch.Reader, prior).Available, "external capture epoch fenced");
    }

    private sealed class ThreadQueue : IRuntimeV3HostThread
    {
        internal bool Deferred { get; set; }
        internal bool ThrowBefore { get; set; }
        internal bool ThrowAfter { get; set; }
        internal Queue<Action> Pending { get; } = new();
        public void Enqueue(Action work)
        {
            if (ThrowBefore) throw new InvalidOperationException();
            if (Deferred) Pending.Enqueue(work); else work();
            if (ThrowAfter) throw new InvalidOperationException();
        }
        internal void Drain()
        {
            while (Pending.TryDequeue(out Action? action)) action();
        }
    }
}
