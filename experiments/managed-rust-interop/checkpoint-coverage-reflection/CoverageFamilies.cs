// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.CheckpointCoverageReflection;

/// <summary>
/// The inventory rows: every ADR 0037 coverage family and every ADR 0040 audit row mapped to the
/// concrete host members that carry that state on the pinned build. Names only; no values.
/// </summary>
internal static partial class CoverageFamilies
{
    private const string Core = "MegaCrit.Sts2.Core.";
    private const string RunState = "Runs.RunState";
    private const string RunManager = "Runs.RunManager";
    private const string RunRngSet = "Runs.RunRngSet";
    private const string Rng = "Random.Rng";
    private const string MegaRandom = "Random.MegaRandom";
    private const string PlayerRngSet = "Random.PlayerRngSet";
    private const string Player = "Entities.Players.Player";
    private const string CombatPlayer = "Entities.Players.PlayerCombatState";
    private const string Creature = "Entities.Creatures.Creature";
    private const string CardModel = "Models.CardModel";
    private const string CombatState = "Combat.CombatState";
    private const string CombatManager = "Combat.CombatManager";
    private const string SerializableRun = "Saves.SerializableRun";
    private const string SerializablePlayer = "Saves.Runs.SerializablePlayer";
    private const string SerializableRoom = "Saves.Runs.SerializableRoom";
    private const string SaveManager = "Saves.SaveManager";
    private const string ActionQueueSet = "GameActions.Multiplayer.ActionQueueSet";
    private const string ActionExecutor = "GameActions.ActionExecutor";
    private const string GameAction = "GameActions.GameAction";

    internal static IReadOnlyList<CoverageFamily> All { get; } =
    [
        Compatibility(), SeedAndEntropy(), Campaign(), PlayerFamily(), Combat(), NonCombatDecisions(),
        PersistenceAndIdentity(), RngMasterSeed(), RngMapAndAct(), RngEncounters(), RngShuffleAndOffers(),
        RngOrderingAndExternalEntropy(), RngCosmeticOnly(),
    ];

    private static MemberExpectation F(string item, string type, string member) => new(Core + type, member, MemberKind.Field, item);
    private static MemberExpectation P(string item, string type, string member) => new(Core + type, member, MemberKind.Property, item);
    private static MemberExpectation M(string item, string type, string member) => new(Core + type, member, MemberKind.Method, item);
    private static MemberExpectation T(string item, string type) => new(Core + type, type[(type.LastIndexOf('.') + 1)..], MemberKind.Type, item);
    private static MemberExpectation Absent(string item, string type, string text) => new(Core + type, text, MemberKind.NoMemberContaining, item);

