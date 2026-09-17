# ADR 0037: native checkpoint coverage inventory

- Status: Proposed; every coverage family metadata-observed on the pinned v0.107.1 build (amendment 2026-09-17); runtime semantics unverified
- Date: 2026-09-13
- Tracking: game-mod #80

## Context

Restarting a seed is not a checkpoint.  The current repeat-seed controller intentionally starts
again from the beginning, and the existing public observations omit future-affecting state.  A
checkpoint producer must therefore fail closed until it can prove that a supported native decision
boundary has a complete, owned-value restore closure.  Neither a host build nor a serialized
public observation is that proof.

This record is the game-mod-owned inventory and boundary matrix required before a checkpoint
codec or route is proposed.  It does not add a callback, persistence, a public schema, or restore
behavior.  Field names below are coverage categories, not claims about proprietary member names.

## Decision

The future checkpoint producer has one authoritative inventory entry for every category below.
Each entry must record the exact host build and source/assembly evidence, host owner, capture and
restore responsibility, stable ordering, serialization representation, unknown-value policy, and
whether it participates in identity.  An unknown required entry rejects capture with a bounded
`unsupported_coverage` result; it is never substituted with an empty value or an earlier state.

| Coverage family | Required closure | Initial status |
| --- | --- | --- |
| Compatibility | game build, adapter revision, mode, character, ascension, modifiers, profile compatibility | unverified |
| Seed and entropy | master seed, derivation version, every gameplay-affecting RNG state/cursor | unverified |
| Campaign | act, floor, map topology/progress, room and encounter identity | unverified |
| Player | health, gold, keys, deck instances/upgrades/temporary values, ordered piles, relics and potions | unverified |
| Combat | turn/phase, energy, powers, enemies/intents, damage modifiers, pending effects and selection state | unverified |
| Non-combat decisions | reward, event, shop, rest and card-selection offers, costs, eligibility and pending choice | unverified |
| Persistence and identity | run/profile/session/epoch binding, saved closure descriptors, ordered canonical bytes | unverified |

The capture owner runs only on the host game thread after an admitted action and its host
save/effect work have settled.  It obtains immutable owned values, rechecks the boundary before
publication, and transfers only those values across the seam.  Capture must not draw RNG, create
playable objects, change a profile, or advance effects.  A concurrent action, transition,
animation, enemy execution, unresolved selection, missing required field, or unknown ordering
returns an explicit unavailable result.

## Supported-boundary matrix

No native phase is advertised as capture-capable by this ADR.  The following is the acceptance
order, not a runtime capability declaration:

| Boundary | Required quiescence proof | Current disposition |
| --- | --- | --- |
| Settled map choice | map/action state stable; no queued effect or transition | candidate; unverified |
| Stable player-turn combat | legal player action surface stable; no pending damage/effect | candidate; unverified |
| Later-turn combat | same as player turn plus changed RNG/turn witnesses | candidate; unverified |
| Reward/card selection | full offer and pending selector closure captured | unsupported pending inventory |
| Event/shop/rest | full choice, costs, eligibility and pending effects captured | unsupported pending inventory |
| Enemy turn, animation, transition, unknown phase | cannot prove a stable closure | explicitly rejected |

The first native implementation may enable only a matrix row after exact-host evidence proves its
complete closure twice without gameplay effect.  Each enabled row needs a versioned owner schema,
canonical encoder vectors, bounded byte limits, source-only rejection tests, and a separate
authorized exact-host test.  A row that cannot meet those conditions remains unavailable.

## Validation and limits

Before implementation, add deterministic synthetic vectors for ordering, duplicate keys, numeric
edge cases, required-unknown rejection, and size bounds.  Exact-host tests must show same-boundary
repeat capture stability, changed hidden future-affecting state changing identity, and race
rejection rather than mixed capture.  Restore, durable artifact storage, gateway/MCP delivery,
and standard seeded admission remain separate owners and acceptance gates.

No proprietary assembly, save, profile, captured state, install path, or hidden RNG value belongs
in this repository or its normal CI output.  Until authorized exact-host evidence is recorded,
this document is source-derived design inventory only.

## Amendment 2026-09-17: exact-build metadata inventory

Tracking: game-mod #80, required-implementation item 1.  This section supersedes the "Initial
status" column of the coverage table above for the pinned build only; the original table and text
are retained as history and the supported-boundary matrix is unchanged.

