// SPDX-License-Identifier: MIT
// Original synthetic API-shaped fakes; no game assemblies, source, saves or profiles.
namespace Godot
{
    public static class GD
    {
        public static List<string> Messages { get; } = new();
        public static void Print(string message) => Messages.Add(message);
        public static void PrintErr(string message) => Messages.Add(message);
    }
    public sealed class SceneTree
    {
        public event Action? ProcessFrame;
        public void Tick() => ProcessFrame?.Invoke();
    }
    public static class Engine
    {
        public static object? MainLoop { get; set; } = new SceneTree();
        public static object? GetMainLoop() => MainLoop;
    }
}
namespace MegaCrit.Sts2.Core.Models
{
    public sealed record ModelId(string Value);
    public sealed record Model(ModelId Id);
    public static class ModelDb
    {
        public static Model[] AllCards { get; set; } = [new(new("card"))];
        public static Model[] AllRelics => [new(new("relic"))];
        public static Model[] AllPotions => [new(new("potion"))];
        public static Model[] AllEvents => [new(new("event"))];
        public static Model[] Monsters => [new(new("monster"))];
        public static Model[] Acts => [new(new("act"))];
        public static Model[] AllCharacters => [new(new("character"))];
        public static ModelId GetId<T>() => new(typeof(T).Name);
    }
}
namespace MegaCrit.Sts2.Core.Models.Characters { public sealed class Ironclad; }
namespace MegaCrit.Sts2.Core.Timeline
{
    public enum EpochState { Revealed }
    public sealed record Epoch(string Id, EpochState State);
    public static class EpochModel { public static string[] AllEpochIds => ["epoch"]; }
}
namespace MegaCrit.Sts2.Core.Entities.Players
{
    using MegaCrit.Sts2.Core.Models;
    public sealed class FightStats { public ModelId? Character { get; set; } public int Wins { get; set; } }
    public sealed class EnemyStats { public List<FightStats> FightStats { get; } = new(); }
}
namespace MegaCrit.Sts2.Core.Runs
{
    public sealed class RunManager
    {
        public static RunManager Instance { get; } = new();
        public bool IsInProgress { get; set; }
    }
}
namespace MegaCrit.Sts2.Core.Saves
{
    using MegaCrit.Sts2.Core.Entities.Players;
    using MegaCrit.Sts2.Core.Models;
    using MegaCrit.Sts2.Core.Timeline;
    public sealed class CharacterStats { public int MaxAscension { get; set; } }
    public sealed class ProgressState
    {
        public int Mutations { get; private set; }
        public int MaxMultiplayerAscension { get; set; }
        public List<Epoch> Epochs { get; } = new();
        public CharacterStats Stats { get; } = new();
        public EnemyStats Enemy { get; } = new();
        public void MarkCardAsSeen(ModelId _) => Mutations++;
        public void MarkRelicAsSeen(ModelId _) => Mutations++;
        public void MarkPotionAsSeen(ModelId _) => Mutations++;
        public void MarkEventAsSeen(ModelId _) => Mutations++;
        public void MarkActAsSeen(ModelId _) => Mutations++;
        public CharacterStats GetOrCreateCharacterStats(ModelId _) => Stats;
        public EnemyStats GetOrCreateEnemyStats(ModelId _) => Enemy;
    }
    public sealed class SaveManager
    {
        public static SaveManager Instance { get; } = new();
        public bool IsProfileInitialized { get; set; } = true;
        public int CurrentProfileId { get; set; } = 1;
        public Task? CurrentRunSaveTask { get; set; }
        public int Switches { get; set; }
        public int Saves { get; set; }
        public bool FailSave { get; set; }
        public ProgressState Progress { get; set; } = new();
        public void SwitchProfileId(int id) { Switches++; CurrentProfileId = id; }
        public void ObtainEpochOverride(string id, EpochState state) => Progress.Epochs.Add(new(id, state));
        public void SaveProgressFile()
        {
            if (FailSave) throw new InvalidOperationException("Synthetic save failure");
            Saves++;
        }
    }
}
