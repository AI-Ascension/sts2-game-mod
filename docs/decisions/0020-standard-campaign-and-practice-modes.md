# ADR 0020: Standard campaign and explicit practice mode

- Status: Proposed; candidate implemented, standard save restoration verified, earned unlocks pending
- Date: 2026-09-06

## Context

The campaign candidate inherited `GameMode.Custom` and `shouldSave: false` from the isolated
combat demonstration. Its visible achievements/Epochs lock and disabled saving do not meet
the requirement for a progressing campaign. Seed replay and persistent progression require
distinct entry paths, consistent with ADR 0019's protection of standard runs.

## Decision

Campaign mode defaults to standard with no operator-supplied seed. It enters the native
single-player character-selection lobby, chooses the host character, and asks that lobby
to begin. The host owns seed generation, acts, normal mode and saving behavior. Startup
completion requires the host to report Standard mode and saving enabled.

The fixture retains a persistent, local-only campaign save store under its already isolated
user directory. Progress is loaded after host model registration. A failed progress load
is not reset to an empty profile. An existing run save requires explicit resume; starting
another campaign does not silently discard it. Explicit fixture resume (`STS2_LIVE_RESUME=1`)
uses host load/setup APIs after progress initialization. Tutorial overlays are disabled through
the host preference API for this automated fixture; no unlock state is forged.

Practice requires `STS2_LIVE_CAMPAIGN_MODE=practice` plus an explicit `STS2_LIVE_SEED`.
Supplying a seed in standard mode, omitting a practice seed, or using an unknown mode fails
configuration. Practice retains the earlier separate local demo store and Custom mode.

The addon does not write achievement/Epoch unlock flags or convert a running practice game
into Standard mode. Host progression, persisted saves, later-process restoration, and
platform achievements require separate evidence. A missing lock icon alone proves none
of those outcomes.
