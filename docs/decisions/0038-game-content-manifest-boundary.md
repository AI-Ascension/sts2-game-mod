# ADR 0038: game-content manifest boundary

- Status: Proposed; source-derived design only
- Date: 2026-09-13
- Tracking: game-mod #83

## Context

The existing loader `BuildManifest` and Workshop package manifest identify this adapter package;
they do not identify the installed game's definitions.  Treating either as a game-content manifest
would make two different content sets appear comparable.  Conversely, reading arbitrary installed
files or constructing playable objects to discover content would exceed this target's authority.

The protocol owner published the game-information-query-v1 candidate in
[PR #49](https://github.com/AI-Ascension/sts2-protocol/pull/49), merge
`34f68b182c09472c3a0573ff478e17e6ed53c91f`. Its ADR remains proposed; publication does not
establish runtime admission or a manifest transport mapping. This decision defines only the
game-owned extraction boundary and acceptance inventory. The source-only producer added in
game-mod #120 implements owner-local definition IDs and revisions without a public wire shape,
registry reflection, or native capability claim.

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

The semantic content-set revision includes registry family identities and counts, but excludes
whether this adapter can project each family. Adapter support belongs to the inventory revision.
Expanding support for an unchanged installed catalog therefore preserves semantic and text
identity while changing inventory identity and invalidating dependent cursors, even for an empty
family. This source-only canonicalization correction changes previously produced semantic hashes;
old owner-local bindings expire rather than being translated. No shared schema or pin changes.

For each supported definition family, the inventory records a stable owner-defined kind, count,
ordering rule, completeness state, and unhandled-family result.  For each definition it records a
namespaced opaque ID, origin package identity/version when supplied by the host, override-chain
references, semantic revision, and separately scoped text revision.  Missing host provenance is
represented as `unknown`, never guessed from display names or disk layout.

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
| Same catalog, changed adapter family support | same semantic/text revisions; changed inventory and expired cursors |
| Added, removed, or reordered package | changed content-set revision and expired old cursors |
| Override of one definition | changed provenance/semantic revision for that definition |
| Duplicate display names | distinct opaque IDs; no display-name deduplication |
| Unknown package version or family | explicit unknown/unsupported completeness, not invented provenance |
| Catalog changes during extraction | bounded unavailable result; no mixed manifest |

An authorized exact-host run must additionally compare extracted counts with the owner registry and
record the exact build, mode, supported families, and exclusions.  It is separate evidence and
does not authorize putting proprietary content or package bytes into the repository.

## Integration and limits

Only after the protocol owner accepts the versioned query/identity contract may game-mod map this
inventory into a bounded transport result.  Gateway, MCP, and harness remain responsible for
their respective authenticated delivery, tool, and replay boundaries.  Until then, callers have
no game-content-manifest capability.

This decision does not enumerate game content, inspect a host, grant access to assets, mutate
game state, or validate native registry APIs.  All native facts remain unverified until the
separate exact-host acceptance evidence is recorded.
