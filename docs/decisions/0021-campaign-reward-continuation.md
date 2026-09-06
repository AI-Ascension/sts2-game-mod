# ADR 0021: Campaign reward continuation

- Status: Proposed; bounded Windows reward-to-next-combat integration verified
- Date: 2026-09-06
- Owner: sts2-game-mod

## Decision

Consume protocol ADR 0012's coordinated continuation revision, including `proceed`,
`confirm_selection`, and `cancel_selection`. The schema digest is
`8e99cea36b7ede97532348fd8efe302ca79260895265a7bf14ddf7e006d8ff63`.
All consumers must migrate together; earlier session envelopes fail closed.

The campaign host adapter projects the visible top reward overlay and card reward selection.
Only enabled, visible native controls become legal actions. Unknown overlays block mutation.
Duplicate reward identities are excluded. Selection IDs append an identifier-safe label derived
from the visible card title; both observation choices and action arguments use that exact ID.
The prefix retains unique host card identity even when labels collide or contain no ASCII letters.
Host objects remain on the host thread.

Reward choice invokes the native reward button. Its completion requires the retained reward
to report successful selection or the card reward selection screen to open. The latter settles
the choice that opened the screen so a subsequent card choice can be admitted. It does not
claim that the reward has already entered the deck. Card choice completion requires that
the selection screen close and the retained card appear in the player's deck.

Proceed invokes the unique enabled reward proceed button. The host map must open and the
retained button must cease to be clickable. The native map can cover a reward screen that
remains visible in the overlay tree. An open visible map therefore takes projection priority;
the host travel-enabled flag, travel state, modal state and travelable points gate its actions.
Every settlement
also requires the existing generation and operation-bound
witness checks. Synchronous invocation failure retains the operation as unknown; no blind retry.

This candidate does not yet advertise confirm or cancel controls. Wire support alone does not
establish their availability. Shops, rest sites, other selection screens, and campaign completion
remain separate host work.

## Validation

The exact installed Windows host compiles these APIs without warnings. The synthetic managed
handler probe checks all three argument-free continuation actions: extra arguments cannot
mutate, an unwitnessed dispatch remains unknown, replay cannot dispatch twice, and read-only
wait can settle an independently completed operation. These checks do not validate native
reward callback behavior by themselves. The coordinated Windows fixture now confirms five settled
actions through reward collection, card selection, map opening and entry into the next combat.
See [bounded evidence](../evidence/campaign-reward-continuation-20260906.md). Full campaign
completion and other room types remain outside that result.
