# ADR 0046: Owner-local combat bookkeeping and public draw-pile projection

- Status: Proposed; source-only owner boundary
- Date: 2026-09-13
- Tracking: game-mod #94
- Depends on: [ADR 0039](0039-field-availability-and-completeness-boundary.md), [ADR 0044](0044-live-card-state-projection.md)

## Context

The existing gameplay observation carries a hand, permanent deck, discard, and exhaust list but
does not identify the current draw-pile count, generated or resolving card zones, round/player
turn identity, or named combat counters.  A before/after comparison also cannot prove unseen
events or preserve a card instance when a generated card moves between zones.

This target needs an owner-local boundary that can be reviewed before the shared game-information
contract and exact host extractor are available.  The boundary must remain useful for public
reads without publishing secret draw order, inventing zero values for unknown counters, or
claiming that a coherent snapshot exists while the host is resolving a transition.

## Decision

`crates/game-mod` owns a source-only `CombatBookkeepingSnapshot` and read-only
`CombatBookkeepingReader`.  A `CombatBookkeepingBinding` fences every value by content manifest,
game instance, run, combat, coherent snapshot, and monotonic epoch.  This binding is deliberately
distinct from the live-card-state binding so a future shared contract must negotiate both
projections instead of silently treating one as the other.

Supported zones are hand, draw, discard, exhaust, limbo, resolving, and temporary.  Every zone
has an explicit inventory status, a count field, an independently available composition field,
composition completeness, and an ordering classification.  A draw pile may expose a count and a
player-permitted composition while retaining `Unordered`, `Hidden`, `Unknown`, or
`NotObserved` order.  A known card position is rejected unless the source explicitly marks the
zone order `Public`; a secret draw order therefore cannot leak through a list index.  Permanent
deck count is carried separately from the current in-combat count, and temporary count is
reconciled only when its source zone is known.  This permits generated cards to make combat totals
differ from the permanent deck without coercing either count.

Card memberships carry a distinct live instance ID, definition ID, owner ID, and the exact
combat binding.  Duplicate instances across zones, moved cards under an older epoch, and
references copied from another combat are rejected.  Current membership is the only lifecycle
evidence; the projection does not infer a generated, destroyed, or moved event from two snapshots.

The counter inventory is intentionally fixed to cards played, damage taken, damage dealt, and
enemies defeated.  Every counter carries an explicit value state, reset boundary, and provenance.
Provenance is either a host-reported value, a semantic-history identity plus event count,
`NotObserved`, or `Unknown`; there is no arbitrary reflection bag and no inferred event count.
Round and player-turn identities, resolving state, and a pending public selection/effect are
independent fields with the same availability semantics.

`CombatField<T>` distinguishes observed values from not-applicable, not-observed, unsupported,
denied, owner-only, busy, stale, and unknown states.  Source errors map to typed `Busy` or
`SourceStale` outcomes.  Replacing a snapshot requires the same manifest, game, run, and combat
identity plus a strictly larger epoch; old card references then return `StaleReference`.

Adoption validates the complete supported-zone inventory before reconciling an available combat
total; an unobserved zone is never treated as an empty zone.  Hidden and unordered compositions
are canonically sorted by card identity while public order is retained.  The adopted snapshot is
private and read-only, and its size bound measures nested turn, card-reference, history,
resolution, selection, and pending-effect strings rather than trusting a partial estimate.

`FixtureCombatBookkeepingSource` and deterministic tests are the only available implementation.
`UnavailableCombatBookkeepingSource` reports `exact_host_evidence_required`.  No native extractor,
managed route, wire schema, gateway/MCP adapter, semantic-history adapter, or gameplay mutation is
claimed by this decision.

## Evidence and limits

The source-only tests cover unordered draw composition and count reconciliation, generated
temporary cards, duplicate-definition instances, public pending selections, pending identity and
effect-size validation, canonicalized hidden composition, named counter provenance and explicit
unknown values, stale epoch replacement, wrong-combat rejection, secret order and count-mismatch
failures, incomplete inventory reconciliation, unsupported zones, owner-only visibility, transient
busy/stale source errors, unavailable capability, and duplicate live instances.  These tests prove
only owned in-memory validation and read-only projection behavior.  Exact host field availability,
thread affinity, native delivery, shared-contract negotiation, and runtime compatibility remain
unverified.
