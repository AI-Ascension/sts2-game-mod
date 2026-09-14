# ADR 0051: Owner-local retained map knowledge

- Status: Proposed; source-only owner boundary
- Date: 2026-09-14
- Tracking: game-mod #98
- Depends on: [ADR 0035](0035-runtime-map-projection.md), [ADR 0038](0038-game-content-manifest-boundary.md), [ADR 0039](0039-field-availability-and-completeness-boundary.md)

## Context

`runtime-map-v1` exposes current-run topology only while the map surface is open. An agent cannot
read already-public topology during combat or another screen, and cannot tell retained knowledge
apart from currently verified navigation authority. This decision defines an owner-local,
host-independent boundary. It does not add a transport route, native extractor, gateway/MCP
adapter, or exact-host compatibility claim, and it does not change `runtime-map-v1` topology IDs,
legal-travel bindings, or snapshot fences.

## Decision

`crates/game-mod/src/retained_map` retains a coherent snapshot copied from a permitted observation
and serves it while the map screen is closed. A `RetainedMapSource` reports capability and the
surface observation state (observable, closed, forbidden, unsupported, unknown). An observation
fails closed for any non-observable surface and never opens, scrolls, or navigates the UI; only an
explicit `observe` reads the source.

Each snapshot binds a static catalog witness (the content-manifest cursor, locale, and an
owner-local producer version) plus live instance, run, act, mode, map-instance, snapshot, and a
monotonic epoch. Reconciliation compares the complete snapshot fence, including `snapshot_id` and
`epoch`, so rotating only the live snapshot identity marks the retained knowledge stale even when
the run, act, and map-instance are unchanged. Replacing a snapshot keeps the same run-level identity
and requires a newer epoch, so an act transition, restore, or new run cannot reuse an old map as
current.

A `RetainedMapReader` exposes explicit freshness (current, retained, stale, withheld, unavailable,
never-observed, unknown), field availability (available, not-applicable, not-observed, unsupported,
denied, hidden, stale, unknown), and per-node visibility. Stale or unknown knowledge is never
reported as current or complete, and a reveal-policy change withholds retained knowledge without
discarding it. Retained travel bindings carry an explicit actionability that is
`current` only while the owning generation matches and the surface is open; `authorize_travel`
rejects every retained, stale, withheld, unavailable, or unknown reference. Travel disclosure and
authorization enforce node visibility: a binding whose `from_node_id` or `to_node_id` is absent or
not visible under the reader's scope is omitted from `travel_references` and rejected by
`authorize_travel` with a typed `scope_denied("travel")` error. A hidden or unknown
node cannot carry a public label or public contents, so hidden future room contents are withheld
rather than fabricated.

Topology pages are bounded and use single-use continuations bound to the snapshot, limit, and
reader. A page exposes both visibility-filtered node summaries and visibility-filtered directed
edges ranked deterministically by endpoint identity. An edge is disclosed only when both endpoints
exist and are visible under the reader's scope, so connectivity beyond immediately available travel
is surfaced without leaking an owner-only or hidden endpoint. The node and edge windows share the
requested limit, the continuation carries both offsets, and a page is `complete` only when both
windows are exhausted and the freshness still trusts topology. The continuation value derives
`Clone`, but single use is enforced server-side: the reader
removes the token on first consumption, so a reused clone is rejected as an invalid continuation.
Node detail and total snapshot bytes are bounded by estimates that count every nested identity,
label, and custom-kind string. Duplicate or ambiguous nodes, edges, and travel actions fail before
publication.

Reveal-policy withholding is a separate dimension from generation freshness: it is preserved across
`observe`, `reconcile`, and `replace_snapshot`, so a current-generation-but-withheld state is
representable, and only an explicit un-withhold clears it. Withheld `topology`, `node`,
`travel_references`, and `authorize_travel` reads fail closed with a typed withheld error. Surface
open/close state is likewise separate from generation freshness: opening the surface never promotes
`retained` back to `current` or re-arms travel, and travel is actionable only when the generation is
current and the surface is open. A rejected observation revokes current authority: a surface now
reported closed demotes a current observation to `retained`, while a rejected read that proves a
changed live identity or generation marks the knowledge `stale` and closes the surface, so an old
travel reference can never authorize after either rejection. Pre-observation `travel_references`
fail closed with the same typed
never-observed error as the other reads rather than returning an empty success.

## Evidence and limits

Synthetic fixtures cover open-map observation, close-map reads with honest retained freshness,
pre-observation unavailable/withheld reads, act/run/mode/map-instance/epoch invalidation, a
snapshot-identity-only fence change, authority revocation after a rejected observation, owner-only
travel visibility and authorization, visible-edge exposure with pagination, hidden
future contents, non-actionable stale travel references, bounded pagination with single-use
continuations, and nested detail/snapshot byte limits. They also confirm the copied
`runtime-map-v1` artifact still verifies unchanged. These prove deterministic local validation and
read-only projection only. Native extraction and exact-host visibility of a closed map, thread
affinity, shared transport/gateway/MCP delivery, and provider acceptance remain unverified.
