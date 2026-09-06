# Campaign host candidate

Status: experimental, incomplete, unmerged. This extends the isolated live-combat fixture;
it is not full-campaign evidence or a release claim.

## Behavior

`STS2_LIVE_COMBAT=1` and `STS2_LIVE_CAMPAIGN=1` install an isolated local save backend
and expose setup after video initialization. Standard mode uses the native character lobby,
host-generated seed, and saving-enabled run path. Practice requires an explicit mode and seed.
Astra can choose Ironclad setup and a visible travelable map point.
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
the visible destination is a known state. A fresh fullscreen practice process subsequently
settled setup, map entry and combat without intervention: 15 decisions, 15 distinct settled
operations, Reward, 75/80 HP. This is one room, not a full campaign.

## Standard progression correction

The earlier practice run displayed the game's achievements/Epochs lock. Its startup explicitly
used Custom mode with saving disabled. Standard mode now enters the host character lobby and
requires Standard mode and saving enabled before reporting startup completion.

Confirmed in the Windows fixture: a standard run with host-generated seed `G346MD49CA` entered
the first room and wrote both `current_run.save` and `progress.save`. A first-launch tutorial
overlap caused a host exception, so that process did not complete combat. The fixture now
disables tutorial overlays through the host preference API. Saved files were backed up.

Confirmed in a later process: `STS2_LIVE_RESUME=1` loaded the standard progress successfully
and resumed the same seed and first-room state through host save APIs. The lock icon was
absent, fullscreen remained 1280 by 800, and Astra continued across combat turns. Resume
does not create a replacement run or edit unlock flags. It is explicit fixture lifecycle
work, not a model-selected strategic action.
The resumed run then settled six Astra decisions and reached Reward with 74/80 HP.

Standard/resume addon SHA256:
`e919b9416b745b80e973dc37c57945a014e27242e1dd618717f90972573bf8ae`.
Resumed fullscreen screenshot SHA256:
`22340879f62228670158bee9bdb43281070e89403e417e42c6aa429bb4b9b2fd`.
Fresh practice trajectory SHA256:
`7cdcb045b06a36060856999d954dcef8ac690167ce2772db579144921916ee2d`.
Private fixture artifacts stay outside Git.

## Validation and limits

- Confirmed: exact Windows host build succeeds with zero warnings and errors.
- Confirmed: managed gameplay request, receipt, settlement, and fingerprint checks pass.
- Confirmed: strict repository policy passes.
- Confirmed: real `gpt-6-astra` selected normal seeded setup, map travel, cards and end turn.
- Confirmed: fresh practice setup through first-combat Reward; standard saved-run restoration.
- Unverified: newly earned Epochs or platform achievements and complete standard-run history.
- Unverified: campaign reward, shop, rest, arbitrary selection, victory, and replay support.
- Unverified: Linux gameplay compatibility and first-launch-after-patch Load Mods flow.

Keep this candidate unmerged until the remaining host surfaces and real execution evidence
are reviewable. Video-menu evidence and the earlier debug-combat replay remain separate.
