# Native event card choices, 2026-09-06

## Confirmed Linux host behavior

The authorized isolated Linux v0.107.1 fixture ran a real OpenAI Astra `gpt-6-astra` seeded practice
episode with seed `AIASCENSIONV3FULL1`. Sapphire Seed opened `NDeckEnchantSelectScreen`. Astra
selected Strike; the host completed `event_card_enchantment_completed`, and Astra proceeded to
the map. The selected card's public enchantment changed and the retained event callback completed.
The bounded selection excerpt has SHA-256
`c1dda417508d4812266d6be93cf7f1c3a392bf9eb4dc0645fc0949b9691a772e`.

The same episode later stopped with an unknown operation at Brain Leech's unsupported
`NSimpleCardSelectScreen`. That unknown action was not retried. After stopping the fixture,
backing up its profile and addon, and installing the new binding, a fresh process replayed the
155 previously settled actions with matching public gameplay observations and no provider calls.
It stopped before the Brain Leech event choice at floor 9, 39/80 HP, 41 gold, and 16 deck cards.
The replay was a verified prefix, not a complete campaign replay.

A separate Astra invocation continued from that verified checkpoint. It chose Share Knowledge,
selected Whirlwind from the native picker, and proceeded out of the event. The host witnessed
`event_card_choice_requested`, `event_card_added_to_deck`, and `event_choice_completed`.
The selected card was present in the resulting 17-card deck. The nine-row transition excerpt
has SHA-256 `d3ea7e0cc99eec71da7edc3123d72e5533aa2e95cea8f195ec19c73006188e38`.

The Linux addon used for Brain Leech has SHA-256
`c6618ac29597f8e86cd078e4ed2f1810cd1bc62ce7944d1f435fe791ba7c5161`.
No private host field, deck contents, task result, OS keyboard input, or OS mouse input was used to
perform either choice. These bindings translate existing native controls and public postconditions.

## Validation and limits

Both Linux and Windows host-assembly builds passed with zero warnings and errors. The managed
gameplay request/receipt/settlement probe and strict repository policy passed. Windows compilation
is not Windows live verification of these two event types. Other event selectors and full campaign
victory remain unverified by this evidence. The continuing seeded episode is a separate artifact.
