# ADR 0022: Native rest-site controls

- Status: Accepted; bounded Windows native smithing, healing and continuation confirmed
- Date: 2026-09-06
- Owner: sts2-game-mod

## Decision

Translate the existing neutral `rest`, `smith(card_id)` and `proceed` actions through the
visible native rest-site controls. No schema revision, route, ABI or provider policy is added.
The current room must be a rest site in the active single-player campaign. Native option
availability, control visibility/enabled state and modal state gate admission.

Healing invokes the native heal button. Completion requires that button to stop accepting input,
the native proceed control to become available, and healing to be observed. A player already at
maximum HP must remain at least at the prior HP. An unrelated generation change cannot settle it.

Single-card smithing exposes only deck cards below their host maximum upgrade level when the
native smith option is enabled and its count is one. The operation retains the model-selected
card, opens the native upgrade screen, selects that exact card holder and invokes its enabled
confirmation control if needed. Frame waits remain on the host thread and are bounded. Closing
the screen is not upgrade proof: settlement still requires the retained card's upgrade level
to increase and the card to remain in the deck. Unknown outcomes reconcile without redispatch.

Proceed uses the native rest-site proceed control and requires the visible host map to open.
All actions retain the existing generation, operation identity and independent witness checks.
Relic-specific rest options and multi-card smithing are not advertised by this candidate.

## Validation

Exact Windows host packaging, the managed gameplay probe and strict policy pass. Real Astra
decisions upgraded Bash to Bash+, continued through combat and rewards, then healed from 53 to
79 HP and opened the map. See [native evidence](../evidence/native-rest-sites-20260906.md).
Fresh-process restoration, Linux gameplay, special rest options and a full campaign remain
unverified by this result.
