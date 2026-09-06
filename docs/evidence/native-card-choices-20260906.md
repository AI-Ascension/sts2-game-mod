# Native card choices and merchant travel: 2026-09-06

## Scope

Confirmed on the authorized disposable Windows v0.107.1 host with real OpenAI Astra
(`gpt-6-astra`), using harness → MCP → gateway → mod → native game controls. No OS input
automation, scripted strategy, forced deck mutation, or proprietary implementation inspection
was used. Profiles and replaced artifacts were backed up before each fixture restart.

## Event selection

Campaign artifact `run-1788685306` resumed a native saved room. The model chose Amalgamator's
combine-Strikes option, then `card:1:Strike` at generation 5. The partial selection settled at
generation 6 and removed that choice from the catalog. The model next chose `card:2:Strike`.
The native event completed at generation 7: both selected identities left the deck, Ultimate
Strike appeared, and deck size changed from 26 to 25 with HP unchanged at 66. The model chose
the event's proceed action and reached the map at generation 8.

The first pick is witnessed through a native highlight-width transition from zero to positive.
Its parent callback remains unfinished and its card remains in the deck. Final completion
requires native screen closure, removal of all selected cards, and parent callback completion.
The diagnostic build that established the public UI binding was replaced by a normal build;
diagnostic dumping and its launch flag were removed.

The same trajectory later entered a merchant through `select_map_node` and settled
`map_room_entered` at generation 58 with the real merchant offers exposed. It then stopped
before a subsequent dispatch because a provider decision was rejected. This remains a recorded
failed controller segment, not a completed campaign.

## Merchant exit

A continuation with the same host session and no unresolved mutation used an explicit provider
plan constraint for card removal. It purchased Shockwave at generation 58, removed a Strike
at generation 59, proceeded to the map at generation 60, then entered the next monster room
from generation 61. All four operations independently settled through native effects. The
14-row `verified-shop-transition.jsonl` is a captured prefix of artifact `run-1788685984`.

The map adapter now waits for the queued move to finish **and** the host destination coordinates
and room replacement to be visible before opening an entered merchant. Its frame awaiter
returns the native Godot signal awaiter directly, preserving host-thread continuation.

## Artifact identities

Artifacts remain outside Git; these digests identify the exact private evidence files.

| Artifact | SHA-256 |
| --- | --- |
| Windows addon DLL | `92b51338aacaede749a97c02aa73d70a3e12c777adc383280e701d9bb601b331` |
| Linux addon DLL, exact-host compile | `e05223a83c3fa07e9ce1226bbd3e30549d670cd1eb8d31a1e35ea4ccd3b6b8f9` |
| `run-1788685306/trajectory.jsonl` | `e29d5b434ab065f2c061a05dc421d09573e56009a7d69b3f8edee792a1248d3e` |
| `verified-shop-transition.jsonl` | `f095bee38b85a615078e9bcbaf850f9eb9c7648bdcb16e12fe77c7862be810e3` |

## Validation and limits

Both exact-host addon builds passed with zero warnings or errors. The managed gameplay
request/receipt/settlement probe, managed source build, Workshop validation, and strict policy
passed. These source checks do not replace the native effect evidence above.

Headbutt's combat selection also completed under real Astra during this implementation series.
Broader combat selectors, event upgrades, arbitrary deck selectors, an uninterrupted campaign
victory, and full fresh-process campaign replay remain unverified by this record.
