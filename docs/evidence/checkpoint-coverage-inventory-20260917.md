# Exact-build checkpoint coverage inventory (ADR 0037, pinned metadata)

Recorded on 2026-09-17 by `experiments/managed-rust-interop/checkpoint-coverage-reflection/` from the metadata tables of the
exact pinned host assembly. The probe never loads, copies, or executes the assembly and records member
names, kinds, and declared types only; no host value, byte, IL, string constant, or install path appears here.

| Field | Value |
| --- | --- |
| Host build | v0.107.1 / `59260271` (pin match: yes) |
| `sts2.dll` SHA-256 | `a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52` |
| `GodotSharp.dll` SHA-256 | `0e4897ecdfb31456a97c7d8028dfb8d7dbdc632e2f73fc9b438d7b266a139289` |
| Assembly name | `sts2` |
| Evidence | `metadata_only_no_target_assembly_load_or_execution` |
| Command | `DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1 dotnet run --project experiments/managed-rust-interop/checkpoint-coverage-reflection/CheckpointCoverageReflection.csproj -c Release -p:STS2GameDataDir=<operator-supplied-host-data> -p:ManagedBuildRoot=<external-build-root> -- --output-dir <dir> --output-stem checkpoint-coverage-inventory-20260917 --recorded-on 2026-09-17` |
| Families | 7 in this table; 0 unresolved |

Labels: `metadata-observed` means the named member exists on the hashed assembly with the recorded kind and
declared type; `metadata-absent` means no member of the named type contains the text; `unresolved` means the
row could not be matched exactly and its family fails closed. Serialization, ordering, restore, and
unknown-value statements below are policy derived from the observed shapes and stay `runtime-unverified`
until an authorized exact-host run records them. Nothing here is native-verified.

## Compatibility (`compatibility`)

- Metadata status: `metadata-observed`; semantics: `runtime-unverified`
- Serialization: Host save carries `SerializableRun.SchemaVersion/GameMode/Ascension/Modifiers/PlatformType`; build identity comes from `ReleaseInfo`, adapter revision from the mod package (not a host member).
- Ordering: Scalar values; `Modifiers` and `BadgeModels` are `IReadOnlyList` and keep host order.
- Restore responsibility: Compatibility is checked, never restored: a differing build, adapter, mode, character, ascension, modifier, or profile tuple rejects the checkpoint (ADR 0042 stays `no_restore_adapter`).
- Unknown-value policy: Any missing tuple element rejects capture with `unsupported_coverage`; no default build, mode, or profile is assumed.

| Closure item | Host member | Kind | Declared type | Visibility | Status |
| --- | --- | --- | --- | --- | --- |
| game build | `Debug.ReleaseInfo.Version` | property | `String` | public | metadata-observed |
| game build | `Debug.ReleaseInfo.Commit` | property | `String` | public | metadata-observed |
| game build | `Debug.ReleaseInfo.MainAssemblyHash` | property | `Int32` | public | metadata-observed |
| adapter revision (loaded mod identity) | `Modding.ModManager.GetLoadedMods` | method | `IEnumerable<Modding.Mod> ()` | public | metadata-observed (overloads=1) |
| adapter revision (loaded mod identity) | `Modding.Mod.version` | field | `Debug.SemanticVersion` | public | metadata-observed |
| adapter revision (loaded mod identity) | `Modding.ModManifest.id` | field | `String` | public | metadata-observed |
| adapter revision (loaded mod identity) | `Modding.ModManifest.version` | field | `String` | public | metadata-observed |
| adapter revision (loaded mod identity) | `Modding.ModManifest.affectsGameplay` | field | `Boolean` | public | metadata-observed |
| adapter revision (loaded mod identity) | `Modding.ModManifest.minGameVersion` | field | `String` | public | metadata-observed |
| mode | `Runs.RunState.GameMode` | property | `Runs.GameMode` | public | metadata-observed |
| mode | `Runs.GameMode.Standard` | field | `Runs.GameMode` | public | metadata-observed |
| mode | `Runs.GameMode.Daily` | field | `Runs.GameMode` | public | metadata-observed |
| mode | `Runs.GameMode.Custom` | field | `Runs.GameMode` | public | metadata-observed |
| mode | `Runs.RunManager.IsSingleplayerOrFakeMultiplayer` | property | `Boolean` | public | metadata-observed |
| character | `Entities.Players.Player.Character` | property | `Models.CharacterModel` | public | metadata-observed |
| ascension | `Runs.RunState.AscensionLevel` | property | `Int32` | public | metadata-observed |
| ascension | `Entities.Players.Player.MaxAscensionWhenRunStarted` | property | `Int32` | public | metadata-observed |
| modifiers | `Runs.RunState.Modifiers` | property | `IReadOnlyList<Models.ModifierModel>` | public | metadata-observed |
| modifiers | `Runs.RunState.BadgeModels` | property | `IReadOnlyList<Models.BadgeModel>` | public | metadata-observed |
| modifiers | `Runs.RunState.ExtraFields` | property | `Runs.ExtraRunFields` | public | metadata-observed |
| modifiers | `Runs.ExtraRunFields.StartedWithNeow` | property | `Boolean` | public | metadata-observed |
| profile compatibility | `Runs.RunState.UnlockState` | property | `Unlocks.UnlockState` | public | metadata-observed |
| profile compatibility | `Entities.Players.Player.UnlockState` | property | `Unlocks.UnlockState` | public | metadata-observed |
| profile compatibility | `Unlocks.UnlockState.NumberOfRuns` | property | `Int32` | public | metadata-observed |
| profile compatibility | `Unlocks.UnlockState._unlockedEpochIds` | field | `HashSet<String>` | private | metadata-observed |
| profile compatibility | `Unlocks.UnlockState._encountersSeen` | field | `HashSet<Models.ModelId>` | private | metadata-observed |
| profile compatibility | `Saves.SaveManager.CurrentProfileId` | property | `Int32` | public | metadata-observed |
| profile compatibility | `Saves.ProgressState.UniqueId` | property | `String` | public | metadata-observed |
| profile compatibility | `Saves.SerializableRun.SchemaVersion` | property | `Int32` | public | metadata-observed |
| profile compatibility | `Saves.SerializableRun.PlatformType` | property | `Platform.PlatformType` | public | metadata-observed |

## Seed and entropy (`seed-and-entropy`)

