# ADR 0048: owner-local structured enemy intents and public targets

- Status: Proposed; source-only owner boundary
- Date: 2026-09-14
- Tracking: game-mod #96
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md), [ADR 0039](0039-field-availability-and-completeness-boundary.md), [ADR 0045](0045-power-status-state.md)

## Context

An enemy intent is not a single damage number or an enum label. A visible intention can combine
attack, block, heal, buff/debuff, summon, escape, phase-change, or source-defined effects, each with
its own parameters, affected references, and target set. Hidden and unknown targets must not be
represented by fabricated sentinel IDs, and an unknown new category must not collapse into a
misleading known kind.

The game-information contract and any exact-host adapter are owned elsewhere. This decision defines
only an owned source boundary and does not claim a transport route, native ABI, host reflection
member, or extractor.

## Decision

`crates/game-mod/src/enemy_intents` owns an ordered, bounded component projection and a coherent
live reader. Static content binding, locale, and producer version reuse the existing content-manifest
cursor. Each live binding carries the catalog, game instance, run, combat, snapshot, and monotonic
epoch; references include the complete binding, the enemy instance, and the distinct intent and
component identities.

An intent retains an ordered component list. Each component carries a category (including explicit
custom/unsupported kinds), an optional description, typed damage (per-hit, hits, total), a typed
amount, bounded effect references, bounded typed parameters, and a target field. Target selection
distinguishes no target, unknown, hidden, and visible target IDs with their domain and family.
Field values use explicit available, not-applicable, not-observed, unsupported, denied, hidden,
stale, and unknown states so absence is never read as zero.

Source snapshots reject duplicate or ambiguous enemy, intent, component, target, effect, and
parameter identities, invalid typed units, malformed target domains, and oversized detail before a
reader is published. Per-enemy detail, total snapshot bytes, and all nested strings are bounded;
custom kinds and text are counted so no category can bypass the limit. Reads do not advance enemy AI
or evaluate random choices.

## Evidence and limits

Synthetic fixtures cover compound attack-plus-effect intents, ordered components, per-hit and total
damage, ally/enemy target references, hidden/unknown/not-observed targets without fake IDs, identity
and epoch fences, typed read errors, and oversized/ambiguous rejection. These prove deterministic
local validation only. Native UI comparison for every advertised intent family, state-change
invalidation, and the exact-build variant inventory remain unresolved acceptance items pending
authorized host evidence and the shared delivery owners. No transport, native extractor, or
exact-host compatibility evidence is included.
