// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.CheckpointCoverageReflection;

internal static partial class CoverageFamilies
{
    private const string RunRngType = "Entities.Rngs.RunRngType";
    private const string PlayerRngType = "Entities.Rngs.PlayerRngType";
    private const string SerializableRunRngSet = "Saves.Runs.SerializableRunRngSet";
    private const string CounterSerialization = "One `Int32` counter per stream in `SerializableRunRngSet.Counters` / `SerializablePlayerRngSet.Counters`; the xoshiro state words are not serialized by the host.";
    private const string EnumOrdering = "Streams are keyed by `RunRngType`/`PlayerRngType`; canonical order is the enum ordinal.";
    private const string HostRestore = "Host-owned via `RunRngSet.LoadFromSerializable` and `Rng.FastForwardCounter`; the mod never writes a cursor.";
    private const string StreamUnknown = "A stream that cannot be read at the boundary fails closed for that build/mode; consumption timing is not certified by metadata.";

    private static CoverageFamily RngMasterSeed() => new("rng-master-seed", "0040", "Master seed",
        "`SerializableRunRngSet.Seed` (`String`) beside `RunRngSet.Seed` (`UInt32`); derivation runs through `RunRngSet..ctor(String)` and `MegaRandom.Splitmix64` (algorithm details are not recorded here).",
        "Single value.", HostRestore, "An unreadable seed returns an explicit unavailable capability.",
        [
            P("canonical seed", RunRngSet, "StringSeed"), P("canonical seed", RunRngSet, "Seed"), M("derivation", RunRngSet, ".ctor"),
            M("derivation", Rng, ".ctor"), M("derivation", MegaRandom, "Splitmix64"), P("serialized form", SerializableRunRngSet, "Seed"),
        ]);

    private static CoverageFamily RngMapAndAct() => new("rng-map-act-generation", "0040", "Map/act generation",
        CounterSerialization, EnumOrdering, HostRestore, StreamUnknown,
        [
            P("stream", RunRngSet, "UpFront"), P("stream", RunRngSet, "UnknownMapPoint"),
            F("stream identity", RunRngType, "UpFront"), F("stream identity", RunRngType, "UnknownMapPoint"),
            F("first-use boundary (map build)", "Map.StandardActMap", "_rng"),
            P("odds input", RunState, "Odds"), P("odds input", "Odds.RunOddsSet", "UnknownMapPoint"),
            M("odds input", "Odds.UnknownMapPointOdds", "Roll"), F("odds input", "Odds.UnknownMapPointOdds", "_nonEventOdds"),
            P("serialized form", SerializableRun, "SerializableOdds"),
        ]);

    private static CoverageFamily RngEncounters() => new("rng-encounters-enemies-targeting", "0040", "Encounters/enemies/targeting",
        CounterSerialization, EnumOrdering, HostRestore, StreamUnknown,
        [
            P("stream", RunRngSet, "MonsterAi"), P("stream", RunRngSet, "CombatTargets"),
            F("stream identity", RunRngType, "MonsterAi"), F("stream identity", RunRngType, "CombatTargets"),
            F("consumer", MonsterModel, "_rng"), F("consumer", MonsterModel, "_runRng"),
            T("consumer", "MonsterMoves.MonsterMoveStateMachine.RandomBranchState"),
        ]);

    private static CoverageFamily RngShuffleAndOffers() => new("rng-shuffle-draw-rewards-shops-events", "0040", "Shuffle/draw/rewards/shops/events",
        CounterSerialization + " Note: the enum member is `RunRngType.CombatOrbs` while the accessor is `RunRngSet.CombatOrbGeneration`.",
        EnumOrdering, HostRestore,
        StreamUnknown + " A `Reward._rngOverride` or `CardCreationOptions.RngOverride` that is set makes the offer's stream identity explicit; an unresolvable override rejects capture.",
        [
            P("stream", RunRngSet, "Shuffle"), P("stream", RunRngSet, "CombatCardGeneration"), P("stream", RunRngSet, "CombatPotionGeneration"),
            P("stream", RunRngSet, "CombatCardSelection"), P("stream", RunRngSet, "CombatEnergyCosts"), P("stream", RunRngSet, "CombatOrbGeneration"),
            P("stream", RunRngSet, "Niche"), P("stream", RunRngSet, "TreasureRoomRelics"),
            F("stream identity", RunRngType, "Shuffle"), F("stream identity", RunRngType, "CombatOrbs"), F("stream identity", RunRngType, "Niche"),
            P("per-player stream", PlayerRngSet, "Rewards"), P("per-player stream", PlayerRngSet, "Shops"), P("per-player stream", PlayerRngSet, "Transformations"),
            F("per-player stream identity", PlayerRngType, "Rewards"), F("per-player stream identity", PlayerRngType, "Shops"),
            F("per-player stream identity", PlayerRngType, "Transformations"),
            F("offer-scoped override", Reward, "_rngOverride"), P("offer-scoped override", "Runs.CardCreationOptions", "RngOverride"),
            P("offer-scoped override", EventModel, "Rng"), P("debug override", CombatManager, "DebugForcedTopCardOnNextShuffle"),
            P("serialized form", "Saves.SerializablePlayerRngSet", "Counters"), P("serialized form", SerializableRunRngSet, "Counters"),
        ]);