    private static CoverageFamily Compatibility() => new("compatibility", "0037", "Compatibility",
        "Host save carries `SerializableRun.SchemaVersion/GameMode/Ascension/Modifiers/PlatformType`; build identity comes from `ReleaseInfo`, adapter revision from the mod package (not a host member).",
        "Scalar values; `Modifiers` and `BadgeModels` are `IReadOnlyList` and keep host order.",
        "Compatibility is checked, never restored: a differing build, adapter, mode, character, ascension, modifier, or profile tuple rejects the checkpoint (ADR 0042 stays `no_restore_adapter`).",
        "Any missing tuple element rejects capture with `unsupported_coverage`; no default build, mode, or profile is assumed.",
        [
            P("game build", "Debug.ReleaseInfo", "Version"), P("game build", "Debug.ReleaseInfo", "Commit"),
            P("game build", "Debug.ReleaseInfo", "MainAssemblyHash"),
            M("adapter revision (loaded mod identity)", "Modding.ModManager", "GetLoadedMods"),
            F("adapter revision (loaded mod identity)", "Modding.Mod", "version"),
            F("adapter revision (loaded mod identity)", "Modding.ModManifest", "id"),
            F("adapter revision (loaded mod identity)", "Modding.ModManifest", "version"),
            F("adapter revision (loaded mod identity)", "Modding.ModManifest", "affectsGameplay"),
            F("adapter revision (loaded mod identity)", "Modding.ModManifest", "minGameVersion"),
            P("mode", RunState, "GameMode"), F("mode", "Runs.GameMode", "Standard"), F("mode", "Runs.GameMode", "Daily"),
            F("mode", "Runs.GameMode", "Custom"), P("mode", RunManager, "IsSingleplayerOrFakeMultiplayer"),
            P("character", Player, "Character"),
            P("ascension", RunState, "AscensionLevel"), P("ascension", Player, "MaxAscensionWhenRunStarted"),
            P("modifiers", RunState, "Modifiers"), P("modifiers", RunState, "BadgeModels"),
            P("modifiers", RunState, "ExtraFields"), P("modifiers", "Runs.ExtraRunFields", "StartedWithNeow"),
            P("profile compatibility", RunState, "UnlockState"), P("profile compatibility", Player, "UnlockState"),
            P("profile compatibility", "Unlocks.UnlockState", "NumberOfRuns"),
            F("profile compatibility", "Unlocks.UnlockState", "_unlockedEpochIds"),
            F("profile compatibility", "Unlocks.UnlockState", "_encountersSeen"),
            P("profile compatibility", SaveManager, "CurrentProfileId"), P("profile compatibility", "Saves.ProgressState", "UniqueId"),
            P("profile compatibility", SerializableRun, "SchemaVersion"), P("profile compatibility", SerializableRun, "PlatformType"),
        ]);

    private static CoverageFamily SeedAndEntropy() => new("seed-and-entropy", "0037", "Seed and entropy",
        "Host saves the string seed plus one counter per stream (`SerializableRunRngSet.Counters`, `SerializablePlayerRngSet.Counters`), not the four `UInt64` state words; a checkpoint carries stream seed and counter per `RunRngType`/`PlayerRngType` and pins the derivation constructor version.",
        "Streams are keyed by enum in `Dictionary<RunRngType,Rng>`; canonical order is the enum ordinal, never dictionary iteration order.",
        "Host-owned via `RunRngSet.LoadFromSerializable` and `Rng.FastForwardCounter`; the mod never writes a cursor (ADR 0040).",
        "A stream whose `Seed` or `Counter` cannot be observed rejects capture; `Chaotic` streams are recorded as present but never certified deterministic.",
        [
            P("master seed", RunRngSet, "StringSeed"), P("master seed", RunRngSet, "Seed"), M("master seed", RunRngSet, ".ctor"),
            M("derivation version", Rng, ".ctor"), M("derivation version", MegaRandom, "Splitmix64"), M("derivation version", MegaRandom, "Reinitialise"),
            P("RNG state/cursor", Rng, "Seed"), P("RNG state/cursor", Rng, "Counter"), F("RNG state/cursor", Rng, "_random"),
            P("RNG state/cursor", Rng, "Chaotic"), M("RNG state/cursor", Rng, "FastForwardCounter"),
            F("RNG state/cursor", MegaRandom, "_s0"), F("RNG state/cursor", MegaRandom, "_s1"),
            F("RNG state/cursor", MegaRandom, "_s2"), F("RNG state/cursor", MegaRandom, "_s3"),
            F("RNG state/cursor", RunRngSet, "_rngs"), F("RNG state/cursor", PlayerRngSet, "_rngs"),
            P("RNG state/cursor", PlayerRngSet, "Seed"), P("RNG state/cursor", Player, "PlayerRng"),
            P("serialized cursor form", "Saves.Runs.SerializableRunRngSet", "Seed"),
            P("serialized cursor form", "Saves.Runs.SerializableRunRngSet", "Counters"),
            P("serialized cursor form", "Saves.SerializablePlayerRngSet", "Seed"),
            P("serialized cursor form", "Saves.SerializablePlayerRngSet", "Counters"),
            M("serialized cursor form", RunRngSet, "ToSerializable"), M("serialized cursor form", RunRngSet, "LoadFromSerializable"),
        ]);
}
