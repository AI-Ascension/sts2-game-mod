# ADR 0032: Native co-op source candidate

- Status: Proposed; source/component candidate, protocol admission pending
- Date: 2026-09-10

## Context

The managed-rust-interop experiment already contained an isolated co-op projection helper. A later
candidate added native multiplayer observation, local actions, shared votes, peer rejoin, and
operation recovery, but it was based on a branch that predated the seeded-run adapter on mod main.
Its callback IDs 9 and 10 therefore collided with the accepted seeded listener routes, and its
shared mutation gate referred to a Runtime-v4 REST helper that is not present on the seeded mainline.

The game-mod target can own the host-facing source boundary, but the protocol schema, gateway,
MCP, and harness remain separate owners. A source integration must preserve those boundaries and
must not turn a producer compile into a multiplayer support claim.

## Decision

Rebase only the co-op host, native encoding, synchronizer, session, and deterministic probe files
onto seeded mod main `caae865986d2274736d92b4f9be2bbda24bab83d`. Candidate map, rest, release, and
other shared-stack changes remain excluded.

The managed/native callback table reserves 16 through 20 for co-op observation, local action,
shared vote, rejoin, and recovery. Seeded start and reconciliation retain callback IDs 9 and 10.
The native listener exposes five authenticated routes under `/api/v1/coop/native/`; all routes
carry the existing instance, caller, session, lease, epoch, and correlation identity into the
game-thread callback. The managed parser rejects unknown or duplicate envelope members and does
not accept caller-supplied effect witnesses.

`CoopHostRuntime` owns operation idempotency, peer identity, host-generation fencing, authority
epochs, native effect witnesses, and all-peer checkpoint convergence. `InstalledNativeCoopHostPort`
is the only producer allowed to call first-party multiplayer synchronizers. A client may rejoin
through the first-party load lobby, while a disconnected or divergent peer leaves the operation
unknown. Every co-op mutation checks pending v2, v3, v4, and seeded operations immediately before
native dispatch; reconciliation remains read-only and is allowed while another profile is pending.

The `coop-native-v1` schema and artifact remain unadmitted protocol-owner material and are not
copied into this target. No gateway, MCP, or harness consumer is claimed. The existing
`coop-synchronization-v1` coordinator-report profile remains separate and read-only.

## Evidence and limits

The rebased source passes the native route tests, including the disjoint callback IDs. The
source-only managed admission, host-receipt, parser, and cross-profile gate probes pass with
warnings treated as errors. The managed loader and host-dependent checkpoint composition compile
against the operator-supplied STS2 v0.107.1 Windows assemblies in the authorized local build.
These checks establish source and exact-host compilation evidence only.

No live two-peer host/client run was performed. Native action settlement, checksum semantics,
running-session rejoin, package loading, and Windows/Linux runtime compatibility remain unverified.
Protocol admission requires the exact candidate artifact, independent gateway/MCP/harness
consumers, cross-language conformance, and a disposable two-peer host/client trace with settled
actions and recovery evidence.
