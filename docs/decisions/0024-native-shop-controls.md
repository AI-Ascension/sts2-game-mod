# ADR 0024: Native shop controls

- Status: Accepted; native relic purchase and shop exit confirmed
- Date: 2026-09-06
- Owner: sts2-game-mod

## Decision

Translate existing shop purchase, card removal and proceed actions through the current
single-player merchant's native inventory and purchase API. Stock, affordability, visible
enabled controls and modal state gate admission. Observations expose only the opened native
inventory. Entering a merchant map node or explicitly resuming there opens the merchant's
native inventory as part of that requested navigation; observation never opens it.

Purchases call the host's ordinary purchase wrapper with its cost checks enabled, retaining
the selected entry and requiring successful purchase plus the expected newly owned card,
relic or potion. Card removal selects the retained removable deck card in the native screen
and requires successful host purchase, the removal entry's used flag and that card's absence.
No wallet, stock or deck value is directly edited. Proceed closes the inventory through its
native back control when needed, then uses the room proceed control and requires the map.

The existing operation identities, generation admission and host-thread fences remain.
No schema, route, ABI or provider-policy change is introduced. Exact-host packaging, strict
policy and real-game evidence remain required before merge.

Potion reward admission also requires a free native potion slot. The reward remains visible
when storage is full, but neither its legal action nor dispatch can claim it.

See [native shop evidence](../evidence/native-shop-controls-20260906.md) for bounded host
verification and the remaining card-removal and campaign-completion limits.
