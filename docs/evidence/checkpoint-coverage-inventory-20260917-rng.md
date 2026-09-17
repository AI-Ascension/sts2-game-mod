# Exact-build seed and RNG stream inventory (ADR 0040, pinned metadata)

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
| Families | 6 in this table; 0 unresolved |

Labels: `metadata-observed` means the named member exists on the hashed assembly with the recorded kind and
declared type; `metadata-absent` means no member of the named type contains the text; `unresolved` means the
row could not be matched exactly and its family fails closed. Serialization, ordering, restore, and
unknown-value statements below are policy derived from the observed shapes and stay `runtime-unverified`
until an authorized exact-host run records them. Nothing here is native-verified.

## Master seed (`rng-master-seed`)

- Metadata status: `metadata-observed`; semantics: `runtime-unverified`
- Serialization: `SerializableRunRngSet.Seed` (`String`) beside `RunRngSet.Seed` (`UInt32`); derivation runs through `RunRngSet..ctor(String)` and `MegaRandom.Splitmix64` (algorithm details are not recorded here).
- Ordering: Single value.
- Restore responsibility: Host-owned via `RunRngSet.LoadFromSerializable` and `Rng.FastForwardCounter`; the mod never writes a cursor.
- Unknown-value policy: An unreadable seed returns an explicit unavailable capability.

| Closure item | Host member | Kind | Declared type | Visibility | Status |
| --- | --- | --- | --- | --- | --- |
| canonical seed | `Runs.RunRngSet.StringSeed` | property | `String` | public | metadata-observed |
| canonical seed | `Runs.RunRngSet.Seed` | property | `UInt32` | public | metadata-observed |
| derivation | `Runs.RunRngSet..ctor` | method | `Void (String)` | public | metadata-observed (overloads=1) |
| derivation | `Random.Rng..ctor` | method | `Void (Entities.Players.Player, Models.ModelId, UInt32, Int32) ; Void (UInt32, Int32) ; Void (UInt32, String)` | public | metadata-observed (overloads=3) |
| derivation | `Random.MegaRandom.Splitmix64` | method | `UInt64 (UInt64&)` | public | metadata-observed (overloads=1) |
| serialized form | `Saves.Runs.SerializableRunRngSet.Seed` | property | `String` | public | metadata-observed |

## Map/act generation (`rng-map-act-generation`)

- Metadata status: `metadata-observed`; semantics: `runtime-unverified`
- Serialization: One `Int32` counter per stream in `SerializableRunRngSet.Counters` / `SerializablePlayerRngSet.Counters`; the xoshiro state words are not serialized by the host.
- Ordering: Streams are keyed by `RunRngType`/`PlayerRngType`; canonical order is the enum ordinal.
- Restore responsibility: Host-owned via `RunRngSet.LoadFromSerializable` and `Rng.FastForwardCounter`; the mod never writes a cursor.
- Unknown-value policy: A stream that cannot be read at the boundary fails closed for that build/mode; consumption timing is not certified by metadata.

| Closure item | Host member | Kind | Declared type | Visibility | Status |
| --- | --- | --- | --- | --- | --- |
| stream | `Runs.RunRngSet.UpFront` | property | `Random.Rng` | public | metadata-observed |
| stream | `Runs.RunRngSet.UnknownMapPoint` | property | `Random.Rng` | public | metadata-observed |
| stream identity | `Entities.Rngs.RunRngType.UpFront` | field | `Entities.Rngs.RunRngType` | public | metadata-observed |
| stream identity | `Entities.Rngs.RunRngType.UnknownMapPoint` | field | `Entities.Rngs.RunRngType` | public | metadata-observed |
| first-use boundary (map build) | `Map.StandardActMap._rng` | field | `Random.Rng` | private | metadata-observed |
| odds input | `Runs.RunState.Odds` | property | `Odds.RunOddsSet` | public | metadata-observed |
| odds input | `Odds.RunOddsSet.UnknownMapPoint` | property | `Odds.UnknownMapPointOdds` | public | metadata-observed |
| odds input | `Odds.UnknownMapPointOdds.Roll` | method | `Rooms.RoomType (IEnumerable<Rooms.RoomType>, Runs.IRunState)` | public | metadata-observed (overloads=1) |
| odds input | `Odds.UnknownMapPointOdds._nonEventOdds` | field | `Dictionary<Rooms.RoomType,Single>` | private | metadata-observed |
| serialized form | `Saves.SerializableRun.SerializableOdds` | property | `Saves.Runs.SerializableRunOddsSet` | public | metadata-observed |

