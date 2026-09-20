# ADR 0068: Owner-local asset handle, media, and rendition reference

- Status: Proposed; source-only owner boundary
- Date: 2026-09-20
- Tracking: game-mod #112
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md), [ADR 0039](0039-field-availability-and-completeness-boundary.md), [ADR 0060](0060-run-configuration-reference.md)

## Context

An asset is not reachable as owned data. An agent cannot read which icons, art, and audio exist for
a definition, what each one states about its media, or the bytes of a rendition it is permitted to
retrieve. Reading those values through the game's own resource pipeline means driving the game: it
resolves a resource path, decodes it into engine-owned memory, and holds it for a live instance.
None of those effects is recoverable by the reader.

Two failure modes make a naive reference unsafe. A resolution service that takes a filesystem path
or a URL lets a reader name bytes outside the game's own catalog, so the reference would leak the
host filesystem and network rather than the game. A retrieval path that returns whatever bytes a
resource link names lets a reader obtain markup or script that a host may later interpret, so a
retrieved "image" could execute. Both have to be closed by the type, not by a caller convention.

This decision defines an owned source boundary that copies an asset catalog into an immutable
reference that can be listed, read, and have permitted rendition bytes retrieved without resolving
a resource path, decoding into engine memory, or driving the game. It does not claim a native
extractor, a live resource read, an install or extraction action, a transport route, a
gateway/MCP adapter, or exact-host compatibility.

## Decision

`crates/game-mod/src/asset_reference` owns an immutable `AssetCatalog` produced by
`AssetCatalogProducer` from a bounded `AssetCatalogSnapshot` read through a read-only
`AssetReadPort`. The catalog binds the existing content-manifest cursor, the locale, an owner-local
producer version, the content-set revision, and a catalog generation, so a catalog and every read
it produced are fenced to one content revision and one locale. A snapshot whose manifest binding,
locale, producer version, content revision, or generation belongs to another revision is refused
rather than read, and a declared media class that reports no assets produces an explicitly empty
coverage instead of an absent one.

### Opaque handles, not paths

