# ADR 0035: additive permitted map projection

Status: accepted for implementation; source/build and synthetic evidence only. Live acceptance
is tracked separately in the map host evidence.

Renumbered from 0030 during integration on 2026-09-08 to preserve the existing visible-combat-intent
decision identifier, and from the conflicting integration candidate identifier 0032. The projection
scope is unchanged.

## Decision and ownership

Expose the fixed authenticated `GET /api/map/v1/snapshot` route under the independent
`runtime-map-v1` profile. Consume the protocol-owned contract at revision
`7c448bd8d7a695ada48830176f3d738286caafe4`, with schema SHA-256
`ceab0d2dfc471d1ec36d12edaf4654b8c7fdced06548bf47265e11c63f98115b`.
Existing gameplay profiles, envelopes, and mutation witnesses retain their meanings.
The private managed/native callback table assigns map reads ID 14; expert state/action remain
7/8 and native co-op remains 9 through 13.

The mod owns host-thread extraction and exact current legal-action bindings. Gateway/MCP own
external identity and lease admission. The harness owns analysis, provider context, bundles,
and replay; the visualizer consumes those artifacts without host access. No new game-core
dependency or gameplay-rule implementation is introduced.

## Projection and failure behavior

The read requires the campaign's current map to be open and visible. It copies all permitted
nodes and directed connections in that scope, including visible unreachable branches. It does
not open or scroll the UI, reconstruct a seed, inspect future acts, or expose hidden outcomes.
Unknown categories remain explicit. Closed maps return `not_observable`; unsupported or
ambiguous state returns bounded unavailable/incomplete information rather than an invented graph.

Node identities use opaque tokens scoped to host run/map object lifetime, separate from
generation-bound navigation action IDs. The registry is bounded at 4096 references and fails
closed after exhaustion until a new run/map object resets it. Projection is bounded to 256
nodes, 1024 edges, and finite traversal work; responses are limited to 256 KiB.

The projection captures gameplay state/catalog generation, copies the map, and reobserves.
The snapshot copies the exact `state_id` from that gameplay observation, including unavailable
and changed-surface responses, so map and gameplay profiles share one identity at a generation. A
changed generation returns an explicit `map_surface_changed` unavailable response. Stable public
node identities participate in the topology fingerprint so tied coordinates do not conceal
rewiring. Host references do not leave the callback as snapshot data.

## Validation and rollout

Managed probes characterize shared gameplay/map identity, identity churn/reset, hidden-category
normalization, tied-coordinate rewiring, and generation rejection. Native route tests preserve the
fixed read seam and legacy behavior. Exact-host builds and package hashes are in
[the host evidence](../evidence/runtime-map-v1-host-build-20260907.md); none proves live extraction,
provider image delivery, or a settled map action.

Integrate the protocol/producer before enabling the gateway/MCP capability and strict harness
map context. Mixed versions reject the independent profile explicitly. Feature-off rollback
disables map delivery and uses the unchanged gameplay profile; cached graph/action identities
cannot authorize a mutation or replace a fresh host observation.