## Encounters/enemies/targeting (`rng-encounters-enemies-targeting`)

- Metadata status: `metadata-observed`; semantics: `runtime-unverified`
- Serialization: One `Int32` counter per stream in `SerializableRunRngSet.Counters` / `SerializablePlayerRngSet.Counters`; the xoshiro state words are not serialized by the host.
- Ordering: Streams are keyed by `RunRngType`/`PlayerRngType`; canonical order is the enum ordinal.
- Restore responsibility: Host-owned via `RunRngSet.LoadFromSerializable` and `Rng.FastForwardCounter`; the mod never writes a cursor.
- Unknown-value policy: A stream that cannot be read at the boundary fails closed for that build/mode; consumption timing is not certified by metadata.

| Closure item | Host member | Kind | Declared type | Visibility | Status |
| --- | --- | --- | --- | --- | --- |
| stream | `Runs.RunRngSet.MonsterAi` | property | `Random.Rng` | public | metadata-observed |
| stream | `Runs.RunRngSet.CombatTargets` | property | `Random.Rng` | public | metadata-observed |
| stream identity | `Entities.Rngs.RunRngType.MonsterAi` | field | `Entities.Rngs.RunRngType` | public | metadata-observed |
| stream identity | `Entities.Rngs.RunRngType.CombatTargets` | field | `Entities.Rngs.RunRngType` | public | metadata-observed |
| consumer | `Models.MonsterModel._rng` | field | `Random.Rng` | private | metadata-observed |
| consumer | `Models.MonsterModel._runRng` | field | `Runs.RunRngSet` | private | metadata-observed |
| consumer | `MonsterMoves.MonsterMoveStateMachine.RandomBranchState` | type | - | public | metadata-observed |

## Shuffle/draw/rewards/shops/events (`rng-shuffle-draw-rewards-shops-events`)

- Metadata status: `metadata-observed`; semantics: `runtime-unverified`
- Serialization: One `Int32` counter per stream in `SerializableRunRngSet.Counters` / `SerializablePlayerRngSet.Counters`; the xoshiro state words are not serialized by the host. Note: the enum member is `RunRngType.CombatOrbs` while the accessor is `RunRngSet.CombatOrbGeneration`.
- Ordering: Streams are keyed by `RunRngType`/`PlayerRngType`; canonical order is the enum ordinal.
- Restore responsibility: Host-owned via `RunRngSet.LoadFromSerializable` and `Rng.FastForwardCounter`; the mod never writes a cursor.
- Unknown-value policy: A stream that cannot be read at the boundary fails closed for that build/mode; consumption timing is not certified by metadata. A `Reward._rngOverride` or `CardCreationOptions.RngOverride` that is set makes the offer's stream identity explicit; an unresolvable override rejects capture.

