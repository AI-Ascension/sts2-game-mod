# ADR 0045: owner-local power and status state

- Status: Proposed; source-only owner boundary
- Date: 2026-09-13
- Tracking: game-mod #90
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md), [ADR 0039](0039-field-availability-and-completeness-boundary.md)

## Context

The current expert observation retains only a status identity, display name, and optional integer
amount.  That shape cannot explain amountless markers, decimal or multi-counter values, source and
owner identity, duration/decay boundaries, stacking policy, suppression, or a pending expiry.  It
also cannot distinguish two live applications of one definition or tell a caller that a value was
hidden, unsupported, or not observed.

The shared game-information contract and exact host adapter are owned elsewhere.  This decision
therefore defines only the game-mod-owned values and deterministic source boundary; it does not
freeze a route, wire schema, native capability, host reflection member, or provider behavior.

## Decision

`crates/game-mod/src/powers` owns an immutable static catalog and a coherent live snapshot.  Static
definitions bind a namespaced definition ID to the existing content-manifest cursor, locale, and
owner-local producer version.  They retain localized title/description, power/status family and
category, package provenance, unlock and visibility state, typed amount shape, stacking policy,
caps, duration and reset rule, decay rule, and effect/keyword/rule references.

Amount shapes are explicit: amountless, integer, boolean, decimal, text, multi-counter, and
owner-defined custom values.  A multi-counter declaration carries a counter ID, localized label,
unit, reset timing, visibility, and optional cap.  Static values are not inferred from a display
name or coerced into one nullable integer.

Live instances have identities independent of their definition IDs.  Each instance binds to one
game instance, run, snapshot, epoch, owner family (player, ally, enemy, or secondary entity), and
owner ID.  A visible source/creator is retained as a separate typed reference.  Live fields carry
explicit available, not-applicable, not-observed, unsupported, denied, unavailable, and unknown
outcomes.  Available values retain typed amount variants, remaining duration counters, application
order, active/suppressed state, and pending visible expiry.

The live reader validates static amount and duration shapes, counter IDs/units/caps, bounded text,
owner/source identities, definition visibility, and detail byte limits before returning a value.
Two instances may share one definition; duplicate live instance IDs are rejected.  A replaced
snapshot must retain the same catalog, game, and run identities and advance the epoch.  Every
older instance reference then returns `stale`; a non-advancing epoch is rejected.  Source reads
must echo the expected binding exactly.

Definition and live pages/readers are bounded and source ordered.  Continuations are opaque,
single-use, and bound to one catalog reader.  Unsupported family coverage, source errors,
unknown definitions, wrong identities, hidden values, malformed shapes, and oversized details
remain typed failures; they are never reported as zero, empty, or complete data.

## Evidence and limits

Synthetic tests cover additive and non-stacking definitions, permanent and counter-based
durations, decimal and multi-counter amounts, amountless markers, caps and semantic references,
duplicate definitions across player/enemy/secondary owners, visible source links, suppression and
pending expiry, visibility scopes, stale catalog/live references, non-monotonic epochs, source
errors, unsupported family coverage, invalid shapes, duplicate identities, and oversized payloads.
Repeated reads are read-only and preserve the same typed values.

The catalog and live source traits are owner-local seams.  No implementation in this decision
reads proprietary assemblies, constructs playable objects, mutates a profile or run, opens a
socket, emits transport bytes, or claims exact-host compatibility.  Native extraction and
gateway/MCP/harness delivery remain separate acceptance gates.
