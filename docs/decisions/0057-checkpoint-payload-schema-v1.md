# ADR 0057: closed checkpoint payload schema v1 and typed model

- Status: Proposed; source-only contract
- Date: 2026-09-17
- Tracking: game-mod #80 (required implementation item 2)
- Depends on: [ADR 0037](0037-native-checkpoint-coverage-inventory.md),
  [ADR 0043](0043-native-checkpoint-capture-port.md),
  [ADR 0054](0054-restricted-canonical-checkpoint-encoder.md),
  [ADR 0055](0055-checkpoint-capture-admission-barrier.md)

## Context

Issue #80 item 2 requires closed, versioned, game-owned payload schemas for the first-release
boundaries. The protocol's `asc-jcs-state-v1` contract (sts2-protocol ADR 0036, revision
`8a2e66f5d2190a0fca7f146dc3508e8d55515ea7`, copied in `protocol-artifact/exact-state-v1`)
deliberately leaves the `state` object open: "game-owned per-phase payload schemas are intentionally
not defined here", because an incomplete closed schema or an untyped bag would silently weaken
coverage. Until now this repository had the boundary matrix (`capability.rs`, ADR 0043), the
canonical encoder (ADR 0054) and the admission barrier (ADR 0055), but no schema, fixture, or typed
model for what a payload at a supported boundary must contain.

## Decision

`schemas/checkpoint-payload-v1.schema.json` (`$id` `sts2-checkpoint-payload-v1`, payload schema
identifier `ascension.checkpoint_payload.v1`, canonical profile `asc-jcs-state-v1`) is the
game-owned payload contract for exactly three boundaries: `settled_map_choice`,
`stable_player_turn_combat`, and `later_turn_combat`. Every other `CheckpointBoundary` value has
no payload schema; `CheckpointPayload::schema_for` returns `None` for it and a payload that names it
is rejected with the existing matrix rejection from `CheckpointCapabilities::for_boundary`
(`unsupported_boundary` with `coverage_inventory_pending`, or `unsafe_boundary`). No new rejection
kind is introduced and no phase is advertised as available: `CheckpointCapability::is_available`
remains `false` for all eleven boundaries.

The schema is closed everywhere: every `type: object` node declares `additionalProperties: false`,
and the test walks the schema to prove it. Every JSON key satisfies the canonical key grammar
`^[a-z][a-z0-9_]*$`. Exact 64-bit values use the profile's tagged `uint64` object form; all other
integers stay inside the IEEE-754 safe range. Identifiers are bounded to 128 bytes of
`[A-Za-z0-9._:/-]`; every collection has an explicit maximum (see `PAYLOAD_MAX_*`).

### Coverage families and the unknown-value policy

The payload carries twelve families drawn from the ADR 0037 inventory (seed and RNG stream cursors;
character/ascension/mode/acts/modifiers; map and room progress; player resources; ordered deck and
piles; card instances/upgrades/temporary values; relics; potions; combat turn witnesses; powers;
enemies and intents; pending effects and external inputs). Each family is
`{"coverage": "captured", "value": {...}}`, `{"coverage": "unknown"}`, or
`{"coverage": "not_applicable"}`. The values are typed placeholders over source-derived
categories; they do not name proprietary members and do not claim that a host exposes them.

| Boundary | Required `captured` | Required `not_applicable` | Extra witness |
| --- | --- | --- | --- |
| `settled_map_choice` | the nine non-combat families | `combat_turn`, `powers`, `enemies_and_intents` | none |
| `stable_player_turn_combat` | all twelve | none | `combat_turn.turn >= 1` |
| `later_turn_combat` | all twelve | none | `combat_turn.turn >= 2` |

A required family reported as `unknown` is rejected (`required_family_unknown` mapped to the
existing `unsupported_coverage` rejection), never substituted with an empty value or an earlier
state. `not_applicable` on a required family, or `captured` on a family that has no state at the
boundary, is rejected the same way. `unknown` is therefore representable but never accepted at a
v1 boundary; it exists so a producer can report an incomplete closure instead of fabricating one.
`pending_effects` is closed to empty: an outstanding effect is an unsafe boundary, not a capture.
The schema encodes these rules with per-boundary conditionals, and the typed model enforces them
again so both witnesses agree.

### Typed model

`crates/game-mod/src/checkpoint/payload` owns `CheckpointPayload`, `CheckpointPayloadFamilies`,
`PayloadFamilyCoverage<T>`, the `Payload*` family types, and `CheckpointPayloadError`.
`CheckpointPayload::new` validates the boundary rules, bounds, and cross-family consistency
(distinct RNG stream identities, distinct card instance identities, every deck/pile reference
resolving to a card instance). `lower()` / `From<&CheckpointPayload> for CanonicalValue` produce
the restricted canonical value; `parse_canonical` / `parse_canonical_text` strictly parse it back,
refusing undeclared fields, so lowering and parsing are inverse and the canonical bytes are stable.
The `Debug` rendering is redacted to the boundary and coverage tokens.

### Relation to the protocol envelope

This payload is the game-owned content that the `ascension.exact_state.v1` envelope leaves open. It
does not replace the envelope, its compatibility block, its manifest, or the `asc-state:v1` /
`asc-checkpoint:v1` identities defined by protocol ADR 0036. How a future producer places the
payload inside the envelope's `state` object (RNG streams into `state.rng`, families into
`state.gameplay`) is deferred to the producer decision; today the payload's canonical bytes are
what the ADR 0054 encoder hashes and the ADR 0055 admission path binds.

## Evidence and limits

`conformance/cases/checkpoint-payload-v1.json` pins the schema digest, four valid fixtures with
their canonical byte counts and state/blob digests, one distinctness pair (a changed RNG cursor
with an identical public projection), and nine invalid fixtures with the expected typed error and
capture rejection; `conformance/fixtures/checkpoint-payload-v1/SHA256SUMS` inventories every
fixture. `crates/game-mod/tests/checkpoint_payload.rs` checks the round trip, the RNG-cursor
identity change, required-unknown rejection, the pinned schema digest and closure, the absence of
a schema for every unsupported phase, that every fixture on disk is consumed, and Debug redaction.

Everything here is synthetic source evidence. Nothing reads a native field, inspects a host,
captures a phase, persists an artifact, or restores a run. The schema's field families are not
proof that the inventory is complete against an exact build (ADR 0037 item 1 remains open), and
no acceptance criterion needing native or exact-host evidence is closed by this record. Native
capture, restore, and host compatibility remain unverified and unavailable.
