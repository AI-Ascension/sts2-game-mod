# ADR 0039: field availability and completeness boundary

- Status: Proposed; source-only owner boundary
- Date: 2026-09-13
- Tracking: game-mod #84

## Context

Current host reflection helpers can return `null` for absent, unsupported, unobservable, denied,
or failed reads.  Projecting every such outcome as empty data would conflate present zero/empty
values with unavailable data and could make an incomplete collection look complete.  The shared
game-information transport contract is still owned elsewhere, so game-mod must not freeze an
external wire representation unilaterally.

## Decision

Game-mod's future owned-value extractor represents each requested field with one explicit local
outcome before any transport mapping.  The exact shared names are negotiated with the protocol
owner; the required distinctions are fixed here:

| Local outcome | Meaning | May not be encoded as |
| --- | --- | --- |
| available | value was observed coherently, including zero or empty | unavailable/null |
| not_applicable | field has no meaning for this entity or phase | empty value |
| not_observed | supported field was outside the current observable surface | unsupported |
| unsupported | owner has no supported extractor for this build/kind | unknown value |
| denied | caller capability/scope disallows the field | reflection failure |
| stale | supplied snapshot/manifest/entity reference no longer matches | fresh result |
| failed | bounded sanitized extraction failure | successful empty result |

Every non-available outcome carries a stable owner reason code, source kind, and the applicable
snapshot or content-manifest reference.  Present values retain their ordinary type so `0`, `false`,
empty string, and empty collection stay distinguishable from an unavailable field.  Raw exception
text, reflection members, host objects, memory paths, and executable expressions never cross this
boundary.

Collection extraction records whether the total is known, returned count, completeness, and an
opaque continuation only when the eventual shared contract permits one.  Over-limit data is split
into an allowlisted detail section or rejected with a typed size outcome; it is never silently
truncated.  Basic-to-detail recovery accepts only owner-defined entity kinds and field groups and
requires the original snapshot plus manifest identity.  A stale reference requires a fresh
observation rather than retrying against a changed host surface.

## Source-only acceptance matrix

The implementation must add deterministic fixtures covering these distinctions before native or
transport enablement:

| Input case | Required local outcome |
| --- | --- |
| observed numeric zero / empty list | available with the original value |
| phase-inapplicable field | not_applicable |
| hidden but supported surface | not_observed |
| unsupported host kind or field | unsupported |
| protected scope | denied |
| wrong epoch, manifest, or snapshot | stale |
| bounded reflection error | failed with sanitized reason |
| unknown field name / arbitrary path | rejected before host access |
| oversized detail / incomplete page | typed size or incomplete result, never complete |

Fixtures must also show that a supported omitted basic field can be recovered through an
allowlisted detail group under the same identity, while an old identity cannot.  These are
source-only requirements; they do not prove native reflection compatibility or external delivery.

## Integration and limits

The protocol owner selects the versioned wire names and gateway/MCP/harness map the negotiated
result through their owned boundaries.  Until that work and exact-host validation exist, no public
detail query or capability is enabled.  This decision does not add arbitrary reflection, a route,
schema, asset access, or a native availability claim.