An asset is named by an `AssetHandle`: an opaque identity that refuses to be a filesystem path or a
URL. A handle carrying `/`, `\`, `:`, or `..`, or a character outside a bounded opaque charset, is
refused, and an empty handle is refused as `InvalidInput` rather than being mistaken for a
path-shaped one. A handle cannot be constructed to name bytes the catalog does not carry, so the
reference can never be turned into a host filesystem or network read.

### Media kind, class coverage, and retrieval state

Every asset states an `AssetMediaKind` (icon, art, or audio) that projects to an `AssetMediaClass`
(image or audio), and a `AssetMediaProperties` value carries the media type with its width, height,
or duration. A kind and its properties that describe different media are refused as
`MediaKindMismatch`, and a retrievable or retrieval-denied asset that states no media properties is
refused as `MissingMediaProperties` rather than published as a zero-size asset. Declared per-class
coverage must state each class exactly once and must agree with the observed per-class counts, so a
class the source does not project cannot be listed.

`AssetRetrievalState` deliberately separates *cannot serve* from *does not exist*:
`RetrievalUnavailable` (this boundary cannot serve the bytes) stays distinct from `AssetMissing`
(the installed build does not contain the linked asset). An asset the build does not contain may
carry neither properties nor bytes, and a metadata-only surface offered binary bytes is refused as
`BinaryInMetadataSurface`.

### Field availability and markup refusal

Every entry carries one closed `AssetField` inventory, and each field is an `AssetFieldValue`
carrying its own `AssetFieldStatus`, so an unknown value is never converted into an empty string or
an invented description. No value is available while its status is absent, unsupported, or
withheld; a collection stated available but empty is refused as `EmptyPresentCollection`; and a
field row that is omitted, repeated, or inconsistent with the value the entry carries is refused.
A definition reference resolves against the content manifest, so an asset naming a definition the
manifest does not carry is refused as `UnknownManifestReference` instead of being published as
opaque text.

A media type that a host could interpret as executable presentation is refused: `text/*`, any
`*xml`, and any `*+xml` are rejected as `MarkupMediaType`, and a media-type string that cannot be
split into a type and subtype is treated as markup rather than published. A rendition is therefore
never interpreted as markup or script.

### Metadata only, with rendition bytes behind a private side table

`AssetRenditionDescriptor` states only metadata; it carries no bytes. Rendition bytes live in a
private side table owned by the `AssetCatalog`, and only `AssetCatalogReader::retrieve` emits them,
as an `AssetRenditionPayload` that is either `MetadataOnly` or a `Binary` payload. A caller holding
a descriptor cannot obtain the bytes, so a metadata surface and a byte-bearing surface stay
separate types rather than a flag on one record.

A rendition request is bounded by `ASSET_RENDITION_LIMITS` (4 MiB stored bytes, 4096 pixels,
300000 ms, 16 MiB decoded, and a decode ratio of 64), and `AssetRenditionLimits::new` refuses a
configuration that exceeds those hard ceilings. A rendition that exceeds the stored-byte,
dimension, duration, decoded-byte, or decode-ratio bound is refused as `OversizedRendition`,
`ExcessiveDimensions`, `ExcessiveDuration`, or `InflatedRendition`, so a small stored file cannot
expand without bound when decoded.

### Scoped reads, generation-leased handles, and bounded pages

Visibility is enforced on every read: a `Hidden` asset is observable in no scope, an `OwnerOnly`
asset never leaves an owner scope, and a read outside its scope is refused as `ExcludedByScope`
rather than returning the asset. Each handle is leased to a generation: a handle whose
`expires_at_generation` the binding generation has passed is refused as `ExpiredHandle`, and a
handle absent from this catalog's own handle set is refused as `ForgedHandle`, so a reference from
another catalog or another generation cannot be resolved.

Assets are listed through a single-use `AssetContinuation` bound to the catalog revision, the
query, and the filter that minted it, and to the reader scope; a stale, reused, or
query-mismatched continuation is refused as `InvalidContinuation`, and a page size that is zero or
exceeds `ASSET_MAX_PAGE_ITEMS` is refused as `InvalidPageSize`. `AssetCatalog::list` refuses a
partial page with `PartialPageRequiresReader`, so a page that does not cover every matching asset
cannot be read without a retained reader that carries the bound continuation. An exact lookup is
bounded to one entry and one catalog.

### Read-only by construction

`AssetReadPort::read_assets(&self, ...)` is the only production seam and `AssetCatalog` exposes no
setter. The reader surface offers `list`, `get`, and `retrieve` and nothing else: there is no
resource-path resolution, no decode into engine memory, no install, no extraction, no execution,
and no mutation method. `AssetRenditionAuthority` is exactly `NotGranted`, so every published
rendition states the capability it withholds, and the type makes the alternative unrepresentable
rather than merely discouraged. A shared borrow is enough to read everything, which is what makes
the boundary read-only by construction.

## Evidence and limits

Synthetic fixtures cover icons, art, and audio across image and audio classes, a retrievable asset
with binary bytes behind the side table, a metadata-only asset whose bytes the catalog does not
carry, an asset the boundary cannot serve, an asset the build does not contain, a withheld field,
and an unavailable manifest reference, over one manifest and one locale. Read-only fixtures prove
one source read per production, that a failure leaves the source and catalog untouched, unchanged
retained records after every read, and that every read is reachable through a shared borrow.

Rejection fixtures prove that a manifest, locale, producer-version, content-revision, or
generation mismatch; a declared coverage that does not match the records reported; a duplicate
handle, definition, or field row; a path-shaped or URL-shaped handle; a definition that does not
resolve in the manifest; a kind that disagrees with its properties; a retrievable asset with no
media properties; a markup media type and a non-splittable media type; binary bytes on a metadata
surface; a rendition that exceeds a stored-byte, dimension, duration, decoded-byte, or
decode-ratio bound; a stale, foreign, expired, or forged handle; a page size the catalog cannot
serve; a reused or query-mismatched continuation; and a partial page read without a retained
reader are each refused with a closed error.

These prove deterministic local validation, scope enforcement, bounded retrieval, and read-only
projection only. Native asset extraction, live resource resolution, engine-memory decode, install,
extraction, execution, thread affinity, shared transport/gateway/MCP delivery, and exact-host
compatibility remain unverified.
