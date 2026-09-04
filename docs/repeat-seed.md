# Repeat-seed practice replay

The AI-Ascension settings tab has an opt-in `Allow repeating seeds` control and a separate
`Replay / reset seed once` action. The setting defaults to off and is persisted in the addon-owned
user-data settings file alongside the existing runtime settings.

## Implemented behavior

The replay action is admitted only when all of these conditions hold:

- repeating seeds is enabled;
- a run is active in the host;
- the run is single-player;
- exactly one player is present; and
- the host identifies the run as Custom.

Standard, daily, multiplayer, and unavailable run states fail closed. The action requires a native
confirmation dialog that warns that current progress will be discarded and no history entry will
be created.

After confirmation, the managed loader snapshots the host-owned character, acts, modifiers, seed,
and ascension on the game thread. It queues one bounded next-frame operation, calls the host's
`RunManager.CleanUp(graceful: true)`, deletes the host's current resume save through
`SaveManager.DeleteCurrentRun`, and starts a new single-player run through
`NGame.StartNewSingleplayerRun` using the captured seed and `GameMode.Custom`. The mod does not
open, parse, or rewrite save files, and it does not call the run-ended/history path.

This is a restart-from-seed operation. It intentionally does not claim to restore the current
floor, combat RNG position, deck state, map progress, or any other later-run checkpoint. A future
checkpoint feature would need its own versioned identity/integrity contract and tests.

The current v0.107.1 custom-run inspection found no duplicate-seed guard in the custom seed-input
path. The checkbox therefore gates the explicit mod-owned replay/reset workflow; it does not patch
standard-mode seed rules or invent a global used-seed database. If a future host adds a duplicate
guard, the direct host API and mode checks must be revalidated before compatibility is promoted.

## Evidence boundary

The managed project builds against the operator-supplied exact host assemblies. That is build
evidence only. The settings UI, confirmation flow, cleanup behavior, replacement run, absence of a
history entry, and protected-mode rejection still require an authorized disposable exact-host
runtime test. Do not use the existing Runtime-v1 overlay probe or Runtime-v2 fake replay tests as
repeat-seed evidence.

Required runtime evidence must record the STS2 version/commit, host assembly hash, Windows x86-64
platform, source and package hashes, disposable profile, exact action sequence, observed seed and
mode, cleanup, and any failed precondition. The shared dirty checkout and valued profiles are not
valid test targets.
