# ADR 0019: Repeat-seed practice replay

- Status: Accepted for the managed settings implementation; exact-host behavior unverified
- Date: 2026-09-04

## Context

The installed STS2 v0.107.1 custom-run seed path accepts a supplied seed; standard mode has a
separate protected path that does not allow seed changes. Agents need a deliberate way to restart
the active custom run from the same seed without editing a save or creating a history record.

## Decision

Add an addon-owned, persisted `allow_repeating_seeds` setting, defaulting to `false`, and keep the
one-shot replay/reset action separate from that toggle. The action is fail-closed unless the host
has an active single-player, one-player Custom run. Standard, daily, and multiplayer runs are not
eligible and are never converted into practice runs.

The action requires explicit confirmation, snapshots only host values needed for a new run, queues
on the Godot main thread, performs host-owned graceful cleanup and current-run deletion, and invokes
the host's `NGame.StartNewSingleplayerRun` with the original seed, captured character/acts/modifiers,
captured ascension, and `GameMode.Custom`. It does not call the end-of-run history path. Restarting
from the seed beginning is the supported behavior; later-floor reset is out of scope until a
checkpoint identity and integrity contract exists.

No hard settings-framework, Rust, gateway, MCP, harness, or protocol dependency is added. The
managed/game boundary remains authoritative for host state, mode protection, thread affinity, and
save lifecycle. No direct save-file access is permitted.

## Consequences

- The normal setting is safe by default and remains reversible without changing game progress.
- A confirmed replay intentionally discards the active run's progress; the UI states this before
  the host cleanup begins.
- The operation is useful from in-game settings, but it is not a history browser or exact
  later-floor replay system.
- Build evidence can validate the managed API shape; only a disposable exact-host test can validate
  the UI, lifecycle, history behavior, and mode isolation.

## Evidence status

The custom seed-input path and host API shape were inspected against the installed host. The
managed loader now builds against that exact host with warnings treated as errors. Runtime and
end-to-end gameplay evidence remain unverified because no game launch or valued-profile mutation
was performed in this implementation pass.
