# Campaign host candidate

Status: experimental, incomplete, unmerged. This extends the isolated live-combat fixture;
it is not full-campaign evidence or a release claim.

## Behavior

`STS2_LIVE_COMBAT=1` and `STS2_LIVE_CAMPAIGN=1` install the existing isolated save backend
and expose setup after video initialization. Astra can choose Ironclad setup and a visible
travelable map point. The host starts a seeded custom run through its normal run API.
Map travel uses the host action queue. Event options use their host completion task.
No OS keyboard or mouse input is generated.

The host retains each exact operation before submission. Completion requires successful
host work, an action-specific postcondition, a fresh generation, and a known destination
state. A generation change alone cannot prove action completion.

## Readiness regression

Confirmed on the Windows fixture: after end turn, the host's legal catalog changed from
empty to actionable while the observation generation remained unchanged. The generic
JSON fingerprint omitted internal readiness properties. The harness correctly refused
to invent a newer generation and eventually timed out.

The fingerprint now explicitly includes readiness, turn index, node, shop items, and
legal catalog. The synthetic managed probe checks unchanged content, independent readiness
changes, turn/node changes, and catalog-only changes. This is source-test evidence.

The first queued map attempt also produced an interim Recovery observation after its
host task completed. The harness rejected that settlement. The candidate now waits until
the visible destination is a known state; this additional guard still needs a fresh
process test.

## Validation and limits

- Confirmed: exact Windows host build succeeds with zero warnings and errors.
- Confirmed: managed gameplay request, receipt, settlement, and fingerprint checks pass.
- Confirmed: strict repository policy passes.
- Confirmed: real `gpt-6-astra` selected normal seeded setup, map travel, cards and end turn.
- Unverified: uninterrupted setup-to-combat completion with the final candidate.
- Unverified: campaign reward, shop, rest, arbitrary selection, victory, and replay support.
- Unverified: Linux gameplay compatibility and first-launch-after-patch Load Mods flow.

Keep this candidate unmerged until the remaining host surfaces and real execution evidence
are reviewable. Video-menu evidence and the earlier debug-combat replay remain separate.
