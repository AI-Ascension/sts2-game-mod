# ADR 0084: Owned live fixed-card-cost inputs

- Status: Accepted source implementation; exact-host and downstream acceptance pending
- Date: 2026-10-22
- Requirement: GameMod #207, upstream dependency of game-core #13

## Problem and scope

The ADR 0077 mapper accepts caller-supplied static metadata. ADR 0083 checks that metadata against
Core's catalog, but neither supplies observations or reads an owner source. The existing retained
card adapter also kept its previous capture after an invalid identity, an enqueue failure, or an
invalidation whose enqueue failed. Retained reads did not recheck the current native run or values.

Implement one production source: the two inputs of Core's `resource.fixed_card_cost` record,
`energy` followed by `card_cost`, both in energy units. The managed owner copies current player
energy and the selected card's resolved energy cost on the host thread. A cost is available only
when the existing optional `HasEnergyCostX` accessor confirms fixedness. Unknown fixedness and
negative sentinels stay unavailable; X cost is explicitly unsupported. Zero is an observed value.
Integers are not narrowed to the compact gameplay projection's byte range.

The producer does not subtract a cost, determine affordability, choose an action, or assert native
parity for modifiers, alternate payment, triggers or rejection order. Evidence is always
`source_derived`. The other fifteen known Core records remain explicit `unsupported` results in
request order; unknown catalog IDs yield `unknown_id`. The other twelve admitted mechanic families
still need authoritative inputs; the entity-interaction family has no admitted Core record.
This is partial delivery: #207 and game-core #13 remain open.

## Owner capture and freshness

`LiveCombatSource` copies values before releasing host objects and uses the existing registry for
occurrence, run-incarnation, snapshot and epoch identities. Native run and profile objects remain
in-thread lifetime handles. No pointer, host object, profile ID or seed enters the facts DTO. The
capture verifies gameplay generation, run/player identity, profile and raw energy before and after
reading cards. It performs no action or game RNG draw.

`LiveCardSourceReadAdapter` clears old retention before replacement reads or invalidation. A failed
invalidation stays pending and retires old handles before the next successful capture. An
incomplete synchronous callback is unavailable. Retained reads recheck profile slot/object, run,
mode, generation, raw energy, occurrence objects and copied fields without allocating a fresh
snapshot or advancing its epoch. Failure removes retention. Raw comparisons catch changes hidden
by the compact gameplay byte representation. The explicit lifecycle invalidation hook still
clears owner handles; future restore owners must call it before exposing restored state.

The authenticated lookup association now retains catalog generation and actual locale from the
typed manifest. Facts recheck caller, session, lease, lease epoch, Harness run, authority epoch,
manifest and native-run association. Wire run identity remains the Harness run, distinct from the
native source's opaque run-incarnation ID. Source scope comes from that retained association.

## Content and Protocol boundary

Queries select an already retained card and parent snapshot. The managed owner captures the
current catalog, and the existing Rust `ContentManifestProducer` derives build, content revisions
and definition membership. The bridge revalidates the live capture after the catalog read and
never refreshes a stale selector. Rust requires current manifest ID, locale and catalog generation
to match the retained binding, and the selected definition to exist in the handled card family.
A catalog reload with the same semantic identity but a different generation refuses old facts.

The canonical Protocol codec admits the entire closed request before managed selector reads:
duplicate keys, correlation, exact instance/snapshot/parent agreement and bounds are checked.
This new path requires live mode and visibility scope `owner`; static requests are unavailable.
The existing generic and Core-admitted static adapters retain their previous behavior.

Rust constructs the actual input inventory and passes it through `CoreFactsInventory` before
mapping observations. Opaque source-reference hashes identify snapshot, occurrence and field,
not paths or parity certificates. Unavailable values stay null with fixed reasons. Canonical
typed refusals contain no host exception text. Public construction of source DTOs remains useful
for tests and is not independent proof of authenticity; the production route owns provenance.

Protocol stays pinned to `1879e0b00993c9b6d662d1f015ef59d6810d82de`; Core stays pinned to
`5070070784d6b62dabf535a230bcdb46a054e3ef`. No schema, digest, version, golden, dependency or
vector changes. The profile is `game-facts-reference-v1`, rules-reference v2, schema digest
`3065a2ff96e6e5af628b3d5cb43b8f2db4232ae4c4dc71324b973419908a2985`.

## Fixed routes and ABI

| Method | Path | Callback | Behavior |
| --- | --- | --- | --- |
| GET | `/api/v1/game-facts/capabilities` | 26 | Empty body; implemented live retention policy |
| POST | `/api/v1/game-facts/query` | 26 | JSON facts-v1 query; one retained owner capture |

The existing bearer-authenticated listener, closed headers and admitted runtime context remain
in force. Callback 26 is read-only and disjoint from existing callbacks. Capabilities describe
implementation, not source readiness: one snapshot, one generation, stale rejection, and
restore/restart/profile/content/run/epoch invalidators. Missing source still refuses a query.
This route grants no lease and does not bypass the Gateway's instance policy.

The additive internal C export `sts2_game_mod_game_facts_handle` uses operation 0 for admission,
1 for capabilities and 2 for production/refusal. Request/response bounds are 262144 bytes; source
copies are at most 17 MiB including a 16 MiB catalog; correlation is at most 128 bytes. Length
checks precede borrowed slices. Failure clears output length, and copying requires caller-owned
capacity. Panic/error details remain internal. ABI structure, version 2 and existing signatures
remain unchanged.

## Evidence and remaining acceptance

The managed probe links actual production capture, retention, association, facts and buffer
bridges to original controlled host shapes, then loads the actual compiled Rust library and
manifest producer. Rust tests cover actual Core order, availability, all known rules, content and
scope fences, bounds and authenticated fixed route dispatch. The probe runs in the existing game
information conformance workflow; see [commands and evidence scope](../OWNED_LIVE_FACTS_TESTING.md).

These checks establish executable source boundaries only. They do not qualify installed-game
property shapes, native profile/restore lifecycle, catalog counts, getter side effects or cost
parity. Exact proprietary assemblies and a disposable native installation remain required.
Gateway, MCP and Harness adoption/replay are separate in-org work and must preserve the Gateway
path. No downstream tool, validation authority or bypass is added here.
