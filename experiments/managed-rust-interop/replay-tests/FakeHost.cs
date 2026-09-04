// SPDX-License-Identifier: MIT

// Synthetic API-shape fakes, not a host implementation or host compatibility evidence.
namespace Godot
{
    internal static class GD
    {
        internal static readonly List<string> Errors = new();
        internal static void Print(string text) { }
        internal static void PrintErr(string text) => Errors.Add(text);
    }
    internal static class Engine
    {
        internal static readonly SceneTree Tree = new();
        internal static object GetMainLoop() => Tree;
    }
    internal sealed class SceneTree
    {
        internal object Root { get; } = new();
        internal event Action? ProcessFrame;
        internal void Pump() => ProcessFrame?.Invoke();
    }
}
namespace MegaCrit.Sts2.Core.Entities.Multiplayer
{
    internal enum NetGameType { Singleplayer, Multiplayer }
}
namespace MegaCrit.Sts2.Core.Multiplayer.Game
{
    internal enum GameMode { Standard, Custom, Daily }
}
namespace MegaCrit.Sts2.Core.Models
{
    internal sealed class CharacterModel { }
    internal sealed class ActModel { }
    internal sealed class ModifierModel { }
}
namespace MegaCrit.Sts2.Core.Runs
{
    using MegaCrit.Sts2.Core.Models;
    using MegaCrit.Sts2.Core.Multiplayer.Game;
    using MegaCrit.Sts2.Core.Entities.Multiplayer;
    internal sealed class RunState
    {
        internal sealed class Player { public CharacterModel Character { get; } = new(); }
        internal sealed class RandomState { public string StringSeed { get; set; } = "SYNTHETIC"; }
        public List<Player> Players { get; } = new() { new() };
        public List<ActModel> Acts { get; } = new() { new() };
        public List<ModifierModel> Modifiers { get; } = new() { new() };
        public RandomState Rng { get; } = new();
        public int AscensionLevel { get; set; } = 3;
    }
    internal sealed class RunManager
    {
        internal static RunManager Instance { get; set; } = new();
        internal sealed class Network { public NetGameType Type { get; set; } }
        internal object? Mode { get; set; } = Multiplayer.Game.GameMode.Custom;
        private object? GameMode => Mode;
        internal bool IsInProgress { get; set; } = true;
        internal Network NetService { get; } = new();
        internal object? DailyTime { get; set; }
        internal bool ShouldSave { get; set; } = true;
        internal RunState State { get; set; } = new();
        internal int Cleanups { get; private set; }
        internal RunState DebugOnlyGetState() => State;
        internal void CleanUp(bool graceful) => Cleanups++;
    }
}
namespace MegaCrit.Sts2.Core.Saves
{
    internal sealed class SaveManager
    {
        internal static SaveManager Instance { get; set; } = new();
        internal Task? CurrentRunSaveTask { get; set; }
        internal int CurrentProfileId { get; set; } = 1;
        internal bool IsProfileInitialized { get; set; } = true;
        internal int Deletes { get; private set; }
        internal void DeleteCurrentRun() => Deletes++;
    }
}
namespace MegaCrit.Sts2.Core.Nodes
{
    using MegaCrit.Sts2.Core.Models;
    using MegaCrit.Sts2.Core.Multiplayer.Game;
    internal sealed class NGame
    {
        internal static NGame Instance { get; set; } = new();
        internal int Starts { get; private set; }
        internal bool Fail { get; set; }
        internal string? Seed { get; private set; }
        internal Task StartNewSingleplayerRun(CharacterModel character, bool shouldSave,
            IReadOnlyList<ActModel> acts, IReadOnlyList<ModifierModel> modifiers, string seed,
            GameMode gameMode, int ascensionLevel, object? dailyTime)
        {
            Starts++;
            Seed = seed;
            return Fail ? Task.FromException(new InvalidOperationException("PRIVATE-PATH")) : Task.CompletedTask;
        }
    }
}
namespace AiAscension.Sts2GameMod.Runtime
{
    internal static class StandaloneProfileSettings
    {
        internal static bool AllowRepeatingSeeds { get; set; } = true;
        internal static Func<bool>? Replay { get; private set; }
        internal static bool TryRegisterReplaySeedResetCallback(Func<bool> callback)
        {
            Replay = callback;
            return true;
        }
    }
}
