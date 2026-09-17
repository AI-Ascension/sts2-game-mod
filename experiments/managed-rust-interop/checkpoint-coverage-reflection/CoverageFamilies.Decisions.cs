// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.CheckpointCoverageReflection;

internal static partial class CoverageFamilies
{
    private const string RewardsSet = "Rewards.RewardsSet";
    private const string Reward = "Rewards.Reward";
    private const string CardReward = "Rewards.CardReward";
    private const string EventModel = "Models.EventModel";
    private const string EventOption = "Events.EventOption";
    private const string MerchantInventory = "Entities.Merchant.MerchantInventory";
    private const string MerchantEntry = "Entities.Merchant.MerchantEntry";
    private const string RestSiteOption = "Entities.RestSite.RestSiteOption";

    private static CoverageFamily NonCombatDecisions() => new("non-combat-decisions", "0037", "Non-combat decisions",
        "Offers and pending choices are not fully serialized by the host run save (`SerializableRoom.ExtraRewards/EventId` and `SerializableRun.EventsSeen` only); a checkpoint must copy the offer values listed here.",
        "`RewardsSet.Rewards`, `CardReward.Cards`, `EventModel.CurrentOptions`, the merchant entry lists, and `RestSiteRoom.Options` are lists and keep host order.",
        "No host-owned restore path for a pending offer is observed; these rows stay `unsupported pending inventory` in the boundary matrix.",
        "An offer with `_isGenerated`/`IsPopulated` false, an unresolved `_rngOverride`, a merchant entry without `Cost`, or a pending `PlayerChoiceContext` rejects capture.",
        [
            P("reward offer", RewardsSet, "Rewards"), P("reward offer", RewardsSet, "Id"), P("reward offer", RewardsSet, "Room"),
            P("reward offer", RewardsSet, "DisallowSkipping"), F("reward offer", RewardsSet, "_isGenerated"),
            P("reward offer", Reward, "SuccessfullySelected"), P("reward offer", Reward, "RewardsSetIndex"), F("reward offer", Reward, "_rngOverride"),
            P("card selection offer", CardReward, "Cards"), P("card selection offer", CardReward, "CanReroll"), P("card selection offer", CardReward, "CanSkip"),
            P("card selection offer", CardReward, "Options"), P("card selection offer", CardReward, "RerollOptions"), F("card selection offer", CardReward, "_hasBeenRerolled"),
            P("card selection offer", "Runs.CardCreationOptions", "Source"), P("card selection offer", "Runs.CardCreationOptions", "RngOverride"),
            P("reward offer", "Rooms.CombatRoom", "ExtraRewards"), P("reward offer", RunManager, "RewardsSetSynchronizer"),
            P("event", "Rooms.EventRoom", "CanonicalEvent"), P("event", "Rooms.EventRoom", "LocalMutableEvent"),
            P("event", EventModel, "CurrentOptions"), P("event", EventModel, "IsFinished"), P("event", EventModel, "Rng"),
            P("event", EventModel, "DynamicVars"), P("event", EventModel, "IsDeterministic"),
            P("event", EventOption, "TextKey"), P("event", EventOption, "IsLocked"), P("event", EventOption, "IsProceed"), P("event", EventOption, "WasChosen"),
            P("shop", "Rooms.MerchantRoom", "Inventories"), P("shop", MerchantInventory, "CharacterCardEntries"),
            P("shop", MerchantInventory, "ColorlessCardEntries"), P("shop", MerchantInventory, "RelicEntries"),
            P("shop", MerchantInventory, "PotionEntries"), P("shop", MerchantInventory, "CardRemovalEntry"),
            P("shop", MerchantEntry, "Cost"), P("shop", MerchantEntry, "IsStocked"), P("shop", "Entities.Players.ExtraPlayerFields", "CardShopRemovalsUsed"),
            P("rest", "Rooms.RestSiteRoom", "Options"), P("rest", RestSiteOption, "OptionId"), P("rest", RestSiteOption, "IsEnabled"),
            P("rest", RestSiteOption, "Owner"), P("rest", RunManager, "RestSiteSynchronizer"),
            T("treasure", "Rooms.TreasureRoom"), F("treasure", "Entities.TreasureRelicPicking.RelicPickingFight", "rounds"),
            P("treasure", RunManager, "TreasureRoomRelicSynchronizer"),
            T("pending choice", "GameActions.Multiplayer.PlayerChoiceSynchronizer"),
            F("pending choice", "Entities.Actions.GameActionState", "GatheringPlayerChoice"),
            P("serialized form", SerializableRoom, "ExtraRewards"), P("serialized form", SerializableRoom, "EventId"),
            P("serialized form", SerializableRun, "EventsSeen"),
        ]);

