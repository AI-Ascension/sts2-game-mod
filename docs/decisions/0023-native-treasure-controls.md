# ADR 0023: Native treasure rewards

- Status: Accepted; bounded Windows native treasure progression confirmed
- Date: 2026-09-06
- Owner: sts2-game-mod

## Decision

Represent the current single-player treasure room as a reward surface. The existing
`choose_reward(reward_id)` contract selects either its unopened chest or a visible relic.
Opening a chest is a separate choice from claiming its contents. Reward IDs identify
the chest or the native relic model; no new contract action, route or ABI is introduced.

Only enabled visible native treasure buttons and relic holders are admitted, with modal
and active-room checks. Chest completion requires the chosen control to become unavailable
and native relic selection, a reward overlay or the room's proceed button to appear.
Relic completion requires a newly owned instance of the retained relic's host model identity
and the selected holder to stop accepting input. The native inventory can clone its display
model, so display-object reference equality is not an ownership requirement.
Proceed requires the native room button and completion requires the visible map to open.
Unknown operations retain their identities and are not redispatched.

Exact-host compilation and strict policy pass. Real Astra decisions opened the chest,
claimed Gorget and opened the map; see [native evidence](../evidence/native-treasure-20260906.md).
Multiplayer treasure voting and relic-specific selection overlays remain unsupported.
