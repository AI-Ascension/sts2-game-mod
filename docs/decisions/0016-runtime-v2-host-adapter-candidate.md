# ADR 0016: Runtime-v2 host adapter candidate

- Status: Proposed for controlled-host validation; source/build evidence only
- Date: 2026-09-02
- Profile: `sts2-protocol/runtime-v2`

## Context

The deterministic Runtime-v2 seam already defines an argument-free `end_turn` operation, but the
game-mod target had no host-facing v2 route or concrete combat adapter. The existing Runtime-v1
overlay probe is not a gameplay oracle and must not be relabeled as one.

Read-only reflection against the operator's exact STS2 v0.107.1 host assembly (`59260271`) found
the common symbols needed for this narrow path: `CombatManager.Instance`, `IsInProgress`,
`IsEnemyTurnStarted`, `PlayerActionsDisabled`, `DebugOnlyGetState`, `RunManager.Instance`,
`RunManager.DebugOnlyGetState`, `LocalContext.GetMe`, and
`PlayerCmd.EndTurn(Player, Boolean, Func<Task>)`. A newer installed host also exposes
`CombatManager.IsPlayPhase`, but the recorded v0.107.1 host does not; the adapter therefore uses
the common `IsEnemyTurnStarted` and `PlayerActionsDisabled` properties instead of depending on the
newer property.

## Decision

The 2026-09-05 review extracts this independent Runtime-v2 work from PR #14 at
`251e4c118a7e2b59bc120ee45d31f95b1c0ba9af`. The owner selected the Exo semantic lane;
the bounded card profile is excluded from this split. Runtime-v2 callback kinds remain
3, 4, and 5; a later semantic profile must allocate a distinct callback kind.

Add the v2 path only inside the existing managed/native experiment boundary. Keep the C ABI struct
unchanged so Runtime-v1 remains compatible. Add distinct internal callback kinds for v2 state,
action, and operation lookup. The operation ID is passed as the bounded body of the operation
lookup callback; it is never treated as arbitrary URL or host data.

The managed bridge must:

- validate the frozen Runtime-v2 metadata, exact field set, identities, lease epoch, generation,
  action identity, and context binding;
- read a bounded owned observation on the Godot main thread;
- revalidate player-turn state and host action availability immediately before calling
  `EndPlayerTurnAction` through `ActionQueueSynchronizer.RequestEnqueue`;
- retain the operation by ID as `unknown` after enqueue; do not treat a later player turn as
  operation-bound completion evidence or emit a `turn_end_settled` witness without that evidence;
- return `unknown` for host-action or transition uncertainty and expose retained operation reads;
  duplicate exact requests replay and conflicting operation reuse is rejected; and
- keep request admission bounded with a mod-side queue capacity of 16 by default and 64 maximum,
  and retain at most 64 operation receipts so a long-running process cannot grow the managed map
  without bound.
- cancel a queued request when the five-second callback boundary expires before the main-thread
  pump claims it; if the pump has already claimed the work, return timeout/uncertainty and require
  read-only reconciliation rather than retrying the action.

The native listener admits only the fixed v2 state, action, and operation paths in addition to the
existing v1 paths. Authentication uses an exact constant-work bearer-token comparison, and all
identity headers remain required and bounded.

## Evidence and limits

The candidate managed assembly builds with zero warnings against the exact recorded host assembly;
the native crate's six tests, formatting, diff check, and Windows package staging pass. This is
`L1` source/build evidence. It does not prove loader discovery, live host execution, combat
legality, settlement timing, profile safety, restart durability, or gateway/MCP/harness
end-to-end behavior. Those require a separately recorded `LIVE_AUTHORIZATION` and an isolated
disposable profile. No host assembly, save, profile, credential, or generated package is stored in
the repository.

## Review correction (2026-09-04)

The source review replaced the candidate's state-delta settlement inference. Neither a later turn
nor changed energy/pile counts proves completion of a particular queued operation. The current
adapter returns `unknown` after enqueue (including enqueue exceptions), retains its operation and
blocks further v2 mutations until independent operation-bound completion is available. It does
not emit a settlement witness from these host adapters. No such host completion binding has yet
been established; this is an integration blocker, not a successful gameplay result.

Runtime-v2 retains one identity fence and one outstanding-mutation exclusion. Exact semantic
retries ignore transport correlation and JSON formatting; run/combat/player replacement
invalidates generation. This bounded observation is not a complete game-state revision
or a game-rule parity claim.
