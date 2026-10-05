# ADR 0076: Observe profile progress through the host lifecycle

- Status: Bounded implementation direction; pinned build and runtime evidence pending
- Date: 2026-10-05
- Owner: sts2-game-mod

Refs [sts2-game-mod#79](https://github.com/AI-Ascension/sts2-game-mod/issues/79) and
[sts2-harness#103](https://github.com/AI-Ascension/sts2-harness/issues/103). This decision adds
an internal readiness prerequisite. It does not complete either issue's native or cross-repository
acceptance criteria.

## Context

The public `SaveManager.Instance` getter may construct its singleton, and `InitProgressData`
loads or creates progress. Neither is a read-only readiness query. Profile identity alone also
does not prove progress initialization. The loader can attach after startup, so earlier lifecycle
calls cannot be reconstructed safely.

## Decision

The managed loader uses host-provided Harmony hooks for the exact inspected STS2 v0.107.1 host
(`sts2.dll` SHA-256
`a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52`) and its local
`0Harmony` 2.4.2.0 file (SHA-256
`ef1898322c9f5c86dc1b0758b272a9c440823b4a41ca9a0b82a3aa6b3d206387`). The project references
that host-provided file with `Private=false`, checks that it exists beside the operator-supplied
host assemblies, and never packages it. The runtime verifies both loaded assembly versions and
file hashes before patching. A mismatch or missing API leaves readiness unavailable.

After launch-contract admission and before profile-touching features or listener startup, the
entry point installs one independent `SceneTree.ProcessFrame` observer and explicitly registers
five lifecycle targets under a unique Harmony owner id. Patch metadata is checked for exactly the
expected patch method and one matching owner on each required target and callback kind; unrelated
owners are left intact. Subscription does not establish thread ownership: only an invoked ProcessFrame
callback records the owner thread. A thread change, missing observer, unsupported host, or incomplete
registration disables this capability while unrelated addon initialization continues.

Prefixes revoke readiness before `InitProfileId`, `SwitchProfileId`, `DeleteProfile`,
`InitProgressData`, and the public `Progress` setter. The synchronous `InitProgressData` postfix
accepts only success with non-null serialized progress and status 0, 8, 9, or 10, or the explicit
missing-file status 2 with failure and no saved data. It retains the exact status as quality. The
missing-file case describes current in-memory default progress; it does not claim that the host
successfully wrote that progress to disk. The finalizer returns the original exception unchanged
and revokes a failed attempt.

Capture and consumption use the same manager instance, initialized profile id, progress object,
generation, load-attempt id, runtime incarnation, and observed owner thread. Host getters are read
only through the exact instance supplied by a lifecycle hook or retained lease; the adapter never
acquires `SaveManager.Instance`, forces initialization, reads private fields, or invents readiness
after a late hook. Nested or reentrant progress loads invalidate the whole active chain; readiness
can return only after a later distinct nonnested load completes. If startup initialization preceded
installation, readiness remains unavailable until a later fully observed successful load. Only a
copied scalar record crosses the internal managed seam; no host object is sent to a queue, native
code, HTTP, or another process.

## Compatibility and evidence

This is an additive internal managed dependency and host callback. There is no route, protocol,
native ABI, package payload, save format, or profile mutation change. Harmony is supplied by the
operator's exact game installation; its binary is neither vendored nor distributed. The local DLL
hash identifies the inspected installed file and is not a claim that it was compared with an
upstream release asset.

The production reducer, callback-registration seam, and owned-method verifier have a synthetic
managed probe. The probe does not run Harmony against the host. Until its pinned build is run, source
review and metadata remain source-derived only. A build against the
operator-supplied pinned assemblies would establish compile compatibility only. Patch discovery,
startup ordering, actual lifecycle callback execution, fresh-profile behavior, and seed-context
acceptance remain unverified and require separate authorized evidence.