    private static CoverageFamily PersistenceAndIdentity() => new("persistence-and-identity", "0037", "Persistence and identity",
        "`SerializableRun` through `SaveManager.SaveRun/LoadRunSave` (System.Text.Json via `MegaCritSerializerContext`); canonical checkpoint bytes are mod-owned `asc-jcs-state-v1` (ADR 0054), not a host format.",
        "The identity tuple is fixed by the mod (ADR 0055); host lists keep order and host hash sets are sorted canonically before encoding.",
        "Restore stays unsupported (ADR 0042); duplicate admission and the durable-versus-in-memory distinction are mod-owned (ADR 0055).",
        "A missing `RunManager.State`, an uninitialized profile, `IsAbandoned` or `IsGameOver` true, or a pending `CurrentRunSaveTask` rejects capture.",
        [
            P("run binding", RunManager, "Instance"), P("run binding", RunManager, "State"), P("run binding", RunManager, "IsInProgress"),
            P("run binding", RunManager, "IsGameOver"), P("run binding", RunManager, "IsAbandoned"), P("run binding", RunManager, "ShouldSave"),
            P("profile binding", SaveManager, "Instance"), P("profile binding", SaveManager, "CurrentProfileId"),
            P("profile binding", SaveManager, "IsProfileInitialized"), P("profile binding", SaveManager, "HasRunSave"),
            P("profile binding", SaveManager, "CurrentRunSaveTask"), F("profile binding", SaveManager, "_saveStore"),
            P("session binding", Player, "NetId"), P("session binding", "Context.LocalContext", "NetId"),
            F("session binding", "Entities.Multiplayer.RunSessionState", "Running"),
            P("epoch/profile inputs", "Saves.ProgressState", "Epochs"), P("epoch/profile inputs", "Saves.ProgressState", "UniqueId"),
            P("epoch/profile inputs", "Saves.SerializableEpoch", "Id"), P("epoch/profile inputs", "Saves.SerializableEpoch", "State"),
            F("epoch/profile inputs", "Saves.EpochState", "Obtained"),
            M("saved closure descriptors", SaveManager, "SaveRun"), M("saved closure descriptors", SaveManager, "LoadRunSave"),
            M("saved closure descriptors", SaveManager, "IncrementNumReloads"),
            P("saved closure descriptors", SerializableRun, "SchemaVersion"), P("saved closure descriptors", SerializableRun, "Players"),
            P("saved closure descriptors", SerializableRun, "SerializableRng"), P("saved closure descriptors", SerializableRun, "SerializableOdds"),
            P("saved closure descriptors", SerializableRun, "SerializableSharedRelicGrabBag"),
            P("saved closure descriptors", SerializableRun, "SaveTime"), P("saved closure descriptors", SerializableRun, "StartTime"),
            P("saved closure descriptors", SerializableRun, "NumReloads"), P("saved closure descriptors", SerializableRun, "GameMode"),
            P("saved closure descriptors", SerializableRun, "Ascension"),
            T("ordered canonical bytes (host JSON only; canonical form is mod-owned)", "Saves.MegaCritSerializerContext"),
            T("ordered canonical bytes (host JSON only; canonical form is mod-owned)", "Saves.JsonSerializationUtility"),
        ]);
}