| Closure item | Host member | Kind | Declared type | Visibility | Status |
| --- | --- | --- | --- | --- | --- |
| stream | `Runs.RunRngSet.Shuffle` | property | `Random.Rng` | public | metadata-observed |
| stream | `Runs.RunRngSet.CombatCardGeneration` | property | `Random.Rng` | public | metadata-observed |
| stream | `Runs.RunRngSet.CombatPotionGeneration` | property | `Random.Rng` | public | metadata-observed |
| stream | `Runs.RunRngSet.CombatCardSelection` | property | `Random.Rng` | public | metadata-observed |
| stream | `Runs.RunRngSet.CombatEnergyCosts` | property | `Random.Rng` | public | metadata-observed |
| stream | `Runs.RunRngSet.CombatOrbGeneration` | property | `Random.Rng` | public | metadata-observed |
| stream | `Runs.RunRngSet.Niche` | property | `Random.Rng` | public | metadata-observed |
| stream | `Runs.RunRngSet.TreasureRoomRelics` | property | `Random.Rng` | public | metadata-observed |
| stream identity | `Entities.Rngs.RunRngType.Shuffle` | field | `Entities.Rngs.RunRngType` | public | metadata-observed |
| stream identity | `Entities.Rngs.RunRngType.CombatOrbs` | field | `Entities.Rngs.RunRngType` | public | metadata-observed |
| stream identity | `Entities.Rngs.RunRngType.Niche` | field | `Entities.Rngs.RunRngType` | public | metadata-observed |
| per-player stream | `Random.PlayerRngSet.Rewards` | property | `Random.Rng` | public | metadata-observed |
| per-player stream | `Random.PlayerRngSet.Shops` | property | `Random.Rng` | public | metadata-observed |
| per-player stream | `Random.PlayerRngSet.Transformations` | property | `Random.Rng` | public | metadata-observed |
| per-player stream identity | `Entities.Rngs.PlayerRngType.Rewards` | field | `Entities.Rngs.PlayerRngType` | public | metadata-observed |
| per-player stream identity | `Entities.Rngs.PlayerRngType.Shops` | field | `Entities.Rngs.PlayerRngType` | public | metadata-observed |
| per-player stream identity | `Entities.Rngs.PlayerRngType.Transformations` | field | `Entities.Rngs.PlayerRngType` | public | metadata-observed |
| offer-scoped override | `Rewards.Reward._rngOverride` | field | `Random.Rng` | family | metadata-observed |
| offer-scoped override | `Runs.CardCreationOptions.RngOverride` | property | `Random.Rng` | public | metadata-observed |
| offer-scoped override | `Models.EventModel.Rng` | property | `Random.Rng` | public | metadata-observed |
| debug override | `Combat.CombatManager.DebugForcedTopCardOnNextShuffle` | property | `Models.CardModel` | public | metadata-observed |
| serialized form | `Saves.SerializablePlayerRngSet.Counters` | property | `Dictionary<Entities.Rngs.PlayerRngType,Int32>` | public | metadata-observed |
| serialized form | `Saves.Runs.SerializableRunRngSet.Counters` | property | `Dictionary<Entities.Rngs.RunRngType,Int32>` | public | metadata-observed |

## Ordering and external entropy (`rng-ordering-external-entropy`)

- Metadata status: `metadata-observed`; semantics: `runtime-unverified`
- Serialization: Wall-clock values are saved as `Int64` (`SaveTime/StartTime/RunTime/WinTime`, `DailyTime`); object IDs are `UInt32`/`Int32` counters; hash-set contents are saved as lists.
- Ordering: Every hash set listed here has no host-defined order and must be sorted canonically before encoding; task and cancellation members are timing inputs, not state.
- Restore responsibility: Not restorable and not part of identity except the daily seed date; the mod records them as declared external inputs.
- Unknown-value policy: A checkpoint taken while `_actionCancelToken`, `_executionTask`, or `_deferredEndTurnTransition` is live is refused as `busy`; an unknown ordering source fails closed.

