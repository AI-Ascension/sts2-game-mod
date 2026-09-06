# Campaign reward continuation: Windows fixture

Date: 2026-09-06. Scope: authorized isolated Windows KVM fixture, STS2 v0.107.1,
OpenAI Astra `gpt-6-astra`, standard campaign resumed through host save APIs.
This is bounded integration evidence, not full campaign completion or full campaign replay.

## Coordinated contract

The tested candidate uses schema digest
`8e99cea36b7ede97532348fd8efe302ca79260895265a7bf14ddf7e006d8ff63` across mod,
gateway, MCP and harness. Protocol PR #14 owns this continuation revision. Gateway PR #14,
MCP PR #18 and harness PR #23 consume its complete artifact. Earlier revisions fail closed.

## Confirmed native effects

The corrected Astra attempt independently settled five consecutive operations from the saved
reward screen through entry into the next combat:

| Operation | Confirmed successor |
| --- | --- |
| `episode-action-2-1` | Gold 99 to 115; reward screen remains |
| `episode-action-3-2` | Card reward selection opens with three choices |
| `episode-action-4-3` | Selected card enters deck; deck size 10 to 11 |
| `episode-action-5-4` | Map opens with three host-travelable destinations |
| `episode-action-6-5` | Selected destination enters the next three-enemy combat |

Player HP remained 74/80 throughout this prefix. The immutable 16-record trajectory prefix has
SHA-256 `85456040d37aca898d9963ae67089cab1dcb7e16e106a07644ab80c467a5c688`.
The managed addon SHA-256 is
`40ff6445cbf63c2645592fe0b059d19191e48f885cbf223d0b8988087acc307d`;
the native addon SHA-256 is
`703816f91a2dd4b91f75d0aa19c6faf0215d33aef61f13d390451148fcd984e2`.
The fixture capture completed successfully after the host reported an actionable reward state
and before Astra started. Combat continued after the bounded prefix; that ongoing attempt is not
claimed as a completed campaign.

An earlier Astra attempt independently settled three operations:

1. Collect gold: player gold changed from 99 to 115 at 74/80 HP.
2. Open the card reward: the host exposed Setup Strike, Tremble and Blood Wall as choices.
3. Select Blood Wall: the host logged the reward, the retained card entered the deck,
   and deck size increased from 10 to 11.

The attempt's trajectory SHA-256 is
`b53f2815fbd9ed548fbb2b1dc7f9730cdd50c31bd54329099fe023539e6c9a2a`.
Its managed addon SHA-256 is
`52bf74bb479409131069f46b9f18a775ad314a6a670bbc336c2a7542ede9be06`.
The next Proceed operation remained unknown; the runner stopped without redispatching it.
A later diagnostic attempt, using managed addon
`4340684af6060bc197ad327b1c054178c148b95f4a19226ac037df1554219b16`,
confirmed the native map view with read-only inspection and a fresh screenshot.
That later screenshot's SHA-256 is
`47ea3fe6e8aa0fc822cde8744715b6e9a417612aeffc93284d12f39eaea177e2`.
It is visual evidence only, not an operation-bound settlement witness.

## Corrections verified by the bounded integration

Selection choices are identifiers in the neutral contract. The adapter now appends bounded
ASCII title labels to unique card IDs; spaces or raw localized titles cannot invalidate an
observation. The actual managed observation validator covers punctuation, non-ASCII and long titles.
Card selection uses the native card-holder Pressed signal. Its generic hitbox click did not select
a card in this host and is not used as evidence of success.

The map can cover a reward screen that remains in the native overlay tree. The latest adapter
prioritizes an open visible map and uses host travel-enabled state, travelable points and modal
state for action admission. Proceed requires the map to open and the retained reward control
to cease being clickable, in addition to generation and operation witness checks.
The corrected reward-to-map transition and the next map-to-combat action are confirmed by the
five-operation prefix above. Other map contexts and terminal boss rewards remain unverified.

## Verification and preservation

Rust workspace contract tests, formatting, Clippy and strict repository policy passed locally.
The source-linked managed handler probe passed continuation admission, extra-argument rejection,
unknown replay, independent completion and selection-identity checks. The exact installed Windows
host compiled and packaged the candidate with zero warnings. These checks are not native-effect proof.
A subsequent long-label guard reserves room for the action kind and maximum generation in each
action ID. Its regression was reproduced failing and then passed through the real action validator;
the exact-host build also passed. The live prefix above predates this defensive long-label change
and exercised short card titles.

Every replacement stopped only the recorded executable in the isolated fixture. Addon versions,
logs and campaign snapshots were retained, with per-file backup hashes verified. Save contents and
progression flags were not edited. Fresh launches resumed the host's saved checkpoint; they are
separate attempts and are not retries of an unknown operation in the same host process.
The fixture remains fullscreen at 1280 by 800. The runner now verifies a fresh desktop capture
and an actionable host observation before starting the provider. No OS mouse/keyboard or clipboard
automation was used. Recent diagnostics reported about 32 FPS; no 60-FPS result is claimed here.

Shops, rest sites, other selection screens, full campaign completion, progression unlocks,
Linux full gameplay, and full campaign replay remain unverified by this evidence.
