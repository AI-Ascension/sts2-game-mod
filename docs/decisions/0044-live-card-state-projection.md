# ADR 0044: owner-local live card state projection

- Status: Proposed; source-only owner boundary
- Date: 2026-09-13
- Tracking: game-mod #87
- Depends on: [ADR 0039](0039-field-availability-and-completeness-boundary.md)

## Context

The existing gameplay observation identifies a card by a definition-like ID and exposes only a
boolean upgrade flag and a nullable resolved cost.  That shape cannot distinguish two copies of
one definition, preserve an enchantment's lifetime, or explain an X, free, unplayable, alternate,
negative, or otherwise unresolved cost.  A later shared game-information contract may select
different wire names; this target must first keep the facts coherent and fail closed locally.

## Decision

`crates/game-mod` owns a source-only `LiveCardStore` with an owned snapshot boundary.  A
`LiveCardReadReference` binds every snapshot to an instance, run, content manifest, coherent
snapshot ID, and monotonic epoch.  A `CardDefinitionReference` is deliberately separate from a
`CardInstanceReference`; identical definitions therefore never collapse two live cards.

Each projection records the owner, pile or selector location and position evidence, upgrade count
and variant/path, retained/exhaust/ethereal flags, bounded effect-parameter overrides, and an
ordered modifier vector.  Modifiers carry a source reference, order, scope, optional amount,
typed value, and expiration condition.  The source vector order is retained because host
semantics may depend on modifier order.

`CardCostSemantics` keeps base, current, and effective costs separately.  `CardCost` has typed
fixed, X, free, unplayable, and alternate-resource forms.  `CardCost::Unknown` and
`CardCostAmount::Unknown` retain an observed signed/sentinel value and reason; they never coerce a
negative host sentinel to zero or an unexplained null.  Visible cost contributors preserve their
source, scope, amount, typed value, and expiration, including unresolved contributors.

Pages are bounded and source ordered.  Pile and selector queries use single-use opaque
continuations bound to the same store, collection, instance set, and read reference.  Complete
card detail is read through the stable instance reference.  A replacement snapshot must advance
the epoch; every old page, continuation, and detail reference then returns `stale` rather than
mixing records from two snapshots.  The store validates duplicate instance IDs, definition
manifest binding, modifier order, local keys, effect/modifier bounds, and detail byte estimates
before returning a value.  Unsupported fields, unavailable detail size, oversized payloads, and
unavailable source capability are typed failures; no successful partial projection is published.

`FixtureLiveCardSource` and the checked-in tests are deterministic source evidence only.
`UnavailableLiveCardSource` returns `exact_host_evidence_required`.  There is no native extractor,
transport route, gateway/MCP adapter, capability claim, or gameplay mutation in this decision.
Future owners must negotiate the versioned shared contract and map these owner-local outcomes
explicitly before enabling an external query.

## Evidence and limits

Fixtures cover duplicate-definition instances with different upgrades/modifiers/costs, selector
cards outside the hand, temporary cost expiry across epochs, alternate and negative/sentinel cost
semantics, stale movement/upgrade references, unsupported fields, pagination, oversized detail,
invalid continuation, and unavailable source behavior.  The tests prove only owned in-memory
projection semantics and read-only behavior; they do not prove exact-host reflection, native
thread affinity, transport delivery, or runtime compatibility.
