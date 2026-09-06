# Focused native hand selection fixture

This host-dependent fixture is excluded from normal addon builds. It exists to reproduce
Armaments' native hand selector without replaying a long campaign for each diagnostic.
It creates a native weak encounter and generates one Armaments into the hand. Never use
its trajectory as normal campaign, deterministic seed replay, or training evidence.

Build the loader with `-p:EnableHandChoiceProbe=true` and an explicitly authorized exact
host assembly directory via `STS2GameDataDir`. The ordinary isolated-profile checks remain
mandatory. Runtime additionally requires `STS2_HAND_CHOICE_PROBE=1`, campaign mode enabled,
practice mode, and the exact seed `AIASCENSIONHANDTEST1`. Preserve the profile and addon
before installation and label every resulting artifact as a generated-card test fixture.

Wait for the native `fixture ready` log before starting a test client or provider. The
fixture uses the public generated-card command only after the native hand is ready.
It does not choose a card, upgrade it directly, or synthesize input. Test actions must
still use the ordinary runtime controls, and completion needs a settled upgrade witness.

Remove the fixture build and its launcher configuration before ordinary campaign work.
A normal build omits this source file and its initialization hook. Native fixture success
is separate from normal campaign progression, Windows compatibility, and full replay.
