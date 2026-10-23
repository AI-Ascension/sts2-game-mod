// SPDX-License-Identifier: MIT
// Independently authored native-shape doubles, not host implementation or compatibility evidence.
using System;
using System.Collections.Generic;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Models;

namespace MegaCrit.Sts2.Core.Combat { internal sealed class Unused { } }
namespace MegaCrit.Sts2.Core.Entities.Cards
{
    internal sealed class Pile { public List<CardModel> Cards { get; } = new(); }
}
namespace MegaCrit.Sts2.Core.Models
{
    internal sealed record ModelId(string Category, string Entry);
    internal class AbstractModel { public ModelId Id { get; set; } = new("card", "strike"); }
    internal sealed class Cost
    {
        public int Value { get; set; } = 1;
        public bool Throws { get; set; }
        public int GetResolved() => Throws ? throw new InvalidOperationException() : Value;
    }
    internal class CardModel : AbstractModel
    {
        public Cost EnergyCost { get; } = new();
        public int CurrentUpgradeLevel { get; set; }
        public bool IsUpgraded { get; set; }
        public string Title { get; set; } = "Strike";
    }
    internal sealed class FixedCard : CardModel
    {
        public bool X { get; set; }
        public bool Throws { get; set; }
        public bool HasEnergyCostX => Throws ? throw new InvalidOperationException() : X;
    }
}
namespace MegaCrit.Sts2.Core.Entities.Players
{
    internal sealed class PlayerCombatState
    {
        public int Energy { get; set; } = 3;
        public Cards.Pile Hand { get; } = new();
        public Cards.Pile DiscardPile { get; } = new();
        public Cards.Pile ExhaustPile { get; } = new();
    }
    internal sealed class Player
    {
        public ulong NetId { get; set; } = 1;
        public PlayerCombatState? PlayerCombatState { get; set; } = new();
        public Cards.Pile Deck { get; } = new();
    }
}
namespace MegaCrit.Sts2.Core.Runs
{
    internal sealed class Rng { public string StringSeed { get; set; } = "seed"; }
    internal sealed class RunState
    {
        public string GameMode { get; set; } = "standard";
        public Rng Rng { get; } = new();
    }
    internal sealed class RunManager
    {
        public static RunManager Instance { get; set; } = new();
        public bool IsInProgress { get; set; } = true;
        public RunState? State { get; set; } = new();
        public RunState? DebugOnlyGetState() => State;
    }
}
namespace MegaCrit.Sts2.Core.Saves
{
    internal sealed class SaveManager
    {
        public static SaveManager Instance { get; set; } = new();
        public bool IsProfileInitialized { get; set; } = true;
        public int CurrentProfileId { get; set; } = 1;
        public object? Progress { get; set; } = new();
    }
}
namespace AiAscension.Sts2GameMod.Runtime
{
    internal interface IRuntimeV3HostThread { void Enqueue(Action work); }
    internal sealed partial class LiveCombatSource
    {
        private readonly int _owner = Environment.CurrentManagedThreadId;
        internal static Player? Player { get; set; }
        internal static ulong Generation { get; set; } = 1;
        private void RequireThread()
        {
            if (_owner != Environment.CurrentManagedThreadId) throw new InvalidOperationException();
        }
        private static Player? CurrentPlayer() => Player;
        private Observation Observe()
        {
            RequireThread();
            return new(Generation);
        }
        private sealed record Observation(ulong Generation);
    }
    internal sealed record NativeContentIndexSnapshot(string ManifestId, string Locale)
    {
        internal NativeContentManifestWitness? ManifestWitness { get; init; }
    }
    internal sealed record NativeContentIndexCapture(NativeContentIndexSnapshot Snapshot);
    internal sealed record NativeContentCatalogSnapshot(string Locale)
    {
        internal static string Language { get; set; } = "en-US";
        internal static NativeContentCatalogSnapshot CaptureKnownInputs() => new(Language);
    }
    internal static class NativeContentCatalogManifestSource
    {
        internal static string Manifest { get; set; } = "manifest";
        internal static bool Available { get; set; } = true;
        internal static NativeContentManifestWitness Witness { get; set; } =
            new(1, "adapter", "content", "text", "manifest");
        internal static bool TryCaptureCanonicalContentIndexWithSource(
            string correlation, string locale, out NativeContentIndexCapture capture)
        {
            capture = new(new(Manifest, locale) { ManifestWitness = Witness });
            return Available;
        }
    }
    internal static partial class NativeContentCatalogManifestSourceHelpers
    {
        private const int MaxCollectionItems = 512;
        private const int MaxValueBytes = 65536;
    }
    internal static class ContentManifestWireContract
    {
        internal static bool ValidIdentity(string value) => value.Length > 0;
    }
}
