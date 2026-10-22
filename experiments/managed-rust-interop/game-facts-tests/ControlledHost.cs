// SPDX-License-Identifier: MIT

// Original source-only host shapes. These compile the production capture methods; they are not
// proprietary assemblies and do not qualify an installed game's API or behavior.
using System;
using System.Collections.Generic;
using System.Reflection;

namespace MegaCrit.Sts2.Core.Combat { internal sealed class CombatShape { } }
namespace MegaCrit.Sts2.Core.Entities.Cards
{
    internal sealed class CardPile
    {
        internal List<MegaCrit.Sts2.Core.Models.CardModel> Cards { get; } = new();
    }
}
namespace MegaCrit.Sts2.Core.Models
{
    internal sealed record ModelId(string Entry);
    internal sealed class EnergyCost
    {
        internal int Value { get; set; } = 2;
        internal int Reads { get; private set; }
        internal bool Throw { get; set; }
        internal int GetResolved()
        {
            Reads++;
            if (Throw) throw new InvalidOperationException("controlled getter failure");
            return Value;
        }
    }
    internal sealed class CardModel
    {
        internal ModelId Id { get; set; } = new("card.fixture");
        internal EnergyCost EnergyCost { get; } = new();
        internal int CurrentUpgradeLevel { get; set; }
        internal string Title { get; set; } = "Fixture card";
        internal bool IsUpgraded { get; set; }
        public object? HasEnergyCostX { get; set; } = false;
    }
}
namespace MegaCrit.Sts2.Core.Entities.Players
{
    internal sealed class CombatState
    {
        internal int Energy { get; set; } = 3;
        internal MegaCrit.Sts2.Core.Entities.Cards.CardPile Hand { get; } = new();
        internal MegaCrit.Sts2.Core.Entities.Cards.CardPile DiscardPile { get; } = new();
        internal MegaCrit.Sts2.Core.Entities.Cards.CardPile ExhaustPile { get; } = new();
    }
    internal sealed class Player
    {
        internal ulong NetId { get; set; } = 1;
        internal CombatState? PlayerCombatState { get; set; } = new();
        internal MegaCrit.Sts2.Core.Entities.Cards.CardPile Deck { get; } = new();
    }
}
namespace MegaCrit.Sts2.Core.Runs
{
    internal sealed class RunRng
    {
        internal string StringSeed { get; set; } = "controlled-seed";
        internal int Draws { get; private set; }
        internal int NextInt() { Draws++; return 1; }
    }
    internal sealed class RunState
    {
        internal string GameMode { get; set; } = "standard";
        internal RunRng Rng { get; } = new();
    }
    internal sealed class RunManager
    {
        internal static RunManager Instance { get; } = new();
        internal bool IsInProgress { get; set; } = true;
        internal RunState? State { get; set; } = new();
        internal RunState? DebugOnlyGetState() => State;
    }
}
namespace MegaCrit.Sts2.Core.Saves
{
    internal sealed class SaveManager
    {
        internal static SaveManager Instance { get; } = new();
        internal bool IsProfileInitialized { get; set; } = true;
        internal int CurrentProfileId { get; set; } = 1;
        internal object Progress { get; set; } = new();
    }
}
namespace AiAscension.Sts2GameMod.Runtime
{
    internal interface IRuntimeV3HostThread { void Enqueue(Action work); }
    internal sealed class ControlledThread : IRuntimeV3HostThread
    {
        internal bool Fail { get; set; }
        internal bool Defer { get; set; }
        internal static bool Active { get; private set; }
        public void Enqueue(Action work)
        {
            if (Fail) throw new InvalidOperationException("controlled queue failure");
            if (Defer) return;
            Active = true;
            try { work(); }
            finally { Active = false; }
        }
    }
    internal sealed partial class LiveCombatSource
    {
        internal static MegaCrit.Sts2.Core.Entities.Players.Player? Player { get; set; }
        internal ulong Generation { get; set; } = 1;
        internal Action? OnObserve { get; set; }
        private static void RequireThread()
        {
            if (!ControlledThread.Active) throw new InvalidOperationException("not owner thread");
        }
        private static MegaCrit.Sts2.Core.Entities.Players.Player? CurrentPlayer() => Player;
        private (ulong Generation, bool Ready) Observe()
        {
            RequireThread();
            OnObserve?.Invoke();
            return (Generation, true);
        }
    }
    internal static class NativeContentCatalogManifestSourceHelpers
    {
        internal static object? ReadOptionalValue(object value, string name) =>
            value.GetType().GetProperty(name,
                BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance)?.GetValue(value);
    }
}
