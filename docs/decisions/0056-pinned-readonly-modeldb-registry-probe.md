# ADR 0056: pinned read-only ModelDb registry probe

- Status: Accepted bounded diagnostic probe design; exact-host runtime comparison pending
- Date: 2026-09-16
- Tracking: game-mod #83

## Context

ADR 0038 requires a complete catalog snapshot, per-family owner-registry totals, definition
provenance and override chains, and a coherent generation witness before the content-manifest
route can return success. The exact-host public API inventory does not expose one universal
instance enumerator or a catalog revision. `ModelDb.AllAbstractModelSubtypes` returns model
types, while its public typed collections cover only part of the model families.

Metadata for the pinned STS2 v0.107.1 host (`ReleaseInfo.Version` `v0.107.1`, game commit
`59260271`, `sts2.dll` SHA-256
`a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52`) confirms a private static
field named `_contentById` with closed type
`Dictionary<ModelId, AbstractModel>`. It also confirms public `ModelId.Category`,
`ModelId.Entry`, and `ModelDb.GetCategoryType(Type)` accessors. This is metadata and build
evidence only; no runtime read of the registry has been made.

The repository already uses a few named private fields when a specific host decision confines
their purpose, such as the visible-intent presentation bindings in ADR 0030. That precedent does
not authorize broad reflection. This decision grants only the bounded read described below.

## Decision

The mod may build a separate local diagnostic addon for the exact pinned host. It may read the
existing `_contentById` dictionary only after the probe itself establishes an owner-thread,
post-initialization witness. The probe must fail closed unless all of these guards pass:

| Guard | Required value |
| --- | --- |
| Expected game identity | the probe expects official `ReleaseInfo.Version` `v0.107.1`; this is an expected value, not a live-value claim |
| Binary preflight | the probe hashes the loaded `ModelDb` assembly file and matches the pinned `sts2.dll` SHA-256 above before reflection |
| Field owner/name | exact `ModelDb` type and private static `_contentById` field |
| Field value type | exactly `Dictionary<ModelId, AbstractModel>` |
| Initialization | a `SceneTree.ProcessFrame` callback observes `ModManager.State == Initialized`, a nonempty registry, and identical bounded registry captures on consecutive frames |
| Thread | the probe captures its owner thread ID inside that `SceneTree.ProcessFrame` callback and verifies it on every subsequent callback and during each capture |

The probe has no caller-supplied initialized flag. If a guard fails or its startup witness does not
arrive before the deadline, it returns a bounded unavailable result. Its cancellation and elapsed
time checks are cooperative between entries and after the category-type accessor returns; a
stalled synchronous host accessor cannot be preempted. It does not try alternate
fields, public reflection fallbacks, filesystem scans, constructors, model `GetEntry` methods, or
other host methods discovered at runtime.

For each existing dictionary entry, the probe may copy only the key's `ModelId.Category` and
`ModelId.Entry`, the existing value's runtime type name, and the type name returned by
`ModelDb.GetCategoryType(value.GetType())`. It does not call constructors, `GetCategory`,
`GetEntry`, semantic or localized getters, serializers, or debug formatting. It does not write to
the dictionary or retain host object references after returning.

The probe is diagnostic-only. It has no HTTP/native callback, no route, no product call site, and
does not feed the content-manifest producer. Its owned result uses a fixed ordinal ordering and
hard bounds:

| Bound | Limit | Overflow behavior |
| --- | ---: | --- |
| Registry entries | 65,536 | refuse the whole capture |
| One identity token | 256 UTF-8 bytes | refuse the whole capture |
| Type-name token | 512 UTF-8 bytes | refuse the whole capture |
| All copied strings | 8 MiB UTF-8 | refuse the whole capture |
| Encoded local report | 12 MiB UTF-8 | refuse the whole report |
| One registry capture | 2 seconds | cancel and refuse the whole capture |
| Full probe | 10 seconds or 300 process frames | cancel; detach on the next frame callback and refuse |

The probe checks dictionary count before and after enumeration, verifies copied-entry count, and
rejects malformed or duplicate IDs. An enumeration mutation or any extraction exception returns
one sanitized failure code; the probe checks cancellation during enumeration and never truncates.
Any grouped counts are labeled as counts observed in this one dictionary enumeration, not as
independent registry totals. A linked per-capture cancellation token enforces its 2-second deadline;
the probe cancellation token is also passed to the assembly hash reader, which streams only the
loaded host assembly and checks a 128 MiB file-size ceiling.

## What the probe can establish

On the exact host and after startup, a bounded run can establish that the pinned private field is
present with its expected type, that its current entries can be copied on the host thread without
construction, and what ID/type/category values it exposes at that observation. The operator may
compare these copied IDs and observed counts with the public typed collection surfaces that exist
for selected families. Such comparisons are cross-surface checks only; they are not assumed to be
independent until exact-host evidence establishes how those surfaces relate to the registry.

The probe does not establish completeness of all available model families. A type inventory is not
an instance inventory. `dictionary.Count` is one global count, and counts grouped from the same
enumeration are not independent per-family totals. Public typed collections are incomplete as a
universal family source and their independence from `_contentById` is not yet verified.

`ModManager.GetLoadedMods()` supplies package identities and versions, but the inspected ModelDb
surface supplies no per-definition origin package or override chain. The inspected ModelDb surface
also supplies no registry generation counter or reload event. Before/after counts, a stable key
copy, or an enumerator mutation exception can detect some changes; none is a host-owned generation
witness. The probe therefore cannot supply the independent totals, definition provenance,
override-chain references, or coherent generation witness required by ADR 0038.

## Validation and limits

Source-only fixtures cover exact field owner/name/type guards, build and hash mismatch, thread
mismatch, uninitialized `ModManager` state, empty or unavailable registry, malformed IDs,
entry/string/report bounds, cancellation, deadline expiry, mutation during enumeration, and no
constructor calls. A `Dictionary<ModelId, AbstractModel>` cannot contain duplicate equal keys;
the capture also rejects duplicate copied ID pairs defensively. The separate host-dependent probe
compiles against the pinned `sts2.dll` and `GodotSharp.dll`. These checks establish source behavior
and symbol compatibility, not native runtime behavior.

The implementation lives under
`experiments/managed-rust-interop/content-registry-probe/`; its host-free regressions are in
`experiments/managed-rust-interop/content-registry-probe-tests/` and run in managed source-only CI.
The exact-host compile and synthetic regressions pass. No live registry read has been performed.

An exact-host read-only run, when the unchanged disposable Steam session is available, must run
from the probe's verified post-initialization main-menu callback and retain only the bounded
ID/type/category report outside the repository. It must not enter a run, alter a profile, install
unrelated packages, or emit semantic or localized content. That run may confirm the field and
captured values for that exact host. Any comparison with public typed collections is separate
follow-up evidence.

ADR 0038 remains unchanged in its acceptance requirements. The owner route continues to return
`missing_capability/source_unavailable` until a supported source provides independent per-family
totals, provenance and overrides, and a coherent generation witness, and exact-host acceptance
records those results. The Steam session is currently unavailable, so no runtime registry claim is
made.
