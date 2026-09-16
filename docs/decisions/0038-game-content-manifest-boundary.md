# ADR 0038: game-content manifest boundary

- Status: Accepted owner-local source boundary and typed transport mapping; exact-host source acceptance pending
- Date: 2026-09-13
- Tracking: game-mod #83

## Context

The existing loader `BuildManifest` and Workshop package manifest identify this adapter package;
they do not identify the installed game's definitions.  Treating either as a game-content manifest
would make two different content sets appear comparable.  Conversely, reading arbitrary installed
files or constructing playable objects to discover content would exceed this target's authority.

The protocol owner published the `game-information-query-v1` candidate in protocol PR #49 (merge
`34f68b182c09472c3a0573ff478e17e6ed53c91f`). Publication is not admission: the protocol-owned ADR
remains `proposed; not admitted`, and requires at least two named consumers to validate
byte-identical vectors before acceptance. Consumer adoption already exists: gateway commit
`b6b94bf1f1d5dd9a2144e0f83cd5c2785b8a6161` records a
[source/schema pin](https://github.com/AI-Ascension/sts2-gateway/blob/b6b94bf1f1d5dd9a2144e0f83cd5c2785b8a6161/protocol-artifact/game-information-query-v1/manifest.json)
and [synthetic component conformance](https://github.com/AI-Ascension/sts2-gateway/blob/b6b94bf1f1d5dd9a2144e0f83cd5c2785b8a6161/protocol-artifact/game-information-query-v1/consumer-conformance.json).
Those records did not establish formal protocol admission, complete manifest transport, or native
acceptance. This decision originally defined only the game-owned extraction boundary and
acceptance inventory; transport is addressed by the later accepted extension below and still makes
no native capability claim.
The protocol owner has since accepted the typed `game-information-content-manifest-v1` extension in
PR #54 (merge `9581a1b3de49c08b2d46eba8131f1edf0f686ce3`) with the canonical producer fields and a
16 MiB bounded codec. This owner decision consumes that pinned contract for one fixed manifest
read; it does not change the separate query-v1 cursor binding or authorize incomplete native data.

## Decision

When an exact-host implementation is authorized, game-mod will produce one immutable,
read-only manifest from supported host catalog access on the host thread.  The producer copies
owned values before crossing the managed/native seam and never uses a filesystem path, assembly
path, account identity, save, profile, or raw host exception as output.

The manifest must keep these identities distinct:

| Identity | Meaning | May not stand in for |
| --- | --- | --- |
| Game build identity | exact installed host/build compatibility evidence | protocol or adapter revision |
| Adapter identity | game-mod package and extractor compatibility | game-content revision |
| Content-set revision | canonical semantic inventory and package/override inputs | localized rendered text |
| Text revision | locale-qualified rendered text inputs | semantic definition identity |
| Definition provenance | opaque definition identity, origin and override chain | install location or user identity |

For each supported definition family, the inventory records a stable owner-defined kind, count,
ordering rule, completeness state, and unhandled-family result.  For each definition it records a
namespaced opaque ID, origin package identity/version when supplied by the host, override-chain
references, semantic revision, and separately scoped text revision.  Missing host provenance is
represented as `unknown`, never guessed from display names or disk layout.

The source-only producer requires independent owner-registry definition totals for every available
family, copied inside the same generation witnesses as the definitions. Missing totals, extra
families, or a mismatch with extracted definitions reject the entire manifest. This applies to
unhandled families too: an explicitly unsupported projection does not authorize inventing zero
or calling a discovered subset a complete semantic inventory. An independently verified zero is
valid. The map representation permits only one total per family; source adapters must reject
duplicate family evidence before constructing it. Counts alone cannot prove that a host adapter
used the correct registry; that remains an exact-host comparison requirement.

Adapter family support belongs to inventory identity and cursor invalidation, not semantic
content identity. Identical catalog inputs with different support for an available family produce
the same semantic revision and different inventory revisions. This corrects the earlier source-only digest
algorithm: previously issued semantic digests must be regenerated, not compared across algorithms.
No released protocol schema, profile, or artifact pin changes.

The producer invalidates the complete manifest and dependent cursors whenever its observed game
build, adapter compatibility, content package/order, or catalog-generation witness changes.  It
must fail closed when it cannot obtain a coherent catalog generation.  A content reload cannot
reuse an old cursor or silently return a partial catalog as complete.

## Acceptance inventory

Source-only fixtures establish the following boundary properties with synthetic catalog values:

| Case | Required result |
| --- | --- |
| Same canonical catalog, different locale | same semantic content-set revision; different text revision permitted |
| Added, removed, or reordered package | changed content-set revision and expired old cursors |
| Override of one definition | changed provenance/semantic revision for that definition |
| Duplicate display names | distinct opaque IDs; no display-name deduplication |
| Unknown package version or unhandled family | preserve unknown version and unsupported projection; require independent registry totals |
| Missing count or partial extraction, including an unhandled family | fail closed, never publish inferred zero or a complete subset |
| Same catalog, different adapter family support | same semantic revision; different inventory revision and expired cursors |
| Catalog changes during extraction | bounded unavailable result; no mixed manifest |

An authorized exact-host run must additionally compare extracted counts with the owner registry and
record the exact build, mode, supported families, and exclusions.  It is separate evidence and
does not authorize putting proprietary content or package bytes into the repository.

## Integration and limits

The mod pins `sts2-protocol` at `9581a1b3de49c08b2d46eba8131f1edf0f686ce3` and maps the
`ContentManifestProducer` result through its schema-validating codec. The fixed owner-local route is
authenticated `GET /api/v1/game-information/content-manifest` with an empty body and the existing
instance, caller, session, lease, epoch, correlation, and locale headers. The owner returns the
full typed manifest on success and echoes correlation in the protocol envelope. Only the protocol's
16 MiB whole-envelope limit is admitted; output is refused as a bounded error rather than
truncated. Semantic inputs and localized text are never sent.

The managed owner path currently returns `missing_capability/source_unavailable`. The exact-host
registry source has not yet supplied all definition families, independent totals, provenance,
overrides, and a coherent generation witness. The partial known-input snapshot reads
`ReleaseInfoManager.Instance?.ReleaseInfo?.Version` directly for `game_build` and fails closed
when it is missing or not a protocol identity token. The existing map source uses the same getter
with its pre-existing `unknown` fallback; the manifest path does not use that fallback.
`ModManager.GetLoadedMods()` is the source-derived loaded-package enumeration used for package
inputs rather than the general `Mods` property. These public accessor signatures are confirmed
from exact host assembly metadata and the managed host build; their live values and package order
are not runtime-verified.

The existing lookup-binding source must continue to refuse until it can consume the exact
`inventory_revision` from this producer output. That field is the only admitted content manifest
identity; neither entity-kind IDs nor a separately reconstructed digest may stand in for it.

Gateway, MCP, and harness retain current-scope authorization, authenticated delivery, tool, and
replay responsibilities. This owner route and synthetic producer-to-wire conformance do not claim
that an installed game can yet provide a complete manifest. Exact-host extraction/count comparison
and integrated tool-path acceptance remain separate gates.

This decision does not enumerate game content, inspect a host, grant access to assets, mutate
game state, or validate native registry APIs.  All native facts remain unverified until the
separate exact-host acceptance evidence is recorded.