    private static CoverageFamily RngOrderingAndExternalEntropy() => new("rng-ordering-external-entropy", "0040", "Ordering and external entropy",
        "Wall-clock values are saved as `Int64` (`SaveTime/StartTime/RunTime/WinTime`, `DailyTime`); object IDs are `UInt32`/`Int32` counters; hash-set contents are saved as lists.",
        "Every hash set listed here has no host-defined order and must be sorted canonically before encoding; task and cancellation members are timing inputs, not state.",
        "Not restorable and not part of identity except the daily seed date; the mod records them as declared external inputs.",
        "A checkpoint taken while `_actionCancelToken`, `_executionTask`, or `_deferredEndTurnTransition` is live is refused as `busy`; an unknown ordering source fails closed.",
        [
            F("wall clock", RunManager, "_startTime"), F("wall clock", RunManager, "_sessionStartTime"), P("wall clock", RunManager, "DailyTime"),
            P("wall clock", RunManager, "RunTime"), P("wall clock", SerializableRun, "SaveTime"), P("wall clock", SerializableRun, "StartTime"),
            P("wall clock", SerializableRun, "DailyTime"),
            F("object IDs", CombatState, "_nextCreatureId"), F("object IDs", ActionQueueSet, "_nextId"), P("object IDs", ActionQueueSet, "NextActionId"),
            P("object IDs", RunState, "NextRoomId"), P("object IDs", RewardsSet, "Id"), P("object IDs", GameAction, "Id"),
            F("hash order", RunState, "_visitedEventIds"), F("hash order", "Map.ActMap", "startMapPoints"), F("hash order", "Map.MapPoint", "parents"),
            P("hash order", "Map.MapPoint", "Children"), F("hash order", "Unlocks.UnlockState", "_encountersSeen"),
            F("hash order", "Runs.RelicGrabBag", "_rarities"), F("hash order", CardModel, "_keywords"), F("hash order", CardModel, "_tags"),
            F("hash order", CombatManager, "_playersReadyToEndTurn"),
            F("async/frame timing", ActionExecutor, "_actionCancelToken"), F("async/frame timing", ActionExecutor, "_queueTaskCompletionSource"),
            F("async/frame timing", GameAction, "_executionTask"), F("async/frame timing", CombatManager, "_deferredEndTurnTransition"),
            F("async/frame timing", "Combat.CombatStateTracker", "_combatStateChangedDeferredTask"),
            P("locale", "Localization.LocManager", "Language"), P("locale", "Localization.LocManager", "CultureInfo"),
            P("locale", "Localization.LocManager", "StringComparer"),
            P("files", "Saves.UserDataPathProvider", "SavesDir"), P("files", "Saves.UserDataPathProvider", "IsRunningModded"),
            M("files", "Saves.ISaveStore", "ReadFile"), M("files", "Saves.ISaveStore", "GetLastModifiedTime"),
            P("files", SerializableRun, "PlatformType"), P("files", SerializableRun, "MapDrawings"),
        ]);

    private static CoverageFamily RngCosmeticOnly() => new("rng-cosmetic-only", "0040", "Cosmetic-only randomness",
        "Not serialized by the host (`Rng.Chaotic` is a backing property; `MegaRandom..ctor()` is the unseeded overload).",
        "Not ordered; excluded from identity only after a live run proves it cannot affect gameplay state.",
        "Not restorable by design.",
        "Metadata shows these members exist; it cannot show they are gameplay-neutral, so this row stays `unverified` for the ADR 0040 required finding.",
        [
            P("chaotic stream", Rng, "Chaotic"), M("unseeded constructor", MegaRandom, ".ctor"),
            F("float increment state", MegaRandom, "_incrDouble"), F("float increment state", MegaRandom, "_incrFloat"),
            P("flavor synchronizer", RunManager, "FlavorSynchronizer"), T("flavor synchronizer", "Multiplayer.Game.FlavorSynchronizer"),
        ]);
}
