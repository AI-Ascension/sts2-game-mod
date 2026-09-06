# Native shop controls and potion capacity

Date: 2026-09-06. Target: sts2-game-mod. Authorized isolated Windows KVM fixture,
game v0.107.1, real OpenAI Astra `gpt-6-astra`, runtime-v3 through gateway/MCP/harness.
No OS input automation was used. Raw saves, trajectories and game files remain external.

## Confirmed host behavior

- The native merchant inventory opened during explicit campaign resume.
- Astra selected Mercury Hourglass at the displayed price of 210. The native purchase
  completed, gold changed from 229 to 19, and the host produced `shop_item_obtained`.
- Astra selected proceed. Native inventory/back/proceed controls opened the map and
  produced `shop_proceed_opened_map`.
- With all three potion slots occupied, a potion reward remained visible but had no legal
  claim action. Astra collected gold, selected Whirlwind and continued through the map.
- A later Windows segment completed 126 settled actions, defeated Vantom, advanced to
  the next act and continued fighting. It stopped at Headbutt's unsupported native
  combat-pile selection. This is campaign progress, not a completed campaign.

## Artifact identities

| External trajectory | SHA-256 |
| --- | --- |
| Relic purchase and shop exit | `5d149e0e7c73cead67284e52cea040faa708ca4a9065afb0985f8488400f2ee3` |
| Full potion slots and continued rewards | `2283360dacfd6ca6cbac8a6ff247a0dd5a7d6a441c7676786bfa02e5917937c7` |
| Subsequent 126-action Windows segment | `382066e2ddd654d48ae2b1d494a57bdba2b9ccce7d70714123da28fe99445ec4` |

The last segment used managed addon digest
`a974f631bee4c3ed814856b16a6b1edf49f454e53ec19e22590c5e84c3606c05`.
Its map diagnostics sampled checks 1, 120 and 600. The final source changes those bounded
diagnostic samples to 1, 10 and 60; that logging-only adjustment is not runtime-verified
by the earlier binary.

## Limits

Native card purchases, potion purchases and card removal compile against the exact host
but have not been exercised in these trajectories. Earlier map operations entered the room
while settlement remained unknown; those episodes stopped without retrying the mutation.
Later map operations settled, but the intermittent failure's root cause remains unverified.
The map helper observes queued completion on host frames, and bounded diagnostics report
queue/work status, error types and destination predicates without exception messages.

Full campaign completion, combat-pile choices, full Linux gameplay and full-run replay
remain separate verification requirements. This evidence does not close them.