The opt-in metadata probe `experiments/managed-rust-interop/checkpoint-coverage-reflection/`
resolved every coverage family below to concrete host members on the exact pinned host
(STS2 v0.107.1 / `59260271`; `sts2.dll` SHA-256
`a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52`, `GodotSharp.dll` SHA-256
`0e4897ecdfb31456a97c7d8028dfb8d7dbdc632e2f73fc9b438d7b266a139289`) by reading its metadata
tables only.  The recorded inventory is
[`checkpoint-coverage-inventory-20260917.md`](../evidence/checkpoint-coverage-inventory-20260917.md)
with its JSON companion and the ADR 0040 table
[`checkpoint-coverage-inventory-20260917-rng.md`](../evidence/checkpoint-coverage-inventory-20260917-rng.md).
Each of the 350 ADR 0037 rows names one host type, member, kind, declared type, and visibility
(72 private members are named, none read); no host value, byte, IL, or install path was recorded,
and the assembly was neither copied nor executed.

| Coverage family | Status on pinned v0.107.1 | Rows | Runtime semantics |
| --- | --- | ---: | --- |
| Compatibility | metadata-observed | 30 | runtime-unverified |
| Seed and entropy | metadata-observed | 25 | runtime-unverified |
| Campaign | metadata-observed | 53 | runtime-unverified |
| Player | metadata-observed (`keys`: metadata-absent) | 75 | runtime-unverified |
| Combat | metadata-observed | 81 | runtime-unverified |
| Non-combat decisions | metadata-observed | 51 | runtime-unverified |
| Persistence and identity | metadata-observed | 35 | runtime-unverified |

`metadata-observed` means only that the named members exist with the recorded kinds and declared
types on that hashed assembly.  Serialization representation, restore responsibility, stable
ordering, and unknown-value behaviour are stated per family in the inventory as policy derived
from the observed shapes; they remain `runtime-unverified` until an authorized exact-host run
records them.  No family, boundary row, or phase is native-verified, and no phase is advertised as
capture-capable.

Findings that constrain the future producer (metadata-observed; semantics unverified):

- RNG cursors: the host serializes one `Int32` counter per stream (`SerializableRunRngSet.Counters`,
  `SerializablePlayerRngSet.Counters`) beside the seed, not the four `UInt64` state words of
  `MegaRandom`.  A checkpoint must carry `(stream, seed, counter)` for the 12 `RunRngType` streams
  and 3 `PlayerRngType` streams and pin the `Rng` constructor variant; `Rng.Chaotic` exists as an
  uncertified stream.
- Hidden future-affecting state includes `RelicGrabBag._deques` (per-rarity relic queues),
  `MonsterModel.NextMove` with `MonsterMoveStateMachine._currentState`, `Reward._rngOverride`, and
  `CardCreationOptions.RngOverride`; checkpoint identity must include them.
- Unordered host containers (`ActMap.startMapPoints`, `MapPoint.parents`/`Children`,
  `RunState._visitedEventIds`, `UnlockState._encountersSeen`, `RelicGrabBag._rarities`,
  `CardModel._keywords`/`_tags`, `CombatManager._playersReadyToEndTurn`) need canonical sorting
  before `asc-jcs-state-v1` encoding.
- Combat and pending-offer state are not part of the host single-player run save
  (`SerializableRoom.EncounterState` is a string map); a combat or offer checkpoint must copy owned
  values itself, which is why those boundary rows stay candidate or unsupported.
- No key-style resource member exists on `Player` or `SerializablePlayer` on this build; the
  `keys` closure item has no host member and is recorded `metadata-absent`.
- `PowerModel._internalData` is an untyped `Object`; an instance whose payload cannot be typed
  rejects capture.

Unknown-value policy, unchanged and explicit: an unknown required entry rejects capture with a
bounded `unsupported_coverage` result; it is never substituted with an empty value, a default, or
an earlier state.  A member that resolves in metadata but cannot be read at the boundary is
unknown for that capture.

The probe's `EveryInventoryFamilyResolvesToPinnedMember` and `UnknownFamilyFailsClosed`
regressions run only with `-p:STS2GameDataDir=<host data>`; hosted CI does not run them and the
host assemblies stay outside the repository.
