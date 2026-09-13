# ADR 0037: native checkpoint coverage inventory

- Status: Proposed; source-derived inventory only
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
