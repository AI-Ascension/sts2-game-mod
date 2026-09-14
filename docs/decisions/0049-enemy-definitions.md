# ADR 0049: Owner-local enemy, boss, elite, and minion reference definitions

- Status: Proposed; source-only owner boundary
- Date: 2026-09-14
- Tracking: game-mod #95
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md), [ADR 0039](0039-field-availability-and-completeness-boundary.md), [ADR 0048](0048-structured-enemy-intents.md)

## Context

Enemy data is not a name, a single HP number, and one compact intent. A definition can carry
localized text, base and difficulty/mode-scaled stats, tags, spawn conditions, encounter and status
links, origin/package variants, multiple behavior phases, compound moves, cooldowns, repetition
restrictions, conditional transitions, and partially known probabilities. The existing gameplay
observation exposes only identity, name, HP/max HP/block/statuses, and one current intent, so an
agent cannot inspect an enemy before encountering it and cannot distinguish an unknown transition
weight from a certain one.

This decision defines an owned source boundary. It does not claim a native extractor, a transport
route, a gateway/MCP adapter, or exact-host compatibility.

## Decision

`crates/game-mod/src/enemies` owns an immutable `EnemyCatalog` produced by `EnemyCatalogProducer`
from a bounded `EnemyCatalogSource`. Static results bind to the existing content-manifest cursor,
the exact locale, and an owner-local producer version; an `EnemyCatalogReader` fences listing and
exact lookups by locale, visibility scope, and single-use continuations.

Each catalog entry is a complete definition: localized name/description, role (normal, elite, boss,
minion, or custom), typed origin/package provenance, unlock and visibility state, tags, base and
scaled stat profiles with fixed, formula-backed, or explicitly unavailable values, spawn
conditions, encounter references, and origin/package variants. Spawn and encounter data use
`EnemyField` so an observed empty collection is never confused with not-observed, unsupported,
denied, not-applicable, or unknown data.

Moves are stable enemy-scoped references with ordered effects, typed targeting, phase membership,
conditions, cooldown and repetition rules, and a probability that is an exact rational, a
non-executable formula with unresolved inputs, or explicitly unavailable. A move effect carries a
kind (attack, block, heal, apply/remove status, summon, escape, phase change, custom, unknown), a
localized description, parameters, and typed references. Phases carry ordered move membership and
an optional entry condition; transitions carry an optional source phase, a destination phase, a
condition, and a probability. Reference AI rules and unknown weights stay separate from any live
chosen move or RNG cursor.

Duplicate or ambiguous enemy, move, phase, effect, parameter, tag, condition, encounter, and
reference identities are rejected before publication. Manifest, locale, and producer mismatches,
unknown manifest references, origin/package mismatches, unsupported or unavailable families,
malformed identities/text, invalid probabilities, and definitions over the nested byte bound fail
with typed errors. An unsupported or unavailable family is never returned as a successful empty
page, and the byte estimate counts nested custom kind strings so no field can bypass the bound.

## Evidence and limits

Synthetic fixtures cover a multi-phase boss with a compound move, a conditional transition, a
summon, difficulty-scaled stats, an origin variant, locked/owner-only/hidden visibility, bounded
deterministic pagination with single-use continuations, exact definition and move lookup plus
definition-scoped phase/transition inspection, unsupported/unavailable family states and
not-applicable/not-observed field states, stale manifest/locale/producer references, duplicate and
ambiguous identities, malformed and oversized input, dangling move references, and the nested
definition byte limit. These prove deterministic local validation and read-only projection only. Native field availability, live AI behavior, thread affinity,
shared transport/gateway/MCP delivery, and exact-host compatibility remain unverified.
