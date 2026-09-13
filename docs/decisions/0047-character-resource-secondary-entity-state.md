# ADR 0047: owner-local character resource and secondary-entity state

- Status: Proposed; source-only owner boundary
- Date: 2026-09-13
- Tracking: game-mod #93
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md), [ADR 0039](0039-field-availability-and-completeness-boundary.md), [ADR 0045](0045-power-status-state.md)

## Context

Character-specific resources and persistent secondary combat entities are not interchangeable
with a character or a generic status.  A resource can have a typed current/max value and ordered
slots, while a secondary entity can have its own health, block, statuses, intent, and controller.
An empty result also cannot distinguish a character with no instances from a mechanic that is
unsupported, not applicable, unavailable, or unknown.

The game-information contract and any exact-host adapter are owned elsewhere.  This decision
therefore defines only an owned source boundary and does not claim a transport route, native ABI,
host reflection member, or extractor.

## Decision

`crates/game-mod/src/character_state` owns an immutable static catalog and a coherent live reader.
Catalog definitions bind typed resource and secondary-entity metadata to the existing content
manifest cursor, locale, and producer version.  Each character/mode has explicit resource and
secondary-entity coverage.  Supported coverage requires definitions; unsupported,
not-applicable, unavailable, and unknown coverage remains visible and never becomes a successful
empty page.

Resources retain a typed value definition, current/max fields, active state, and ordered slots.
Secondary entities retain a definition, owner, controller link, HP/max HP/block, bounded statuses,
and one typed intent with target and rule references.  Values use explicit available,
not-applicable, not-observed, unsupported, denied, unavailable, and unknown states.

Every live binding carries the catalog, game instance, run, mode, snapshot, and monotonic epoch.
References include the complete binding and the live instance identity.  Replacing a snapshot
requires the same catalog/game/run/mode and a strictly increasing epoch; old references are stale.
Visibility is enforced at catalog, resource-slot, status, intent, and entity boundaries.  Source
snapshots reject duplicate IDs, malformed slot order, wrong typed units, invalid controller or
intent identities, unsupported coverage, and oversized detail before a reader is published.
Static pages use bounded, single-use continuations tied to the catalog and query scope.

## Evidence and limits

Synthetic source fixtures cover typed resource slots, secondary-entity controller/status/intent
links, pagination and continuation reuse, owner-only visibility, wrong run/epoch, explicit
coverage states, malformed shapes, and detail limits.  These tests prove deterministic local
validation only.  The exact-build public-mechanic inventory, lifecycle/change comparisons, and
native observations remain unresolved acceptance items pending authorized host evidence and the
shared delivery owners.  No transport, native implementation, game-thread extractor, or
exact-host compatibility evidence is included.
