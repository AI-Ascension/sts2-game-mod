# ADR 0031: Native standard seeded-run adapter

- Status: Proposed pending independent live-host verification
- Date: 2026-09-09

## Context

The seeded-run contract needs a game-facing mutation boundary that keeps the native standard
campaign authoritative. A native HTTP route returning `200` or an accepted receipt does not prove
that the requested seed started the intended run. The adapter also needs to preserve the existing
listener authentication, identity fence, host-thread queue, and bounded operation lifecycle.

## Decision

The game-mod native listener assigns callback kind `9` to `start_request` and kind `10` to
read-only reconciliation. It exposes `POST /v2/seeded-run` and
`GET /v2/seeded-operations/{operation_id}`. Both routes retain bearer authentication and require
the six downstream headers: `x-sts2-instance-id`, `x-sts2-caller-id`, `x-sts2-session-id`,
`x-sts2-lease-id`, `x-sts2-lease-epoch`, and `x-sts2-correlation-id`. The operation suffix remains
an opaque bounded identity.

The managed boundary accepts only the native standard context: Ironclad, ascension 0, no
modifiers, `standard_default` act selection, enabled saving, a fresh isolated profile baseline,
and matching game/mod compatibility digests. It computes the ordered acts with the native
`Rng`/`ActModel.GetRandomList` inputs before admission. The mutation consists of selecting the
native character and setting `NGame.DebugSeedOverride` only while the standard lobby's `SetReady`
call consumes it; the override is cleared in `finally`. Custom-run entry points, direct RNG writes,
act mutation, and save deletion remain outside this adapter.

The fresh-profile baseline is a deterministic SHA-256 over a sorted relative-path inventory and
the SHA-256 of each file. It excludes only the root-level `shader_cache/` and `sentry/` subtrees
and root-level `sentry.dat`, with ordinal case-insensitive name matching because these are created
as boot telemetry. Every other path remains an input, including settings, progress, current-run
saves, nested directories with similar names, and unknown files. The baseline is captured before
the seeded-run overlay and is compared through the existing profile identity and digest fence.

Settlement requires a causal native witness. The pre-admission guard requires a null
`RunManager.DebugOnlyGetState()`; after `SetReady` admits the native transition, the first
non-null `RunState` observed by the host pump is retained before the current `NRun` gate. Readback
must continue to observe that same native state object (and the first current run node when
available), the requested context, the canonical RNG seed, native saving, and a fresh Runtime-v3
generation. A later run that happens to use the same seed and context cannot settle the operation;
an instance change returns an unknown outcome. The settled receipt carries the `run_started`
effect witness and the matched observation.

Accepted, rejected, settled, and unknown receipts are retained by operation identity. An unknown
or timeout outcome is never retried as a new mutation; reconciliation checks the same operation
read-only. A timeout may retain a bounded late-readback window, but it never reissues the native
mutation.

## Evidence and limits

The native route tests cover callback IDs and both seeded paths. The source-only context probe
covers canonical digest ordering, native uppercase act IDs, enabled-save context, and rejection of
digest/list mutations. The profile baseline filesystem probe verifies that cache and sentry
telemetry changes preserve the digest while settings, progress, current-run saves, and unknown
files change it. The exact host build compiles against the operator-supplied Linux host
assemblies, and the release package is retained outside the repository. Repository policy, format,
Clippy, workspace tests, managed source probes, and package-path tests were rerun after the
reference-binding change. These checks establish source and compatibility evidence; they do not
establish that a live game has settled a seeded run. A disposable isolated host run must verify the
first-state causal witness, native seed readback, and saved standard campaign before this ADR is
accepted.
