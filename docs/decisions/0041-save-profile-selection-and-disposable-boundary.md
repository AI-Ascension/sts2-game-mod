# ADR 0041: save-profile selection and disposable provisioning boundary

- Status: Proposed; source-only owner boundary
- Date: 2026-09-13
- Tracking: game-mod #78

## Context

`SeededRunProfileBaseline` proves only that an already-isolated user-data root matches a
read-only digest. It is not a general save-profile discovery or selection API. The session
launcher requires a non-secret authorization record before any profile access and deliberately
does not inspect or copy saves. A future workflow must keep save-slot identity distinct from
instance, user-data, provider, and workflow profile identities.

## Decision

Game-mod may expose only bounded host-owned save-slot summaries and current selection after a
negotiated cross-owner contract exists. A summary uses an opaque slot identity, supported-host
compatibility, active-run restriction, and freshness/baseline references. It contains no save
payload, install or filesystem path, account identity, or raw host error.

Selecting a slot is host-thread work and requires explicit selection authority, expected slot
identity, compatible instance/user-data identity, baseline fence, idempotency key, and
authoritative post-operation readback. Active run, pending/failed save, stale baseline, wrong
instance, concurrent selection, unsupported slot, or unavailable host is an explicit rejection.
No retry may select a different slot after a lost response.

Disposable user-data creation belongs to the gateway's isolated launch/provisioning boundary;
game-mod never creates directories, adopts unknown existing content, copies a valued profile, or
overwrites a slot. The selected disposable identity and baseline are inputs to game-mod's
host-thread admission only. Existing profiles remain untouched on every rejection.

## Source-only acceptance

Fixtures must prove distinct opaque identity types, summary redaction, selection idempotency,
stale/wrong-baseline rejection, active-run and in-use refusal, lost-response reconciliation, and
that failed requests perform no selection mutation. They must reject path-like identifiers,
unknown existing directory adoption, implicit default selection, and use of a workflow/provider
profile where a save-slot identity is required.

Exact-host acceptance separately requires a disposable profile and authorization record under
ADR 0013. It must show authoritative selection/readback and that original profiles and selection
are unchanged after every failed operation. No synthetic result certifies host save semantics.

## Limits

This decision adds no profile route, filesystem access, slot mutation, directory provisioning,
host launch, or shared schema. Gateway, MCP, harness, and Studio retain their named contract and
delivery work; without their accepted companion changes this capability stays unavailable.
