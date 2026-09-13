# ADR 0042: native checkpoint restore boundary

- Status: Proposed; no native restore implementation
- Date: 2026-09-13
- Tracking: game-mod #81

## Decision

Restore is the inverse of an advertised ADR 0037 capture codec, performed only by the game-owned
host-thread port. Before any effect it verifies complete private closure, exact build/adapter/schema
and coverage compatibility, allowed boundary, disposable destination identity, profile baseline,
and operation/authority fences. Unknown phase, incomplete coverage, corruption, mismatch, or
oversize input rejects before mutation; public observations can never reconstruct a checkpoint.

Gateway provisions a separate disposable destination. The source checkpoint and source branch are
immutable. Reusing a destination requires explicit exclusive stopped ownership and an identified
replace operation; an uncertain or partial destination is quarantined and cannot admit actions.

Each restore operation is durably reconciled as admitted, restoring, verifying, verified, rejected,
failed, or unknown. Disconnect or restart re-reads that same operation; it never blindly invokes a
second native restore. After restore, game-mod recaptures through the independent capture port and
compares canonical private bytes and declared compatibility. Only this destination recapture plus a
new execution epoch can bind a verified receipt. Old action catalogs, proposals, and epochs are
invalid even when bytes match.

## Acceptance and limits

Source-only fixtures must reject wrong identity/coverage, missing or oversized closure, forged
receipt, interruption at each persistence point, stale action catalog, and a second effect after
uncertainty. Exact-host evidence must prove fresh-process capture/restore/recapture equality and
same-action continuation across every advertised phase. Restore verification is distinct from
continuation certification.

This decision adds no codec, persistence, route, process provisioning, native host action, or
restore capability. It remains unavailable until #80, #78, gateway lifecycle/provisioning, and
authorized exact-host evidence meet these requirements.
