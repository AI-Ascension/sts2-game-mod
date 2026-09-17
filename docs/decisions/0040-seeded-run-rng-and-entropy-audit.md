# ADR 0040: seeded-run RNG and entropy audit boundary

- Status: Proposed; stream inventory metadata-observed on the pinned v0.107.1 build (amendment 2026-09-17); lifecycle, consumption, and cosmetic-only findings unverified
- Date: 2026-09-13
- Tracking: game-mod #82

## Context

`RunState.Rng.StringSeed` is visible to the existing repeat-seed controller, but one string seed
does not prove that every future-affecting source is controlled.  The controller deliberately
restarts only a Custom run and does not expose RNG state; ADR 0031's standard adapter likewise
does not authorize direct RNG writes.  Exact deterministic certification requires a separate
native lifecycle audit and cold-launch evidence.

## Decision

The future host-thread audit emits a private, bounded initialization witness at the first stable
seeded boundary.  It is read-only: producing or rereading it must not draw randomness, create a
playable object, mutate a profile, or alter action ordering.  Detailed stream state remains
private and must not enter model observations, public logs, or normal fixtures.

For every discovered future-affecting stream or entropy input, the inventory records its exact
host-build evidence, owner, algorithm/version where observable, master-seed derivation or
independent source, initial state/cursor representation, creation/reset timing, call categories,
serialization availability, and support classification.  Unknown coverage fails closed for that
build/mode rather than certifying determinism.

| Audit category | Required finding | Current evidence |
| --- | --- | --- |
| Master seed | canonical requested/read-back seed and derivation version | source-derived only |
| Map/act generation | stream/input lifecycle and first-use boundary | unverified |
| Encounters/enemies/targeting | stream/input lifecycle and action consumption | unverified |
| Shuffle/draw/rewards/shops/events | stream/input lifecycle and action consumption | unverified |
| Ordering and external entropy | wall clock, GUID/object IDs, hash order, async/frame timing, locale/files | unverified |
| Cosmetic-only randomness | evidence it cannot affect gameplay state | unverified |

The witness binds to the game build, adapter compatibility, supported mode, profile compatibility,
controlled external-input declaration, and content manifest when available.  An unsupported host,
mode, missing stream, changed build, or incoherent first boundary returns an explicit unavailable
capability.  It does not permit an independently chosen sub-seed or arbitrary cursor write.

## Acceptance

Source-only fixtures must reject missing build binding, duplicate stream identity, unknown
coverage, malformed cursor evidence, over-limit witness data, and a changed witness under one
read-only operation.  They must distinguish known-zero state from unavailable state and ensure
the public projection contains no raw stream state.

Authorized exact-host evidence then requires at least three independent cold launches per
supported matrix cell, repeated no-effect reads, controlled legal actions for each advertised
category, and timing/process perturbations.  Any nondeterministic or unknown future-affecting
input yields unsupported capability or a separately scoped owner fix; it cannot be hidden by a
successful `StringSeed` comparison.

## Limits

This decision adds no RNG extractor, route, schema, native certification, or host execution.
Checkpoint capture remains owned by ADR 0037, standard admission by ADR 0031, and cross-owner
delivery remains subject to its accepted contracts and exact-host authorization.

## Amendment 2026-09-17: exact-build stream inventory

Tracking: game-mod #80 item 1 (shared with #82).  This section supersedes the "Current evidence"
column above for the pinned build only; the original table is retained as history.

The opt-in metadata probe `experiments/managed-rust-interop/checkpoint-coverage-reflection/`
resolved every audit category to concrete host members on the exact pinned host (STS2 v0.107.1 /
`59260271`; `sts2.dll` SHA-256 `a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52`)
by reading its metadata tables only; the 88 rows are recorded in
[`checkpoint-coverage-inventory-20260917-rng.md`](../evidence/checkpoint-coverage-inventory-20260917-rng.md).

| Audit category | Status on pinned v0.107.1 | Rows | Required finding |
| --- | --- | ---: | --- |
| Master seed | metadata-observed (`RunRngSet.StringSeed`/`Seed`, `SerializableRunRngSet.Seed`) | 6 | canonical read-back and derivation version: runtime-unverified |
| Map/act generation | metadata-observed (`UpFront`, `UnknownMapPoint`, `StandardActMap._rng`, `UnknownMapPointOdds`) | 10 | lifecycle and first-use boundary: runtime-unverified |
| Encounters/enemies/targeting | metadata-observed (`MonsterAi`, `CombatTargets`, `MonsterModel._rng`, `RandomBranchState`) | 7 | consumption per action: runtime-unverified |
| Shuffle/draw/rewards/shops/events | metadata-observed (8 run streams, 3 player streams, offer-scoped overrides) | 23 | consumption per action: runtime-unverified |
| Ordering and external entropy | metadata-observed (wall clock, ID counters, 9 hash sets, task/cancellation members, locale, files) | 36 | controlled-input declaration: runtime-unverified |
| Cosmetic-only randomness | metadata-observed members (`Rng.Chaotic`, unseeded `MegaRandom`, flavor synchronizer) | 6 | gameplay-neutrality: unverified |

Findings (metadata-observed; semantics unverified): the host persists one `Int32` counter per
stream plus the seed (`SerializableRunRngSet.Counters`, `SerializablePlayerRngSet.Counters`) and
restores through `RunRngSet.LoadFromSerializable` and `Rng.FastForwardCounter`, so the audit
witness must record `(stream, seed, counter)` rather than generator state words; the
`RunRngType.CombatOrbs` enum member is exposed as `RunRngSet.CombatOrbGeneration`; `Reward._rngOverride`,
`CardCreationOptions.RngOverride`, and `EventModel.Rng` are offer-scoped stream bindings that must
be declared per offer; and `CombatManager.DebugForcedTopCardOnNextShuffle` is a debug override that
must be observed absent.  Nothing here certifies determinism, stream lifecycle, or that any stream
is gameplay-neutral; the acceptance, limits, and cold-launch requirements above are unchanged.
