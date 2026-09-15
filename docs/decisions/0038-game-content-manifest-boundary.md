# ADR 0038: game-content manifest boundary

- Status: Accepted owner-local source boundary; native and transport acceptance pending
- Date: 2026-09-13
- Tracking: game-mod #83

## Context

The existing loader `BuildManifest` and Workshop package manifest identify this adapter package;
they do not identify the installed game's definitions.  Treating either as a game-content manifest
would make two different content sets appear comparable.  Conversely, reading arbitrary installed
files or constructing playable objects to discover content would exceed this target's authority.

The protocol owner accepted `game-information-query-v1` in protocol PR #49 (merge
`34f68b182c09472c3a0573ff478e17e6ed53c91f`). This decision defines the game-owned extraction
boundary and acceptance inventory. It introduces
no public route, wire shape, definition IDs, registry reflection, or native capability claim.

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

Before any transport integration, source-only fixtures must establish the following boundary
properties with synthetic catalog values:

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

The accepted v1 envelope binds queries to `content_manifest_id` but does not define a complete
manifest payload. Its closed entity-kind and field-name enums and capability shape cannot carry
active package/version/order, inventory totals/revision, unhandled owner families, or definition
override chains. These facts must not be encoded into `description`, `tags`, or `source_id`.
The `game-information-query-v2` candidate concerns rest reads and is not an admitted replacement.
Full manifest transport therefore needs a protocol-owned compatible extension and named consumer
adoption before game-mod can map this inventory to the supported bounded query path.

Gateway, MCP, and harness retain their authenticated delivery, tool, and replay responsibilities.
This source-only boundary provides no public manifest capability and does not complete issue #83.
Exact-host extraction/count comparison and integrated tool-path acceptance remain separate gates.

This decision does not enumerate game content, inspect a host, grant access to assets, mutate
game state, or validate native registry APIs.  All native facts remain unverified until the
separate exact-host acceptance evidence is recorded.
