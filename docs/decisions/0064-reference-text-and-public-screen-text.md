# ADR 0064: Owner-local reference text and public-screen text

- Status: Proposed; source-only owner boundary
- Date: 2026-09-19
- Tracking: game-mod #109
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md), [ADR 0039](0039-field-availability-and-completeness-boundary.md), [ADR 0053](0053-reward-offer-reference.md)

## Context

Game reference material is not reachable as owned data. An agent cannot search the tutorial, help,
lore, or credits text, cannot read the public UI text a screen currently displays, and cannot learn
what a blocking tutorial or message says without dismissing it. Reading those surfaces through the
game's own UI is not an option: advancing a blocking message or clicking through a tutorial changes
the running game, and the resulting state change is not recoverable by the reader.

This decision defines an owned source boundary that copies the owner's inventoried non-gameplay
reference text and the semantic description of supported public screens into an immutable catalog
that can be searched and read without interacting with the game. It does not claim a native
extractor, a live run read, a transport route, a gateway/MCP adapter, or exact-host compatibility.

## Decision

`crates/game-mod/src/reference_text` owns an immutable `ReferenceTextCatalog` produced by
`ReferenceTextCatalogProducer` from a bounded `ReferenceTextSnapshot` read through a read-only
`ReferenceTextSource`. The catalog binds the existing content-manifest cursor and an owner-local
producer version, so a catalog and every read it produced are fenced to one content and text
revision. Reading is one-way: `read_reference(&self)` copies values, and the seam has no setter, no
click, no confirmation, and no dismissal.

### Inventoried families and explicit unsupported scope

`ReferenceFamily` is the documented inventory: tutorial, help, lore, credits, and the public UI text
a screen displays. A family the owner reported but this producer cannot project stays
`ReferenceFamily::Unknown` and is carried in the catalog rather than dropped, so coverage reports it
in `ReferenceFamily::ALL` order together with the identities of the documents it could not project.
Searching therefore never turns "the owner authored it but this producer does not project it" into
an empty result that reads as "nothing was authored".

### Documents, discovery, and explicit unavailability

One `ReferenceDocumentInput` carries its family, stable `namespaced_id`, authored locale, discovery
policy, text revision, ordered title segments, sections, searchable keywords, and typed references
to other definitions. `ReferenceDiscovery` is the owner-declared read policy: `Open` and
`Discovered` are readable, `Locked` and `Unknown` are not. A document that is not readable is
answered with `ReferenceUnavailableReason::Locked` in place of its text and a `Partial`
completeness, and a document in an unprojected family is answered with
`ReferenceUnavailableReason::UnsupportedScope` and an `Unsupported` completeness. Neither case
returns an empty string, a zero, or an invented description, and neither is reported as `Complete`.

### Supported public screens without interaction

One `PublicScreenInput` carries the semantic screen kind, stable screen ID, authored locale,
whether the screen blocks progress, whether it can be dismissed, its ordered visible text, and its
described controls. The screen kinds are modal, dismissible tutorial overlay, blocking message, and
confirmation; a kind the owner reported but this producer does not classify stays
`PublicScreenKind::Unknown` and is reported as unsupported scope rather than presented as a
complete description.

`ScreenReadEffect` has exactly one variant, `None`, because a read cannot click, confirm, or
dismiss. Reading a blocking message returns what it says and which controls it carries while
leaving the game exactly as it was; the type makes the alternative unrepresentable rather than
merely discouraged. A control whose kind is `TextInput` carries a value that is never read:
`PublicControlRead::value_withheld` records the deliberate withholding and
`PublicScreenRead::withheld_private_inputs` counts it, so a withheld value is distinguishable from
an absent one. A control that declares itself unavailable must state why; `None` at that point is
refused as `MissingUnavailableReason` because it would be indistinguishable from an undescribed
control.

### Presentation is preserved, not interpreted

Text is retained as inert structure: `ReferenceTextSegment` distinguishes literal text, an
emphasis run the owner rendered, an explicit line break, a typed reference to another definition,
and an explicit unavailability with reason. Markup, non-Latin text, and multiline text are carried
verbatim; embedded control characters, executable script presentation, and script-scheme content
are refused as `UnsafePresentation`, and text shaped as an instruction to its consumer is refused
as `InstructionBearingText` rather than repaired, because a reference read must never become agent
authority. Per-document and per-screen retained byte bounds, segment, section, keyword, control,
document, and screen bounds are enforced before anything enters the catalog, and a value that
exceeds one is rejected as `TextTooLarge` or `InvalidInput` rather than truncated.

### Family-partitioned listing

`ReferenceListQuery` filters by family, keyword, and locale, and `ReferencePage` returns summary
rows with a bounded continuation and the whole catalog's coverage. `ReferenceContinuation` is bound
to one family filter, one catalog revision, and one query, so a page walk can never silently mix
two families or survive a text revision change; a continuation presented to another family,
another revision, or a changed query is rejected as `QueryMismatch`, `StaleReference`, or
`InvalidContinuation`. Row order is deterministic across repeated reads, and listing never mutates
the catalog.

## Evidence and limits

Synthetic fixtures cover the inventoried families with per-family coverage, a locked document, an
unprojected family, a dismissible overlay, a blocking message read without dismissing it, a modal
carrying a private text input, an unclassified screen, keyword search over document and section
keywords, family/keyword/locale filters, a bounded page walk, and coverage that travels with an
empty page. Read-only fixtures prove one source read per production, byte-identical repeated
production, unchanged retained definitions after every read, and that every read is reachable
through a shared borrow.

Rejection fixtures prove that a duplicate document, section, screen, or control identity; a
reference to a definition absent from the manifest; a manifest or producer-version mismatch; an
invalid or oversized identity token; text shaped as an instruction; executable presentation; an
empty or unsafe keyword; an unavailable control without a reason; too many segments, sections,
keywords, related references, controls, documents, or screens; an oversized document or visible
text; a zero or oversized page size; a stale, reused, or query-mismatched continuation; and an
absent reference are each rejected with a closed error.

These prove deterministic local validation and read-only projection only. Native reference-text
extraction, live run reads, screen interaction, thread affinity, shared transport/gateway/MCP
delivery, and exact-host compatibility remain unverified.
