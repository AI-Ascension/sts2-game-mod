# ADR 0056: pinned read-only ModelDb registry probe

- Status: Accepted bounded diagnostic extraction; content-manifest acceptance pending
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

The exact-host metadata also marks `ModelDb` as `BeforeFieldInit`. Its type initializer constructs
an empty `Dictionary<ModelId, AbstractModel>` and stores it in `_contentById`; the initializer has
no model-construction call. Therefore `FieldInfo.GetValue(null)` may allocate and assign that
empty BCL dictionary, but does not invoke `ModelDb.Init` or construct model definitions.
`ModelDb.Init` is separate and calls `Activator.CreateInstance`; its call site is
`OneTimeInitialization.ExecuteEssential`. `ModManager.Initialize` is called from
`<ExecuteVeryEarly>d__7.MoveNext`, and public `ModelDb` metadata exposes no initialization-status
accessor. The public `ModelDb.GetCategoryType(Type)` method's pinned call-target inventory is
limited to `System.Type.get_BaseType`, `System.Type.GetTypeFromHandle`, and
`System.Type.op_Inequality`; it has no model-construction or host-mutation call target. These
metadata summaries describe the exact hash-pinned binary without copying or retaining proprietary
method bodies or IL.

The repository already uses a few named private fields when a specific host decision confines
their purpose, such as the visible-intent presentation bindings in ADR 0030. That precedent does
not authorize broad reflection. This decision grants only the bounded read described below.

## Decision

The mod may build a separate local diagnostic addon for the exact pinned host. It may read the
existing `_contentById` dictionary only after the probe verifies the exact host binary and field
shape. Reflection may run the pinned type initializer, which only allocates and assigns the empty
dictionary described above. The probe must fail closed unless all guards pass:

| Guard | Required value |
| --- | --- |
| Expected game identity | the probe expects official `ReleaseInfo.Version` `v0.107.1`; this is an expected value, not a live-value claim |
| Binary preflight | the probe hashes the loaded `ModelDb` assembly file and matches the pinned `sts2.dll` SHA-256 above before reflection |
| Field owner/name | exact `ModelDb` type and private static `_contentById` field |
| Field value type | exactly `Dictionary<ModelId, AbstractModel>` |
| Static type initializer | exact pinned metadata shows only empty `Dictionary<ModelId, AbstractModel>` allocation and assignment; this may occur during the field read |
| Registry read | only after the binary pin and exact owner/name/private/static/closed-type guards pass; a null or unavailable value refuses |
| Readiness observation | a `SceneTree.ProcessFrame` callback observes `ModManager.State == Initialized`, a nonempty registry, and identical bounded owned-value captures on consecutive frames; this is an observed partial snapshot and stability interval, not proof of `ModelDb.Init` or content-loading completion |
| Thread | the probe captures its owner thread ID inside that `SceneTree.ProcessFrame` callback and verifies it on every subsequent callback and during each capture |

The probe has no caller-supplied readiness flag. Its report labels the result as an observed
partial-registry snapshot and names the loader-state/two-capture basis; it explicitly states that
the basis does not establish `ModelDb.Init` completion or catalog completeness. An empty registry
waits until the fixed deadline and then returns a bounded unavailable result. Its cancellation and
elapsed-time checks are cooperative around synchronous host accessors, sorting, snapshot copying,
and serialization; a stalled synchronous host accessor cannot be preempted. It does not call
`ModelDb.Init`, try alternate fields or public reflection fallbacks, scan filesystems, construct
models, call model `GetEntry` methods, or invoke other host methods discovered at runtime.

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

The bounded capture can establish that the pinned private field is present with its expected type,
that its current entries can be copied on the observed callback thread without constructing
definitions, and what ID/type/category values it exposes during two matching observations. It
labels this as a partial registry snapshot; it does not establish that `ModelDb.Init` or all
content loading completed. Any comparison with public typed collection surfaces is separate
cross-surface evidence and does not establish completeness by itself.

The probe does not establish completion of `ModelDb.Init` or completeness of all available model
families. It reports only the entries present during two matching captures on consecutive owner
thread callbacks. A type inventory is not
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
constructor calls. A synthetic static-field read verifies that reflection can run an owner type
initializer, while the production pinned-field reader test confirms its synthetic initializer only
creates an empty dictionary and does not invoke `ModelDb.Init` or construct definitions. A
`Dictionary<ModelId, AbstractModel>` cannot contain duplicate equal keys; the capture also rejects
duplicate copied ID pairs defensively. The separate host-dependent probe compiles against the
pinned `sts2.dll` and `GodotSharp.dll`. These checks establish source behavior and symbol
compatibility, not native runtime behavior.

The buildable implementation lives under
`experiments/managed-rust-interop/content-registry-probe/`; its host-free regressions are in
`experiments/managed-rust-interop/content-registry-probe-tests/` and run in managed source-only CI.
The exact-host compile and synthetic regressions pass. The probe is not wired to a product route,
and no live registry read has been performed.

An exact-host read-only run remains separate evidence. It must retain only the bounded
ID/type/category report outside the repository and preserve the report's explicit partial-snapshot
readiness basis. It must not enter a run, alter a profile, install unrelated packages, or emit
semantic or localized content. That run may confirm the field and captured values for that exact
host only; it does not establish content-loading completion. Any comparison with public typed
collections is separate follow-up evidence.

ADR 0038 remains unchanged in its acceptance requirements. The owner route continues to return
`missing_capability/source_unavailable` until a supported source provides independent per-family
totals, provenance and overrides, and a coherent generation witness, and exact-host acceptance
records those results. The Steam session is currently unavailable, so no runtime registry claim is
made.
