# ADR 0015: Runtime-v3 gameplay card candidate

- Status: Proposed for controlled-host validation; source/build evidence only
- Date: 2026-09-02
- Profile: `sts2-protocol/runtime-v3-gameplay`
- Action: `play_card`

## Context

The frozen Runtime-v2 profile intentionally supports only argument-free `end_turn`. A gameplay
action needs a separate contract so existing consumers do not acquire new fields or meanings by
accident. Read-only reflection against the recorded STS2 v0.107.1 host found a concrete card-play
path and bounded combat state symbols, including `PlayerCombatState.Hand`, `CardModel.CanPlay`,
`CardModel.CanPlayTargeting`, `PlayCardAction(CardModel, Creature)`, and
`ActionQueueSynchronizer.RequestEnqueue(GameAction)`.

## Decision

Use `runtime-v3-gameplay` for one bounded `play_card` action. The transport accepts a card index from
0 through 64 and an optional opaque target ID. The mod translates an absent target to the player
creature and an explicit target to a numeric enemy `CombatId`, revalidates the card and target on
the host thread, and queues the host `PlayCardAction`. The candidate reports `unknown` after enqueue and requires independent operation-bound completion
evidence before it can emit `settled` and a `play_card_settled` witness. The gateway, MCP, and harness consumers keep this profile separate from
Runtime-v2.

## Limits

The candidate has protocol, pure-core, managed/native build, gateway, MCP, and harness source or
component evidence. It has no authorized live card-play trace, no support claim for another host
version, and no claim that a returned wrapper value or UI acknowledgement proves a gameplay effect.
No host assembly, save, profile, credential, or generated package is stored in the repository.

## Review correction (2026-09-04)

The source review replaced the candidate's state-delta settlement inference. Neither a later turn
nor changed energy/pile counts proves completion of a particular queued operation. The current
adapter returns `unknown` after enqueue (including enqueue exceptions), retains its operation and
blocks further v2/v3 mutations until independent operation-bound completion is available. It does
not emit a settlement witness from these host adapters. No such host completion binding has yet
been established; this is an integration blocker, not a successful gameplay result.

Both gameplay profiles share one identity fence and one outstanding-mutation exclusion. Exact
semantic retries ignore transport correlation/JSON formatting; card-slot replacement, reordering,
run/combat/player replacement, and observed playability changes invalidate v3 generation. The
bounded observation is not a complete game-state revision or a game-rule parity claim.
