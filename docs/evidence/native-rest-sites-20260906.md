# Native rest-site evidence — 2026-09-06

## Confirmed Windows fixture result

The isolated Train Windows v0.107.1 game ran visibly in the existing RDP session.
OpenAI Astra (`gpt-6-astra`) resumed the campaign at a rest site and selected Bash.
The native upgrade selection completed and the authoritative deck contained Bash+.
The same run proceeded through a Raider fight, rewards, and a later rest site.
Astra selected healing there: HP increased from 53/87 to 79/87. Both rest-site proceed
operations settled with the native map open. No OS mouse or keyboard automation was used.

The trajectory contains 29 distinct settled operation IDs, counting settled receipts and
completed waits. These include one smith, one heal and two rest-site proceed operations.
The first smith settled directly after an unknown receipt; counting only wait-completion
events would omit that operation. Unknown operations retained their original identities.

The final map choice entered the mandatory treasure room. Treasure controls were unsupported,
so that operation remained unknown and the harness exited 2. Live read-only state reported
recovery/outside_combat at 79/87 HP. This is bounded rest-site and campaign-continuation
evidence, not a successful full run or a successful whole harness invocation.

## Artifact identity

Private fixture artifacts remain outside Git. The completed trajectory SHA-256 is
`02aa10bf444fd170fb8f063e63380f437e3d99421d415f566158cd84f684248b`.
The ten-line prefix covering smith, proceed and entry into combat has SHA-256
`b3f6eb758dad578bb705872727ceb3b091a5433a2020232a7007208a41563760`.

Installed files were compared against the staged package before launch:

| File | SHA-256 |
| --- | --- |
| Managed addon | `2c17cdbc69fa0199f9fbe8d9fe72e2da7e78965fd288d76805fbacf3e1c11830` |
| Native library | `ca50250498eb8c844acc484a8082749d7376cecbc5edb468b469ee6fdf64fa64` |
| Addon manifest | `559e177f0b6e5d82fc44f6b086b1e728353b2f6e437f5e8fae98983d85659984` |

## Validation limits

Exact-host packaging passed without warnings or errors; the managed gameplay validation
probe passed; strict policy passed with 240 sized files and no warnings or errors before
this evidence document was added. CI and exact-head merge checks are separate requirements.
Fresh-process save restoration, Linux gameplay, special relic rest options, multi-card
smithing, maximum-HP healing and full campaign completion remain unverified.
