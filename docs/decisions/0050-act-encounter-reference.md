# ADR 0050: Owner-local act, encounter, and map-generation reference data

- Status: Proposed; source-only owner boundary
- Date: 2026-09-14
- Tracking: game-mod #97
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md), [ADR 0039](0039-field-availability-and-completeness-boundary.md), [ADR 0049](0049-enemy-definitions.md)

## Context

Runtime-map-v1 exposes current-run topology, not an act/encounter/generation encyclopedia. An agent
cannot inspect act structure, room/node categories, encounter composition, eligibility, weighted
pools, or map-generation rules independently of the displayed map. This decision defines an owned
source boundary. It does not claim a native extractor, a transport route, a gateway/MCP adapter, or
exact-host compatibility.

## Decision

`crates/game-mod/src/act_reference` owns an immutable `ActCatalog` produced by `ActCatalogProducer`
from a bounded `ActCatalogSource`. Static results bind to the existing content-manifest cursor, the
exact locale, and an owner-local producer version; an `ActCatalogReader` fences listing and exact
lookups by locale, visibility scope, and single-use continuations.

Each act entry carries localized name/description, owner order, unlock and visibility state,
room/node categories, encounter definitions, encounter pools, and map-generation constraints.
Encounters carry a category, an optional room category, ordered enemy groups with explicit enemy
quantity and variants, eligibility predicates, a generation weight, and typed references. Pools
carry weighted entries; constraints carry a category, an owner rule reference, and mode/difficulty
dependencies. `ActField` keeps an observed empty collection distinct from not-observed, unsupported,
denied, failed, not-applicable, and unknown data.

Definition identities are namespaced and act/encounter scoped, and `ActMapNodeReference` is a
separate live topology identity. `EncounterPossibility` states enumerable pool references or an
explicit withheld/unavailable state; it never reveals a seed-specific assignment or hidden future
room. Enemy and content references resolve through the common manifest, and a category or pool
reference that dangles within its act fails closed.

Duplicate or ambiguous act, category, encounter, group, enemy, variant, pool, entry, eligibility,
constraint, and reference identities are rejected before publication. Manifest, locale, and producer
mismatches, unknown manifest or intra-act references, unsupported or unavailable families, malformed
identities/text, invalid weights/quantities, and definitions over the nested byte bound fail with
typed errors. An unsupported or unavailable family is never a successful empty page, and the byte
estimate counts nested custom kind strings so no field bypasses the bound.

## Evidence and limits

Synthetic fixtures cover a rich act with three categories, normal/elite/boss encounters, an enemy
variant, weighted pools, mode-scoped constraints, bounded deterministic pagination with single-use
continuations, exact act/encounter/category/pool/constraint lookup, reference-possibility resolution
with withheld/unavailable states, locked/owner-only/hidden visibility, manifest/locale/producer and
stale-reference rejection, unsupported/unavailable families, dangling and duplicate identities,
malformed and oversized input, and the nested definition byte limit. These prove deterministic local
validation and read-only projection only. Native act/encounter coverage, live map or seed assignment,
thread affinity, shared transport/gateway/MCP delivery, and exact-host compatibility remain
unverified.