| Closure item | Host member | Kind | Declared type | Visibility | Status |
| --- | --- | --- | --- | --- | --- |
| wall clock | `Runs.RunManager._startTime` | field | `Int64` | private | metadata-observed |
| wall clock | `Runs.RunManager._sessionStartTime` | field | `Int64` | private | metadata-observed |
| wall clock | `Runs.RunManager.DailyTime` | property | `Nullable<DateTimeOffset>` | public | metadata-observed |
| wall clock | `Runs.RunManager.RunTime` | property | `Int64` | public | metadata-observed |
| wall clock | `Saves.SerializableRun.SaveTime` | property | `Int64` | public | metadata-observed |
| wall clock | `Saves.SerializableRun.StartTime` | property | `Int64` | public | metadata-observed |
| wall clock | `Saves.SerializableRun.DailyTime` | property | `Nullable<DateTimeOffset>` | public | metadata-observed |
| object IDs | `Combat.CombatState._nextCreatureId` | field | `UInt32` | private | metadata-observed |
| object IDs | `GameActions.Multiplayer.ActionQueueSet._nextId` | field | `UInt32` | private | metadata-observed |
| object IDs | `GameActions.Multiplayer.ActionQueueSet.NextActionId` | property | `UInt32` | public | metadata-observed |
| object IDs | `Runs.RunState.NextRoomId` | property | `Int32` | public | metadata-observed |
| object IDs | `Rewards.RewardsSet.Id` | property | `Int32` | public | metadata-observed |
| object IDs | `GameActions.GameAction.Id` | property | `Nullable<UInt32>` | public | metadata-observed |
| hash order | `Runs.RunState._visitedEventIds` | field | `HashSet<Models.ModelId>` | private | metadata-observed |
| hash order | `Map.ActMap.startMapPoints` | field | `HashSet<Map.MapPoint>` | public | metadata-observed |
| hash order | `Map.MapPoint.parents` | field | `HashSet<Map.MapPoint>` | public | metadata-observed |
| hash order | `Map.MapPoint.Children` | property | `HashSet<Map.MapPoint>` | public | metadata-observed |
| hash order | `Unlocks.UnlockState._encountersSeen` | field | `HashSet<Models.ModelId>` | private | metadata-observed |
| hash order | `Runs.RelicGrabBag._rarities` | field | `HashSet<Entities.Relics.RelicRarity>` | private | metadata-observed |
| hash order | `Models.CardModel._keywords` | field | `HashSet<Entities.Cards.CardKeyword>` | private | metadata-observed |
| hash order | `Models.CardModel._tags` | field | `HashSet<Entities.Cards.CardTag>` | private | metadata-observed |
| hash order | `Combat.CombatManager._playersReadyToEndTurn` | field | `HashSet<Entities.Players.Player>` | private | metadata-observed |
| async/frame timing | `GameActions.ActionExecutor._actionCancelToken` | field | `Threading.CancellationTokenSource` | private | metadata-observed |
| async/frame timing | `GameActions.ActionExecutor._queueTaskCompletionSource` | field | `Threading.Tasks.TaskCompletionSource<Boolean>` | private | metadata-observed |
| async/frame timing | `GameActions.GameAction._executionTask` | field | `Threading.Tasks.Task` | private | metadata-observed |
| async/frame timing | `Combat.CombatManager._deferredEndTurnTransition` | field | `Func<Threading.Tasks.Task>` | private | metadata-observed |
| async/frame timing | `Combat.CombatStateTracker._combatStateChangedDeferredTask` | field | `Threading.Tasks.Task` | private | metadata-observed |
| locale | `Localization.LocManager.Language` | property | `String` | public | metadata-observed |
| locale | `Localization.LocManager.CultureInfo` | property | `Globalization.CultureInfo` | public | metadata-observed |
| locale | `Localization.LocManager.StringComparer` | property | `StringComparer` | public | metadata-observed |
| files | `Saves.UserDataPathProvider.SavesDir` | property | `String` | public | metadata-observed |
| files | `Saves.UserDataPathProvider.IsRunningModded` | property | `Boolean` | public | metadata-observed |
| files | `Saves.ISaveStore.ReadFile` | method | `String (String)` | public | metadata-observed (overloads=1) |
| files | `Saves.ISaveStore.GetLastModifiedTime` | method | `DateTimeOffset (String)` | public | metadata-observed (overloads=1) |
| files | `Saves.SerializableRun.PlatformType` | property | `Platform.PlatformType` | public | metadata-observed |
| files | `Saves.SerializableRun.MapDrawings` | property | `Saves.MapDrawing.SerializableMapDrawings` | public | metadata-observed |

## Cosmetic-only randomness (`rng-cosmetic-only`)

- Metadata status: `metadata-observed`; semantics: `runtime-unverified`
- Serialization: Not serialized by the host (`Rng.Chaotic` is a backing property; `MegaRandom..ctor()` is the unseeded overload).
- Ordering: Not ordered; excluded from identity only after a live run proves it cannot affect gameplay state.
- Restore responsibility: Not restorable by design.
- Unknown-value policy: Metadata shows these members exist; it cannot show they are gameplay-neutral, so this row stays `unverified` for the ADR 0040 required finding.

| Closure item | Host member | Kind | Declared type | Visibility | Status |
| --- | --- | --- | --- | --- | --- |
| chaotic stream | `Random.Rng.Chaotic` | property | `Random.Rng` | public | metadata-observed |
| unseeded constructor | `Random.MegaRandom..ctor` | method | `Void () ; Void (UInt64)` | public | metadata-observed (overloads=2) |
| float increment state | `Random.MegaRandom._incrDouble` | field | `Double` | private | metadata-observed |
| float increment state | `Random.MegaRandom._incrFloat` | field | `Single` | private | metadata-observed |
| flavor synchronizer | `Runs.RunManager.FlavorSynchronizer` | property | `Multiplayer.Game.FlavorSynchronizer` | public | metadata-observed |
| flavor synchronizer | `Multiplayer.Game.FlavorSynchronizer` | type | - | public | metadata-observed |
