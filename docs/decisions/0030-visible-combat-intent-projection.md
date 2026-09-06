# ADR 0030: Visible combat intent projection

- Status: Proposed pending independent host verification
- Date: 2026-09-06

## Context

Runtime-v3 combat observations previously emitted `Unknown` with zero damage and hits for every
enemy. An intent may only cross the fair-play boundary when the native combat UI is currently
rendering it. The host can retain a move after its UI has been hidden, during a transition, or
while a composite move contains more than one effect; projecting that state would expose hidden
or incomplete information.

## Decision

The managed projection admits an intent only when the matching native `NCreature` and visible
intent container exist, intent visibility is not debug-suppressed, every rendered `NIntent` child
is visible, and each child is bound to the corresponding current host intent. The rendered binding
also has to identify the same owner, expose an unfrozen presentation, and provide a target list
whose native creature nodes are visible. A composite move is reported as `Unknown` because the
current neutral schema has one intent object and cannot preserve secondary effects. For an attack,
the projection uses only the targets carried by the rendered `NIntent`; every target must be one of
the currently visible player creatures before native `AttackIntent.GetSingleDamage` is called.
Damage and repeat counts must fit the wire bounds; out-of-range or unavailable values fail closed to
`Unknown` rather than being clamped. Enemy IDs remain derived from the authoritative combat ID.

Reflection is limited to native `NIntent` presentation bindings (`_intent`, `_owner`, `_targets`,
and `_isFrozen`) so the projection can prove that the host intent being read is the one rendered by
the visible UI and that its target identity is current. It does not read future RNG, hidden move
state, private saves, or policy data. If any binding field is absent after a host update, or a
binding is stale, frozen, or unavailable, the projection returns `Unknown` until a reviewed
compatibility change updates this adapter.

## Evidence and limits

The source-only managed probe cannot compile or execute this host-dependent path. The loader build
against the operator-supplied host assembly checks symbol compatibility only. An independent
verifier must inspect the exact diff and run disposable-host comparisons for single attacks,
multi-hit attacks, defend, buff, debuff, composite/unknown moves, hidden/frozen/stale intent UI,
and single/all-player target identity before this ADR is accepted or the patch is merged.