- Metadata status: `metadata-observed`; semantics: `runtime-unverified`
- Serialization: Host saves the string seed plus one counter per stream (`SerializableRunRngSet.Counters`, `SerializablePlayerRngSet.Counters`), not the four `UInt64` state words; a checkpoint carries stream seed and counter per `RunRngType`/`PlayerRngType` and pins the derivation constructor version.
- Ordering: Streams are keyed by enum in `Dictionary<RunRngType,Rng>`; canonical order is the enum ordinal, never dictionary iteration order.
- Restore responsibility: Host-owned via `RunRngSet.LoadFromSerializable` and `Rng.FastForwardCounter`; the mod never writes a cursor (ADR 0040).
- Unknown-value policy: A stream whose `Seed` or `Counter` cannot be observed rejects capture; `Chaotic` streams are recorded as present but never certified deterministic.

| Closure item | Host member | Kind | Declared type | Visibility | Status |
| --- | --- | --- | --- | --- | --- |
| master seed | `Runs.RunRngSet.StringSeed` | property | `String` | public | metadata-observed |
| master seed | `Runs.RunRngSet.Seed` | property | `UInt32` | public | metadata-observed |
| master seed | `Runs.RunRngSet..ctor` | method | `Void (String)` | public | metadata-observed (overloads=1) |
| derivation version | `Random.Rng..ctor` | method | `Void (Entities.Players.Player, Models.ModelId, UInt32, Int32) ; Void (UInt32, Int32) ; Void (UInt32, String)` | public | metadata-observed (overloads=3) |
| derivation version | `Random.MegaRandom.Splitmix64` | method | `UInt64 (UInt64&)` | public | metadata-observed (overloads=1) |
| derivation version | `Random.MegaRandom.Reinitialise` | method | `Void (UInt64)` | public | metadata-observed (overloads=1) |
| RNG state/cursor | `Random.Rng.Seed` | property | `UInt32` | public | metadata-observed |
| RNG state/cursor | `Random.Rng.Counter` | property | `Int32` | public | metadata-observed |
| RNG state/cursor | `Random.Rng._random` | field | `Random.MegaRandom` | private | metadata-observed |
| RNG state/cursor | `Random.Rng.Chaotic` | property | `Random.Rng` | public | metadata-observed |
| RNG state/cursor | `Random.Rng.FastForwardCounter` | method | `Void (Int32)` | public | metadata-observed (overloads=1) |
| RNG state/cursor | `Random.MegaRandom._s0` | field | `UInt64` | private | metadata-observed |
| RNG state/cursor | `Random.MegaRandom._s1` | field | `UInt64` | private | metadata-observed |
| RNG state/cursor | `Random.MegaRandom._s2` | field | `UInt64` | private | metadata-observed |
| RNG state/cursor | `Random.MegaRandom._s3` | field | `UInt64` | private | metadata-observed |
| RNG state/cursor | `Runs.RunRngSet._rngs` | field | `Dictionary<Entities.Rngs.RunRngType,Random.Rng>` | private | metadata-observed |
| RNG state/cursor | `Random.PlayerRngSet._rngs` | field | `Dictionary<Entities.Rngs.PlayerRngType,Random.Rng>` | private | metadata-observed |
| RNG state/cursor | `Random.PlayerRngSet.Seed` | property | `UInt32` | public | metadata-observed |
| RNG state/cursor | `Entities.Players.Player.PlayerRng` | property | `Random.PlayerRngSet` | public | metadata-observed |
| serialized cursor form | `Saves.Runs.SerializableRunRngSet.Seed` | property | `String` | public | metadata-observed |
| serialized cursor form | `Saves.Runs.SerializableRunRngSet.Counters` | property | `Dictionary<Entities.Rngs.RunRngType,Int32>` | public | metadata-observed |
| serialized cursor form | `Saves.SerializablePlayerRngSet.Seed` | property | `UInt32` | public | metadata-observed |
| serialized cursor form | `Saves.SerializablePlayerRngSet.Counters` | property | `Dictionary<Entities.Rngs.PlayerRngType,Int32>` | public | metadata-observed |
| serialized cursor form | `Runs.RunRngSet.ToSerializable` | method | `Saves.Runs.SerializableRunRngSet ()` | public | metadata-observed (overloads=1) |
| serialized cursor form | `Runs.RunRngSet.LoadFromSerializable` | method | `Void (Saves.Runs.SerializableRunRngSet)` | public | metadata-observed (overloads=1) |

## Campaign (`campaign`)

- Metadata status: `metadata-observed`; semantics: `runtime-unverified`
- Serialization: `SerializableRun.Acts/CurrentActIndex/VisitedMapCoords/MapPointHistory/EventsSeen/PreFinishedRoom` and `SerializableActMap.Points/StartMapPointCoords/GridWidth/GridHeight` (loaded through `RunManager.SavedMapsToLoad`).
- Ordering: `Acts`, `VisitedMapCoords`, `MapPointHistory`, and `_currentRooms` are lists and keep host order; `ActMap.startMapPoints`, `MapPoint.parents/Children`, and `_visitedEventIds` are hash sets and are sorted canonically by `MapCoord(col,row)` or `ModelId` before encoding.
- Restore responsibility: Host-owned through the save loader; the mod records `RunLocation`/`MapLocation` and room `Id` for identity only.
- Unknown-value policy: A null `CurrentMapCoord`, a missing `CurrentRoom`, or a room without a resolvable encounter or event identity rejects capture; no earlier floor is substituted.

