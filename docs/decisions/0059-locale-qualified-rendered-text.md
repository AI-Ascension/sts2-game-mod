# ADR 0059: Owner-local locale-qualified rendered text, fallback, and text provenance

- Status: Proposed; source-only owner boundary
- Date: 2026-09-19
- Tracking: game-mod #111
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md), [ADR 0039](0039-field-availability-and-completeness-boundary.md), [ADR 0053](0053-reward-offer-reference.md)

## Context

Rendered game text is not reachable as owned data. An agent cannot ask for one definition's text in
one language and learn which language actually supplied it, which fallback steps were consulted, or
whether the answer was complete. A locale switch is a global, stateful, host-owned operation, so
reading a second language through the game's own UI is not an option: it would change the language
for the running game and for every other reader.

This decision defines an owned source boundary that copies the owner registry's ordered locales,
their explicit fallback chains and text directions, and owner-supplied rendered text into an
immutable catalog that can be read in any supported locale without touching the active game
language. It does not claim a native extractor, a live run read, a locale switch, a transport route,
a gateway/MCP adapter, or exact-host compatibility.

## Decision

`crates/game-mod/src/locale_reference` owns an immutable `LocaleCatalog` produced by
`LocaleCatalogProducer` from a bounded `LocaleCatalogSnapshot` read through `LocaleRenderSource`.
The catalog binds the existing content-manifest cursor and an owner-local producer version, so a
catalog and every reference it produced are fenced to one content and text revision. Reading is
one-way: there is no setter, no locale switch, no profile or configuration mutation, and no live run
read.

### Language-independent identity

`LocaleEntityReference` is the stable `(entity_kind, namespaced_id)` pair. It is deliberately
separate from `LocaleTextReference`, which additionally binds the catalog binding and the locale the
reference was produced for. A localized answer therefore never changes which entity it describes:
the same request made in two languages returns byte-identical entity identities and differing text,
direction, and provenance only. A reference produced by a different catalog binding, or for a
different locale, is rejected as stale rather than silently re-resolved.

### Explicit fallback and completeness

Each `LocaleInput` carries an explicit fallback chain that must start at its own locale and end at
the catalog default. The chain is validated for shape, length, membership in the supported set, and
repeats; a chain that is empty, too deep, unordered, unsupported, or cyclic is rejected rather than
repaired. A repeat is diagnosed as cyclic before the terminal step is judged, because a chain that
revisits a locale is not a simple path to the default and "ends at the default" is not a meaningful
question to ask of it.

`LocaleRenderedText` reports the requested locale, the effective locale that actually supplied the
text, the exact fallback chain consulted, the effective direction, the text revision, the effective
plural category of the entry that supplied the text, the ordered segments, and any declared
placeholder that stayed unresolved. `LocaleCompleteness` is `Complete` only when the requested
locale supplied the text, the served plural category was the one requested (the canonical `Other`
fallback excepted), and every declared placeholder resolved; text from a fallback locale, a
substituted `Unknown` plural form, or any unresolved placeholder, is `Partial`. An exhausted chain
returns the closed `NotFound` error instead of an empty string, a zero, or an invented description,
and a caller-supplied placeholder with no usable value is carried as
`LocaleUnavailableReason` rather than collapsed. Only the explicit plural order requested, then
`Other`, then `Unknown`, is consulted: an entry stored under some other form alone is never
silently served for a different request.

### Placeholders and preserved numbers

`LocaleTextSegment` distinguishes literal text, a substitution point naming a required placeholder,
a typed reference to another definition, and an effect amount. A declared placeholder that appears
in no segment, or a segment naming an undeclared placeholder, is rejected, so the declared set and
the used set cannot drift. `LocalePlaceholderValue` is localized text, a numeric amount, a stable
reference, or an explicit unavailability with reason. A numeric amount is carried exactly as the
owner rendered it and is never reformatted, so localization cannot silently change a number; an
effect amount without a rendered magnitude is rejected. A placeholder with no usable supplied value
stays visible as `LocaleRenderedSegment::UnresolvedPlaceholder(name)` together with the input it
needs, and its name is reported in `unresolved_placeholders`.

### Presentation is preserved, not interpreted

Owner-supplied text is normalized and validated as presentation, not parsed as markup: ordinary
markup, non-Latin text, and right-to-left text are preserved exactly, while embedded control
characters, executable script presentation, and script-scheme content are rejected. A locale's
reported direction is carried as owner-declared metadata; it is never inferred from the text.

### Locale-partitioned listing

`LocaleEntryListQuery` filters by locale and entity family, and `LocaleEntryPage` returns summary
rows with a bounded continuation. `LocaleContinuation` is bound to one locale, one catalog revision,
and one query, so a page walk can never silently mix two languages or survive a text revision
change; a continuation presented to another locale, another revision, or a changed query is
rejected. Pages are deterministic across repeated reads, and rendering never mutates the catalog.

## Evidence and limits

Synthetic fixtures cover a multi-locale catalog with per-locale direction and explicit fallback
chains, a requested-locale hit and a fallback hit, plural form selection with fallback to the other
form, dynamic placeholder substitution, a supplied reference placeholder, unresolved placeholders,
non-Latin and right-to-left samples, rich markup, effect amounts carried verbatim, locale-partitioned
pagination with single-use continuations, cross-locale and cross-revision continuation rejection,
stale-reference rejection, and tenant-free read-only projection.

Rejection fixtures prove that a duplicate locale, a duplicate entry, an absent default locale, an
empty/oversized/unordered/unsupported/cyclic fallback chain, an entry for a definition absent from
the manifest, a reference segment to an absent definition, a manifest mismatch, a producer-version
mismatch, an undeclared or unused placeholder, an effect amount without a rendered number, an
oversized text, too many segments, and executable or control-character presentation are each
rejected with a closed error. Concurrent locale reads in two languages return correct per-locale
text and identical entity identities without changing any active game language, and a content/text
revision change invalidates a cached rendering instead of serving stale text.

These prove deterministic local validation and read-only projection only. Native rendered-text
extraction, live run reads, locale switching, thread affinity, shared transport/gateway/MCP
delivery, and exact-host compatibility remain unverified.
