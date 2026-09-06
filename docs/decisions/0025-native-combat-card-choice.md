# ADR 0025: Native card-choice input boundaries

- Status: Accepted for the verified native choice types; broader choices remain unverified
- Date: 2026-09-06
- Owner: sts2-game-mod

A queued card play can enter the host's `GatheringPlayerChoice` state before it finishes.
Waiting exclusively for final completion prevents the model from answering that choice.
The adapter may witness the original operation reaching this specific input boundary when
the exact retained play action is gathering input, its card has left the hand, and the native
combat-pile selection screen exposes enabled card holders. The effect is
`card_play_choice_requested`, distinct from `play_card_settled`.

The model selects a currently exposed card through the existing `select_card` action.
The adapter invokes its native holder and, if needed, the unique enabled native confirmation
control. Completion requires the original retained play action to finish successfully and
the selection screen to close. No card, pile, task completion source or host action state is
edited. Unsupported screens or choices that cannot complete remain unknown and block progress.

This narrow translation reuses the existing selection schema and operation-bound witnesses.
It does not introduce a route, ABI change, alternate strategy or direct host bypass.

An event callback can similarly wait for a deck-upgrade selection. With exactly one retained,
unfinished event callback from an event observation, a new native upgrade screen establishes
`event_card_choice_requested`. The selected card must subsequently increase its upgrade level,
the screen must close, and the exact parent callback and postcondition must complete before
`event_card_upgrade_completed`. The parent request remains retained throughout this sequence.

Some event selectors are attached outside the overlay stack. Discovery may fall back to exactly
one visible `NCardGridSelectionScreen` beneath the current game node. Only supported native
screen types expose actions; multiple visible candidates remain unavailable.

Bounded Windows host verification has confirmed the Headbutt request/selection/completion
sequence with real Astra. Event-upgrade verification remains pending.

Native deck-removal events can ask for several cards. Their public card highlights expose
selection through a numeric shader width. For this typed screen only, the adapter admits a
holder with width zero and requires its native pressed signal to produce positive width before
witnessing a partial selection. The selected holder then leaves the visible choice catalog,
advancing the observation generation. Missing or ambiguous highlight controls remain unavailable.
The adapter does not claim that a partial selection has removed the card from the deck.

On the final selection, the native screen must close, every selected card must leave the deck,
and the exact parent event callback and its postcondition must complete. A unique enabled native
confirmation button may be invoked. The selection effect is `event_card_selection_applied`;
the resulting state distinguishes a partial selection from the completed event. No private host
field, shader value, deck contents, or callback result is modified by the adapter.

Real Astra verified both Amalgamator picks on Windows: two distinct Strikes were replaced by
Ultimate Strike through the host event, and the model continued to the map. This UI binding is
host-version-sensitive and is not evidence for every deck selector. See the
[native choice evidence](../evidence/native-card-choices-20260906.md).

The retained event boundary also supports native deck-enchantment and simple card-addition screens.
For enchantment, the selected card must remain in the deck and acquire a different enchantment
reference or amount. For addition, the offered card must be absent from the deck before selection
and present afterward. Both paths invoke only the native holder and optional unique confirmation
control, require the screen to close, and require the retained parent callback and postcondition
to complete. A different result remains unknown rather than being inferred from screen closure.
Linux Astra verification covered Sapphire Seed enchantment and Brain Leech adding Whirlwind;
see [the event evidence](../evidence/native-event-card-choices-20260906.md).