| Closure item | Host member | Kind | Declared type | Visibility | Status |
| --- | --- | --- | --- | --- | --- |
| act | `Runs.RunState.Acts` | property | `IReadOnlyList<Models.ActModel>` | public | metadata-observed |
| act | `Runs.RunState.CurrentActIndex` | property | `Int32` | public | metadata-observed |
| act | `Runs.RunState.Act` | property | `Models.ActModel` | public | metadata-observed |
| floor | `Runs.RunState.ActFloor` | property | `Int32` | public | metadata-observed |
| floor | `Runs.RunState.TotalFloor` | property | `Int32` | public | metadata-observed |
| map topology | `Runs.RunState.Map` | property | `Map.ActMap` | public | metadata-observed |
| map topology | `Map.ActMap.Grid` | property | `Map.MapPoint[,]` | family | metadata-observed |
| map topology | `Map.ActMap.StartingMapPoint` | property | `Map.MapPoint` | public | metadata-observed |
| map topology | `Map.ActMap.BossMapPoint` | property | `Map.MapPoint` | public | metadata-observed |
| map topology | `Map.ActMap.SecondBossMapPoint` | property | `Map.MapPoint` | public | metadata-observed |
| map topology | `Map.ActMap.startMapPoints` | field | `HashSet<Map.MapPoint>` | public | metadata-observed |
| map topology | `Map.MapPoint.coord` | field | `Map.MapCoord` | public | metadata-observed |
| map topology | `Map.MapPoint.parents` | field | `HashSet<Map.MapPoint>` | public | metadata-observed |
| map topology | `Map.MapPoint.Children` | property | `HashSet<Map.MapPoint>` | public | metadata-observed |
| map topology | `Map.MapPoint.PointType` | property | `Map.MapPointType` | public | metadata-observed |
| map topology | `Map.MapPoint.Quests` | property | `IReadOnlyList<Models.AbstractModel>` | public | metadata-observed |
| map topology | `Map.MapCoord.col` | field | `Int32` | public | metadata-observed |
| map topology | `Map.MapCoord.row` | field | `Int32` | public | metadata-observed |
| map topology | `Map.StandardActMap._rng` | field | `Random.Rng` | private | metadata-observed |
| map progress | `Runs.RunState.VisitedMapCoords` | property | `IReadOnlyList<Map.MapCoord>` | public | metadata-observed |
| map progress | `Runs.RunState.CurrentMapCoord` | property | `Nullable<Map.MapCoord>` | public | metadata-observed |
| map progress | `Runs.RunState.CurrentMapPoint` | property | `Map.MapPoint` | public | metadata-observed |
| map progress | `Runs.RunState.MapPointHistory` | property | `IReadOnlyList<IReadOnlyList<Runs.History.MapPointHistoryEntry>>` | public | metadata-observed |
| map progress | `Runs.RunState.MapLocation` | property | `Runs.MapLocation` | public | metadata-observed |
| map progress | `Runs.MapLocation.actIndex` | field | `Int32` | public | metadata-observed |
| map progress | `Runs.MapLocation.coord` | field | `Nullable<Map.MapCoord>` | public | metadata-observed |
| room and encounter identity | `Runs.RunState.CurrentRoom` | property | `Rooms.AbstractRoom` | public | metadata-observed |
| room and encounter identity | `Runs.RunState.NextRoomId` | property | `Int32` | public | metadata-observed |
| room and encounter identity | `Runs.RunState.CurrentRoomCount` | property | `Int32` | public | metadata-observed |
| room and encounter identity | `Runs.RunState._currentRooms` | field | `List<Rooms.AbstractRoom>` | private | metadata-observed |
| room and encounter identity | `Runs.RunState.RunLocation` | property | `Runs.RunLocation` | public | metadata-observed |
| room and encounter identity | `Runs.RunLocation.roomId` | field | `Nullable<Int32>` | public | metadata-observed |
| room and encounter identity | `Runs.RunState.VisitedEventIds` | property | `IReadOnlySet<Models.ModelId>` | public | metadata-observed |
| room and encounter identity | `Rooms.AbstractRoom.Id` | property | `Nullable<Int32>` | public | metadata-observed |
| room and encounter identity | `Rooms.AbstractRoom.RoomType` | property | `Rooms.RoomType` | public | metadata-observed |
| room and encounter identity | `Rooms.AbstractRoom.ModelId` | property | `Models.ModelId` | public | metadata-observed |
| room and encounter identity | `Rooms.AbstractRoom.IsPreFinished` | property | `Boolean` | public | metadata-observed |
| room and encounter identity | `Rooms.CombatRoom.Encounter` | property | `Models.EncounterModel` | public | metadata-observed |
| room and encounter identity | `Rooms.CombatRoom.ParentEventId` | property | `Models.ModelId` | public | metadata-observed |
| serialized form | `Saves.SerializableRun.Acts` | property | `List<Saves.Runs.SerializableActModel>` | public | metadata-observed |
| serialized form | `Saves.SerializableRun.CurrentActIndex` | property | `Int32` | public | metadata-observed |
| serialized form | `Saves.SerializableRun.VisitedMapCoords` | property | `List<Map.MapCoord>` | public | metadata-observed |
| serialized form | `Saves.SerializableRun.MapPointHistory` | property | `List<List<Runs.History.MapPointHistoryEntry>>` | public | metadata-observed |
| serialized form | `Saves.SerializableRun.EventsSeen` | property | `List<Models.ModelId>` | public | metadata-observed |
| serialized form | `Saves.SerializableRun.PreFinishedRoom` | property | `Saves.Runs.SerializableRoom` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializableActMap.Points` | property | `List<Saves.Runs.SerializableMapPoint>` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializableActMap.StartMapPointCoords` | property | `List<Map.MapCoord>` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializableActMap.GridWidth` | property | `Int32` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializableActMap.GridHeight` | property | `Int32` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializableRoom.EncounterId` | property | `Models.ModelId` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializableRoom.EventId` | property | `Models.ModelId` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializableRoom.EncounterState` | property | `Dictionary<String,String>` | public | metadata-observed |
| serialized form | `Runs.RunManager.SavedMapsToLoad` | property | `Dictionary<Int32,Saves.Runs.SerializableActMap>` | public | metadata-observed |

## Player (`player`)

- Metadata status: `metadata-observed`; semantics: `runtime-unverified`
- Serialization: `SerializablePlayer` carries character, hp, gold, energy, potion-slot count, `Deck` as `List<SerializableCard>` (`Id/CurrentUpgradeLevel/Enchantment/Props/FloorAddedToDeck`), `Relics`, `Potions`, and `RelicGrabBag`; combat piles are not part of the host run save.
- Ordering: `CardPile._cards`, `_relics`, `_potionSlots`, `_runPiles`, and the `RelicGrabBag._deques` values are lists whose order is gameplay-affecting and is retained; `CardModel._keywords/_tags` are hash sets and are sorted canonically.
- Restore responsibility: Host-owned; the mod never constructs a card, relic, or potion (ADR 0042).
- Unknown-value policy: A card without a resolvable `Pile`, `Owner`, or upgrade level, or a relic without `Status`, rejects capture; unreadable temporary card values are never encoded as defaults. No key-style resource member exists on this build (`metadata-absent`), so the ADR 0037 `keys` item has no host closure.

| Closure item | Host member | Kind | Declared type | Visibility | Status |
| --- | --- | --- | --- | --- | --- |
| health | `Entities.Players.Player.Creature` | property | `Entities.Creatures.Creature` | public | metadata-observed |
| health | `Entities.Creatures.Creature.CurrentHp` | property | `Int32` | public | metadata-observed |
| health | `Entities.Creatures.Creature.MaxHp` | property | `Int32` | public | metadata-observed |
| health | `Entities.Creatures.Creature.Block` | property | `Int32` | public | metadata-observed |
| gold | `Entities.Players.Player.Gold` | property | `Int32` | public | metadata-observed |
| keys | `Entities.Players.Player.Key` | no-member-containing | - | - | metadata-absent (no field, property, or method name contains the text) |
| keys | `Saves.Runs.SerializablePlayer.Key` | no-member-containing | - | - | metadata-absent (no field, property, or method name contains the text) |
| deck instances | `Entities.Players.Player.Deck` | property | `Entities.Cards.CardPile` | public | metadata-observed |
| deck instances | `Entities.Cards.CardPile._cards` | field | `List<Models.CardModel>` | private | metadata-observed |
| deck instances | `Entities.Cards.CardPile.Cards` | property | `IReadOnlyList<Models.CardModel>` | public | metadata-observed |
| deck instances | `Entities.Cards.CardPile.Type` | property | `Entities.Cards.PileType` | public | metadata-observed |
| deck instances | `Runs.RunState._allCards` | field | `List<Models.CardModel>` | private | metadata-observed |
| deck instances | `Models.CardModel.Owner` | property | `Entities.Players.Player` | public | metadata-observed |
| deck instances | `Models.CardModel.Pile` | property | `Entities.Cards.CardPile` | public | metadata-observed |
| deck instances | `Models.CardModel.CloneOf` | property | `Models.CardModel` | public | metadata-observed |
| deck instances | `Models.CardModel.IsDupe` | property | `Boolean` | public | metadata-observed |
| deck instances | `Models.CardModel.FloorAddedToDeck` | property | `Nullable<Int32>` | public | metadata-observed |
| upgrades | `Models.CardModel.CurrentUpgradeLevel` | property | `Int32` | public | metadata-observed |
| upgrades | `Models.CardModel.Enchantment` | property | `Models.EnchantmentModel` | public | metadata-observed |
| upgrades | `Models.CardModel.Affliction` | property | `Models.AfflictionModel` | public | metadata-observed |
| temporary values | `Models.CardModel.EnergyCost` | property | `Entities.Cards.CardEnergyCost` | public | metadata-observed |
| temporary values | `Models.CardModel.CanonicalEnergyCost` | property | `Int32` | family | metadata-observed |
| temporary values | `Models.CardModel._energyCost` | field | `Entities.Cards.CardEnergyCost` | private | metadata-observed |
| temporary values | `Models.CardModel._temporaryStarCosts` | field | `List<Entities.Cards.TemporaryCardCost>` | private | metadata-observed |
| temporary values | `Models.CardModel.CurrentStarCost` | property | `Int32` | public | metadata-observed |
| temporary values | `Models.CardModel.DynamicVars` | property | `Localization.DynamicVars.DynamicVarSet` | public | metadata-observed |
| temporary values | `Models.CardModel._exhaustOnNextPlay` | field | `Boolean` | private | metadata-observed |
| temporary values | `Models.CardModel._hasSingleTurnRetain` | field | `Boolean` | private | metadata-observed |
| temporary values | `Models.CardModel._hasSingleTurnSly` | field | `Boolean` | private | metadata-observed |
| temporary values | `Models.CardModel._keywords` | field | `HashSet<Entities.Cards.CardKeyword>` | private | metadata-observed |
| temporary values | `Models.CardModel._tags` | field | `HashSet<Entities.Cards.CardTag>` | private | metadata-observed |
| ordered piles | `Entities.Players.Player.Piles` | property | `IEnumerable<Entities.Cards.CardPile>` | public | metadata-observed |
| ordered piles | `Entities.Players.Player._runPiles` | field | `Entities.Cards.CardPile[]` | private | metadata-observed |
| ordered piles | `Entities.Players.PlayerCombatState.Hand` | property | `Entities.Cards.CardPile` | public | metadata-observed |
| ordered piles | `Entities.Players.PlayerCombatState.DrawPile` | property | `Entities.Cards.CardPile` | public | metadata-observed |
| ordered piles | `Entities.Players.PlayerCombatState.DiscardPile` | property | `Entities.Cards.CardPile` | public | metadata-observed |
| ordered piles | `Entities.Players.PlayerCombatState.ExhaustPile` | property | `Entities.Cards.CardPile` | public | metadata-observed |
| ordered piles | `Entities.Players.PlayerCombatState.PlayPile` | property | `Entities.Cards.CardPile` | public | metadata-observed |
| ordered piles | `Entities.Players.PlayerCombatState.AllPiles` | property | `IReadOnlyList<Entities.Cards.CardPile>` | public | metadata-observed |
| ordered piles | `Entities.Players.PlayerCombatState._piles` | field | `Entities.Cards.CardPile[]` | private | metadata-observed |
| relics | `Entities.Players.Player.Relics` | property | `IReadOnlyList<Models.RelicModel>` | public | metadata-observed |
| relics | `Entities.Players.Player._relics` | field | `List<Models.RelicModel>` | private | metadata-observed |
| relics | `Models.RelicModel.StackCount` | property | `Int32` | public | metadata-observed |
| relics | `Models.RelicModel.Status` | property | `Entities.Relics.RelicStatus` | public | metadata-observed |
| relics | `Models.RelicModel.DynamicVars` | property | `Localization.DynamicVars.DynamicVarSet` | public | metadata-observed |
| relics | `Models.RelicModel._isWax` | field | `Boolean` | private | metadata-observed |
| relics | `Models.RelicModel._isMelted` | field | `Boolean` | private | metadata-observed |
| relics | `Models.RelicModel.FloorAddedToDeck` | property | `Int32` | public | metadata-observed |
| relic grab bag (hidden future relics) | `Entities.Players.Player.RelicGrabBag` | property | `Runs.RelicGrabBag` | public | metadata-observed |
| relic grab bag (hidden future relics) | `Runs.RunState.SharedRelicGrabBag` | property | `Runs.RelicGrabBag` | public | metadata-observed |
| relic grab bag (hidden future relics) | `Runs.RelicGrabBag._deques` | field | `Dictionary<Entities.Relics.RelicRarity,List<Models.RelicModel>>` | private | metadata-observed |
| relic grab bag (hidden future relics) | `Runs.RelicGrabBag._rarities` | field | `HashSet<Entities.Relics.RelicRarity>` | private | metadata-observed |
| relic grab bag (hidden future relics) | `Runs.RelicGrabBag._originalRelics` | field | `List<Models.RelicModel>` | private | metadata-observed |
| relic grab bag (hidden future relics) | `Runs.RelicGrabBag._refreshAllowed` | field | `Boolean` | private | metadata-observed |
| potions | `Entities.Players.Player.PotionSlots` | property | `IReadOnlyList<Models.PotionModel>` | public | metadata-observed |
| potions | `Entities.Players.Player._potionSlots` | field | `List<Models.PotionModel>` | private | metadata-observed |
| potions | `Entities.Players.Player.MaxPotionCount` | property | `Int32` | public | metadata-observed |
| potions | `Entities.Players.Player.CanRemovePotions` | property | `Boolean` | public | metadata-observed |
| potions | `Models.PotionModel.IsQueued` | property | `Boolean` | public | metadata-observed |
| potions | `Models.PotionModel.DynamicVars` | property | `Localization.DynamicVars.DynamicVarSet` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializablePlayer.CharacterId` | property | `Models.ModelId` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializablePlayer.CurrentHp` | property | `Int32` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializablePlayer.MaxHp` | property | `Int32` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializablePlayer.Gold` | property | `Int32` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializablePlayer.MaxEnergy` | property | `Int32` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializablePlayer.MaxPotionSlotCount` | property | `Int32` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializablePlayer.Deck` | property | `List<Saves.Runs.SerializableCard>` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializablePlayer.Relics` | property | `List<Saves.Runs.SerializableRelic>` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializablePlayer.Potions` | property | `List<Saves.Runs.SerializablePotion>` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializablePlayer.RelicGrabBag` | property | `Saves.Runs.SerializableRelicGrabBag` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializableCard.Id` | property | `Models.ModelId` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializableCard.CurrentUpgradeLevel` | property | `Int32` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializableCard.Enchantment` | property | `Saves.Runs.SerializableEnchantment` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializableCard.Props` | property | `Saves.Runs.SavedProperties` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializableCard.FloorAddedToDeck` | property | `Nullable<Int32>` | public | metadata-observed |

## Combat (`combat`)

- Metadata status: `metadata-observed`; semantics: `runtime-unverified`
- Serialization: Combat state is not part of the single-player host run save (`SerializableRoom.EncounterState` is a string map and `NetFullCombatState` is a multiplayer carrier); a combat checkpoint must serialize the owned values listed here itself.
- Ordering: `Allies/Enemies/Creatures`, the combat piles, `_powers`, `_orbs`, `_pets`, and `StateLog` are lists and keep host order; `_playersReadyToEndTurn` is a hash set.
- Restore responsibility: No host-owned combat restore path is observed; ADR 0042 stays `no_restore_adapter`.
- Unknown-value policy: Capture is refused unless `CombatManager.IsInProgress` holds with `PlayerTurnPhase.Play`, an empty `ActionQueueSet`, `ActionExecutor.IsRunning` false, and no action in `GatheringPlayerChoice`; a `PowerModel._internalData` (`Object`) that cannot be typed rejects capture.

| Closure item | Host member | Kind | Declared type | Visibility | Status |
| --- | --- | --- | --- | --- | --- |
| turn/phase | `Entities.Players.PlayerCombatState.TurnNumber` | property | `Int32` | public | metadata-observed |
| turn/phase | `Entities.Players.PlayerCombatState.Phase` | property | `Combat.PlayerTurnPhase` | public | metadata-observed |
| turn/phase | `Combat.PlayerTurnPhase.Play` | field | `Combat.PlayerTurnPhase` | public | metadata-observed |
| turn/phase | `Combat.PlayerTurnPhase.End` | field | `Combat.PlayerTurnPhase` | public | metadata-observed |
| turn/phase | `Combat.CombatState.RoundNumber` | property | `Int32` | public | metadata-observed |
| turn/phase | `Combat.CombatState.CurrentSide` | property | `Combat.CombatSide` | public | metadata-observed |
| turn/phase | `Combat.CombatState.Encounter` | property | `Models.EncounterModel` | public | metadata-observed |
| turn/phase | `Rooms.CombatRoom.CombatState` | property | `Combat.CombatState` | public | metadata-observed |
| turn/phase | `Combat.CombatManager.IsInProgress` | property | `Boolean` | public | metadata-observed |
| turn/phase | `Combat.CombatManager.IsStarting` | property | `Boolean` | public | metadata-observed |
| turn/phase | `Combat.CombatManager.IsEnding` | property | `Boolean` | public | metadata-observed |
| turn/phase | `Combat.CombatManager.IsEnemyTurnStarted` | property | `Boolean` | public | metadata-observed |
| turn/phase | `Combat.CombatManager.EndingPlayerTurnPhaseOne` | property | `Boolean` | public | metadata-observed |
| turn/phase | `Combat.CombatManager.EndingPlayerTurnPhaseTwo` | property | `Boolean` | public | metadata-observed |
| turn/phase | `Combat.CombatManager.IsPaused` | property | `Boolean` | public | metadata-observed |
| turn/phase | `Combat.CombatManager.PlayerActionsDisabled` | property | `Boolean` | public | metadata-observed |
| turn/phase | `Combat.CombatManager._playersReadyToEndTurn` | field | `HashSet<Entities.Players.Player>` | private | metadata-observed |
| turn/phase | `Combat.CombatManager._deferredEndTurnTransition` | field | `Func<Threading.Tasks.Task>` | private | metadata-observed |
| energy | `Entities.Players.PlayerCombatState.Energy` | property | `Int32` | public | metadata-observed |
| energy | `Entities.Players.PlayerCombatState.MaxEnergy` | property | `Int32` | public | metadata-observed |
| energy | `Entities.Players.PlayerCombatState.Stars` | property | `Int32` | public | metadata-observed |
| energy | `Entities.Players.PlayerCombatState.OrbQueue` | property | `Entities.Orbs.OrbQueue` | public | metadata-observed |
| energy | `Entities.Orbs.OrbQueue._orbs` | field | `List<Models.OrbModel>` | private | metadata-observed |
| energy | `Entities.Orbs.OrbQueue.Capacity` | property | `Int32` | public | metadata-observed |
| powers | `Entities.Creatures.Creature.Powers` | property | `IReadOnlyList<Models.PowerModel>` | public | metadata-observed |
| powers | `Entities.Creatures.Creature._powers` | field | `List<Models.PowerModel>` | private | metadata-observed |
| powers | `Models.PowerModel.Amount` | property | `Int32` | public | metadata-observed |
| powers | `Models.PowerModel._amount` | field | `Int32` | private | metadata-observed |
| powers | `Models.PowerModel._amountOnTurnStart` | field | `Int32` | private | metadata-observed |
| powers | `Models.PowerModel._skipNextDurationTick` | field | `Boolean` | private | metadata-observed |
| powers | `Models.PowerModel._internalData` | field | `Object` | private | metadata-observed |
| powers | `Models.PowerModel.Applier` | property | `Entities.Creatures.Creature` | public | metadata-observed |
| powers | `Models.PowerModel.Target` | property | `Entities.Creatures.Creature` | public | metadata-observed |
| enemies/intents | `Combat.CombatState.Enemies` | property | `IReadOnlyList<Entities.Creatures.Creature>` | public | metadata-observed |
| enemies/intents | `Combat.CombatState.Allies` | property | `IReadOnlyList<Entities.Creatures.Creature>` | public | metadata-observed |
| enemies/intents | `Combat.CombatState.EscapedCreatures` | property | `IReadOnlyList<Entities.Creatures.Creature>` | public | metadata-observed |
| enemies/intents | `Combat.CombatState._nextCreatureId` | field | `UInt32` | private | metadata-observed |
| enemies/intents | `Entities.Players.PlayerCombatState.Pets` | property | `IReadOnlyList<Entities.Creatures.Creature>` | public | metadata-observed |
| enemies/intents | `Entities.Creatures.Creature.CombatId` | property | `Nullable<UInt32>` | public | metadata-observed |
| enemies/intents | `Entities.Creatures.Creature.Monster` | property | `Models.MonsterModel` | public | metadata-observed |
| enemies/intents | `Entities.Creatures.Creature.ModelId` | property | `Models.ModelId` | public | metadata-observed |
| enemies/intents | `Entities.Creatures.Creature.Side` | property | `Combat.CombatSide` | public | metadata-observed |
| enemies/intents | `Entities.Creatures.Creature.IsAlive` | property | `Boolean` | public | metadata-observed |
| enemies/intents | `Entities.Creatures.Creature.IsStunned` | property | `Boolean` | public | metadata-observed |
| enemies/intents | `Entities.Creatures.Creature.MonsterMaxHpBeforeModification` | property | `Nullable<Int32>` | public | metadata-observed |
| enemies/intents | `Models.MonsterModel.NextMove` | property | `MonsterMoves.MonsterMoveStateMachine.MoveState` | public | metadata-observed |
| enemies/intents | `Models.MonsterModel.MoveStateMachine` | property | `MonsterMoves.MonsterMoveStateMachine.MonsterMoveStateMachine` | public | metadata-observed |
| enemies/intents | `Models.MonsterModel.IsPerformingMove` | property | `Boolean` | public | metadata-observed |
| enemies/intents | `Models.MonsterModel.SpawnedThisTurn` | property | `Boolean` | public | metadata-observed |
| enemies/intents | `MonsterMoves.MonsterMoveStateMachine.MonsterMoveStateMachine._currentState` | field | `MonsterMoves.MonsterMoveStateMachine.MonsterState` | private | metadata-observed |
| enemies/intents | `MonsterMoves.MonsterMoveStateMachine.MonsterMoveStateMachine._performedFirstMove` | field | `Boolean` | private | metadata-observed |
| enemies/intents | `MonsterMoves.MonsterMoveStateMachine.MonsterMoveStateMachine.StateLog` | property | `List<MonsterMoves.MonsterMoveStateMachine.MonsterState>` | public | metadata-observed |
| enemies/intents | `MonsterMoves.Intents.AbstractIntent` | type | - | public | metadata-observed |
| damage modifiers | `Combat.CombatState.Modifiers` | property | `IReadOnlyList<Models.ModifierModel>` | public | metadata-observed |
| damage modifiers | `Combat.CombatState.BadgeModels` | property | `IReadOnlyList<Models.BadgeModel>` | public | metadata-observed |
| damage modifiers | `Combat.CombatState.MultiplayerScalingModel` | property | `Models.Singleton.MultiplayerScalingModel` | public | metadata-observed |
| damage modifiers | `Rooms.CombatRoom.GoldProportion` | property | `Single` | public | metadata-observed |
| pending effects | `Runs.RunManager.ActionQueueSet` | property | `GameActions.Multiplayer.ActionQueueSet` | public | metadata-observed |
| pending effects | `Runs.RunManager.ActionExecutor` | property | `GameActions.ActionExecutor` | public | metadata-observed |
| pending effects | `GameActions.Multiplayer.ActionQueueSet.IsEmpty` | property | `Boolean` | public | metadata-observed |
| pending effects | `GameActions.Multiplayer.ActionQueueSet.NextActionId` | property | `UInt32` | public | metadata-observed |
| pending effects | `GameActions.Multiplayer.ActionQueueSet._actionQueues` | field | `List<ActionQueue>` | private | metadata-observed |
| pending effects | `GameActions.Multiplayer.ActionQueueSet._actionsWaitingForResumption` | field | `List<ActionWaitingForResumption>` | private | metadata-observed |
| pending effects | `GameActions.ActionExecutor.IsRunning` | property | `Boolean` | public | metadata-observed |
| pending effects | `GameActions.ActionExecutor.IsPaused` | property | `Boolean` | public | metadata-observed |
| pending effects | `GameActions.ActionExecutor.CurrentlyRunningAction` | property | `GameActions.GameAction` | public | metadata-observed |
| pending effects | `GameActions.GameAction.State` | property | `Entities.Actions.GameActionState` | public | metadata-observed |
| pending effects | `GameActions.GameAction.Id` | property | `Nullable<UInt32>` | public | metadata-observed |
| pending effects | `GameActions.GameAction.OwnerId` | property | `UInt64` | public | metadata-observed |
| pending effects | `Entities.Actions.GameActionState.Executing` | field | `Entities.Actions.GameActionState` | public | metadata-observed |
| pending effects | `Entities.Actions.GameActionState.Finished` | field | `Entities.Actions.GameActionState` | public | metadata-observed |
| selection state | `Entities.Actions.GameActionState.GatheringPlayerChoice` | field | `Entities.Actions.GameActionState` | public | metadata-observed |
| selection state | `Runs.RunManager.PlayerChoiceSynchronizer` | property | `GameActions.Multiplayer.PlayerChoiceSynchronizer` | public | metadata-observed |
| selection state | `GameActions.Multiplayer.PlayerChoiceContext._modelStack` | field | `Stack<Models.AbstractModel>` | private | metadata-observed |
| selection state | `Models.CardModel.CurrentTarget` | property | `Entities.Creatures.Creature` | public | metadata-observed |
| selection state | `Models.CardModel.CurrentPlayIndex` | property | `Int32` | public | metadata-observed |
| selection state | `Combat.CombatState._allCards` | field | `List<Models.CardModel>` | private | metadata-observed |
| serialized form | `Saves.Runs.SerializableRoom.EncounterState` | property | `Dictionary<String,String>` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializableRoom.IsPreFinished` | property | `Boolean` | public | metadata-observed |
| serialized form | `Saves.SerializableRun.PreFinishedRoom` | property | `Saves.Runs.SerializableRoom` | public | metadata-observed |
| serialized form | `Entities.Multiplayer.NetFullCombatState` | type | - | public | metadata-observed |

## Non-combat decisions (`non-combat-decisions`)

- Metadata status: `metadata-observed`; semantics: `runtime-unverified`
- Serialization: Offers and pending choices are not fully serialized by the host run save (`SerializableRoom.ExtraRewards/EventId` and `SerializableRun.EventsSeen` only); a checkpoint must copy the offer values listed here.
- Ordering: `RewardsSet.Rewards`, `CardReward.Cards`, `EventModel.CurrentOptions`, the merchant entry lists, and `RestSiteRoom.Options` are lists and keep host order.
- Restore responsibility: No host-owned restore path for a pending offer is observed; these rows stay `unsupported pending inventory` in the boundary matrix.
- Unknown-value policy: An offer with `_isGenerated`/`IsPopulated` false, an unresolved `_rngOverride`, a merchant entry without `Cost`, or a pending `PlayerChoiceContext` rejects capture.

| Closure item | Host member | Kind | Declared type | Visibility | Status |
| --- | --- | --- | --- | --- | --- |
| reward offer | `Rewards.RewardsSet.Rewards` | property | `List<Rewards.Reward>` | public | metadata-observed |
| reward offer | `Rewards.RewardsSet.Id` | property | `Int32` | public | metadata-observed |
| reward offer | `Rewards.RewardsSet.Room` | property | `Rooms.AbstractRoom` | public | metadata-observed |
| reward offer | `Rewards.RewardsSet.DisallowSkipping` | property | `Boolean` | public | metadata-observed |
| reward offer | `Rewards.RewardsSet._isGenerated` | field | `Boolean` | private | metadata-observed |
| reward offer | `Rewards.Reward.SuccessfullySelected` | property | `Boolean` | public | metadata-observed |
| reward offer | `Rewards.Reward.RewardsSetIndex` | property | `Int32` | public | metadata-observed |
| reward offer | `Rewards.Reward._rngOverride` | field | `Random.Rng` | family | metadata-observed |
| card selection offer | `Rewards.CardReward.Cards` | property | `IEnumerable<Models.CardModel>` | public | metadata-observed |
| card selection offer | `Rewards.CardReward.CanReroll` | property | `Boolean` | public | metadata-observed |
| card selection offer | `Rewards.CardReward.CanSkip` | property | `Boolean` | public | metadata-observed |
| card selection offer | `Rewards.CardReward.Options` | property | `Runs.CardCreationOptions` | private | metadata-observed |
| card selection offer | `Rewards.CardReward.RerollOptions` | property | `Runs.CardCreationOptions` | private | metadata-observed |
| card selection offer | `Rewards.CardReward._hasBeenRerolled` | field | `Boolean` | private | metadata-observed |
| card selection offer | `Runs.CardCreationOptions.Source` | property | `Runs.CardCreationSource` | public | metadata-observed |
| card selection offer | `Runs.CardCreationOptions.RngOverride` | property | `Random.Rng` | public | metadata-observed |
| reward offer | `Rooms.CombatRoom.ExtraRewards` | property | `IReadOnlyDictionary<Entities.Players.Player,List<Rewards.Reward>>` | public | metadata-observed |
| reward offer | `Runs.RunManager.RewardsSetSynchronizer` | property | `Multiplayer.Game.RewardsSetSynchronizer` | public | metadata-observed |
| event | `Rooms.EventRoom.CanonicalEvent` | property | `Models.EventModel` | public | metadata-observed |
| event | `Rooms.EventRoom.LocalMutableEvent` | property | `Models.EventModel` | public | metadata-observed |
| event | `Models.EventModel.CurrentOptions` | property | `IReadOnlyList<Events.EventOption>` | public | metadata-observed |
| event | `Models.EventModel.IsFinished` | property | `Boolean` | public | metadata-observed |
| event | `Models.EventModel.Rng` | property | `Random.Rng` | public | metadata-observed |
| event | `Models.EventModel.DynamicVars` | property | `Localization.DynamicVars.DynamicVarSet` | public | metadata-observed |
| event | `Models.EventModel.IsDeterministic` | property | `Boolean` | public | metadata-observed |
| event | `Events.EventOption.TextKey` | property | `String` | public | metadata-observed |
| event | `Events.EventOption.IsLocked` | property | `Boolean` | public | metadata-observed |
| event | `Events.EventOption.IsProceed` | property | `Boolean` | public | metadata-observed |
| event | `Events.EventOption.WasChosen` | property | `Boolean` | public | metadata-observed |
| shop | `Rooms.MerchantRoom.Inventories` | property | `List<Entities.Merchant.MerchantInventory>` | public | metadata-observed |
| shop | `Entities.Merchant.MerchantInventory.CharacterCardEntries` | property | `IReadOnlyList<Entities.Merchant.MerchantCardEntry>` | public | metadata-observed |
| shop | `Entities.Merchant.MerchantInventory.ColorlessCardEntries` | property | `IReadOnlyList<Entities.Merchant.MerchantCardEntry>` | public | metadata-observed |
| shop | `Entities.Merchant.MerchantInventory.RelicEntries` | property | `IReadOnlyList<Entities.Merchant.MerchantRelicEntry>` | public | metadata-observed |
| shop | `Entities.Merchant.MerchantInventory.PotionEntries` | property | `IReadOnlyList<Entities.Merchant.MerchantPotionEntry>` | public | metadata-observed |
| shop | `Entities.Merchant.MerchantInventory.CardRemovalEntry` | property | `Entities.Merchant.MerchantCardRemovalEntry` | public | metadata-observed |
| shop | `Entities.Merchant.MerchantEntry.Cost` | property | `Int32` | public | metadata-observed |
| shop | `Entities.Merchant.MerchantEntry.IsStocked` | property | `Boolean` | public | metadata-observed |
| shop | `Entities.Players.ExtraPlayerFields.CardShopRemovalsUsed` | property | `Int32` | public | metadata-observed |
| rest | `Rooms.RestSiteRoom.Options` | property | `IReadOnlyList<Entities.RestSite.RestSiteOption>` | public | metadata-observed |
| rest | `Entities.RestSite.RestSiteOption.OptionId` | property | `String` | public | metadata-observed |
| rest | `Entities.RestSite.RestSiteOption.IsEnabled` | property | `Boolean` | public | metadata-observed |
| rest | `Entities.RestSite.RestSiteOption.Owner` | property | `Entities.Players.Player` | family | metadata-observed |
| rest | `Runs.RunManager.RestSiteSynchronizer` | property | `Multiplayer.Game.RestSiteSynchronizer` | public | metadata-observed |
| treasure | `Rooms.TreasureRoom` | type | - | public | metadata-observed |
| treasure | `Entities.TreasureRelicPicking.RelicPickingFight.rounds` | field | `List<Entities.TreasureRelicPicking.RelicPickingFightRound>` | public | metadata-observed |
| treasure | `Runs.RunManager.TreasureRoomRelicSynchronizer` | property | `Multiplayer.Game.TreasureRoomRelicSynchronizer` | public | metadata-observed |
| pending choice | `GameActions.Multiplayer.PlayerChoiceSynchronizer` | type | - | public | metadata-observed |
| pending choice | `Entities.Actions.GameActionState.GatheringPlayerChoice` | field | `Entities.Actions.GameActionState` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializableRoom.ExtraRewards` | property | `Dictionary<UInt64,List<Saves.Runs.SerializableReward>>` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializableRoom.EventId` | property | `Models.ModelId` | public | metadata-observed |
| serialized form | `Saves.SerializableRun.EventsSeen` | property | `List<Models.ModelId>` | public | metadata-observed |

## Persistence and identity (`persistence-and-identity`)

- Metadata status: `metadata-observed`; semantics: `runtime-unverified`
- Serialization: `SerializableRun` through `SaveManager.SaveRun/LoadRunSave` (System.Text.Json via `MegaCritSerializerContext`); canonical checkpoint bytes are mod-owned `asc-jcs-state-v1` (ADR 0054), not a host format.
- Ordering: The identity tuple is fixed by the mod (ADR 0055); host lists keep order and host hash sets are sorted canonically before encoding.
- Restore responsibility: Restore stays unsupported (ADR 0042); duplicate admission and the durable-versus-in-memory distinction are mod-owned (ADR 0055).
- Unknown-value policy: A missing `RunManager.State`, an uninitialized profile, `IsAbandoned` or `IsGameOver` true, or a pending `CurrentRunSaveTask` rejects capture.

| Closure item | Host member | Kind | Declared type | Visibility | Status |
| --- | --- | --- | --- | --- | --- |
| run binding | `Runs.RunManager.Instance` | property | `Runs.RunManager` | public | metadata-observed |
| run binding | `Runs.RunManager.State` | property | `Runs.RunState` | private | metadata-observed |
| run binding | `Runs.RunManager.IsInProgress` | property | `Boolean` | public | metadata-observed |
| run binding | `Runs.RunManager.IsGameOver` | property | `Boolean` | public | metadata-observed |
| run binding | `Runs.RunManager.IsAbandoned` | property | `Boolean` | public | metadata-observed |
| run binding | `Runs.RunManager.ShouldSave` | property | `Boolean` | public | metadata-observed |
| profile binding | `Saves.SaveManager.Instance` | property | `Saves.SaveManager` | public | metadata-observed |
| profile binding | `Saves.SaveManager.CurrentProfileId` | property | `Int32` | public | metadata-observed |
| profile binding | `Saves.SaveManager.IsProfileInitialized` | property | `Boolean` | public | metadata-observed |
| profile binding | `Saves.SaveManager.HasRunSave` | property | `Boolean` | public | metadata-observed |
| profile binding | `Saves.SaveManager.CurrentRunSaveTask` | property | `Threading.Tasks.Task` | public | metadata-observed |
| profile binding | `Saves.SaveManager._saveStore` | field | `Saves.ISaveStore` | private | metadata-observed |
| session binding | `Entities.Players.Player.NetId` | property | `UInt64` | public | metadata-observed |
| session binding | `Context.LocalContext.NetId` | property | `Nullable<UInt64>` | public | metadata-observed |
| session binding | `Entities.Multiplayer.RunSessionState.Running` | field | `Entities.Multiplayer.RunSessionState` | public | metadata-observed |
| epoch/profile inputs | `Saves.ProgressState.Epochs` | property | `IReadOnlyList<Saves.SerializableEpoch>` | public | metadata-observed |
| epoch/profile inputs | `Saves.ProgressState.UniqueId` | property | `String` | public | metadata-observed |
| epoch/profile inputs | `Saves.SerializableEpoch.Id` | property | `String` | public | metadata-observed |
| epoch/profile inputs | `Saves.SerializableEpoch.State` | property | `Saves.EpochState` | public | metadata-observed |
| epoch/profile inputs | `Saves.EpochState.Obtained` | field | `Saves.EpochState` | public | metadata-observed |
| saved closure descriptors | `Saves.SaveManager.SaveRun` | method | `Threading.Tasks.Task (Rooms.AbstractRoom, Boolean)` | public | metadata-observed (overloads=1) |
| saved closure descriptors | `Saves.SaveManager.LoadRunSave` | method | `Saves.ReadSaveResult<Saves.SerializableRun> ()` | public | metadata-observed (overloads=1) |
| saved closure descriptors | `Saves.SaveManager.IncrementNumReloads` | method | `Threading.Tasks.Task (Saves.SerializableRun, Boolean)` | public | metadata-observed (overloads=1) |
| saved closure descriptors | `Saves.SerializableRun.SchemaVersion` | property | `Int32` | public | metadata-observed |
| saved closure descriptors | `Saves.SerializableRun.Players` | property | `List<Saves.Runs.SerializablePlayer>` | public | metadata-observed |
| saved closure descriptors | `Saves.SerializableRun.SerializableRng` | property | `Saves.Runs.SerializableRunRngSet` | public | metadata-observed |
| saved closure descriptors | `Saves.SerializableRun.SerializableOdds` | property | `Saves.Runs.SerializableRunOddsSet` | public | metadata-observed |
| saved closure descriptors | `Saves.SerializableRun.SerializableSharedRelicGrabBag` | property | `Saves.Runs.SerializableRelicGrabBag` | public | metadata-observed |
| saved closure descriptors | `Saves.SerializableRun.SaveTime` | property | `Int64` | public | metadata-observed |
| saved closure descriptors | `Saves.SerializableRun.StartTime` | property | `Int64` | public | metadata-observed |
| saved closure descriptors | `Saves.SerializableRun.NumReloads` | property | `Int32` | public | metadata-observed |
| saved closure descriptors | `Saves.SerializableRun.GameMode` | property | `Runs.GameMode` | public | metadata-observed |
| saved closure descriptors | `Saves.SerializableRun.Ascension` | property | `Int32` | public | metadata-observed |
| ordered canonical bytes (host JSON only; canonical form is mod-owned) | `Saves.MegaCritSerializerContext` | type | - | notpublic | metadata-observed |
| ordered canonical bytes (host JSON only; canonical form is mod-owned) | `Saves.JsonSerializationUtility` | type | - | public | metadata-observed |
