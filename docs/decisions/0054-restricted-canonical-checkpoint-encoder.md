# ADR 0054: restricted canonical checkpoint encoder and conformance witness

- Status: Proposed; source-only encoder
- Date: 2026-09-14
- Tracking: game-mod #80
- Depends on: [ADR 0037](0037-native-checkpoint-coverage-inventory.md),
  [ADR 0043](0043-native-checkpoint-capture-port.md)

## Context

ADR 0043 deferred all canonicalization to the protocol owner and kept the game boundary as an
inert consumer of protocol-validated bytes. Issue #80 requires the game owner to preserve exact
numeric representations, ordered collections, and RNG state in a game-owned payload schema, and to
carry a rejection witness for the values that must never enter an exact-state payload. A consumer
that only checks bytes cannot enforce that contract or localize a game-owned schema defect.

## Decision

`crates/game-mod` now owns a source-only restricted canonical encoder for a game-owned value model
and a conformance witness against the pinned `sts2-protocol` revision
`8a2e66f5d2190a0fca7f146dc3508e8d55515ea7`. The protocol owner still owns complete RFC 8785
canonicalization and its full conformance suite; this target implements only the profile-restricted
subset its payloads may contain and does not claim general JCS coverage.

The value model is `CanonicalValue`: `Null`, `Bool`, `Integer` (safe range `±(2^53 - 1)`), `Text`,
`Array`, `Object` (keys matching `^[a-z][a-z0-9_]*$`, deterministic ordering), `Uint64`, and
`Float64Bits`. Both parser and encoder validate the decoded key grammar; non-ASCII keys return
`NonAsciiKey` and other invalid keys return `InvalidKey`. Object members are ordered by key.
Object and array nesting is bounded by the public `CANONICAL_MAX_DEPTH` constant (64),
as required by the pinned protocol witness `tools/exact-state/canonical.mjs` and schema
`schemas/exact-state-v1.schema.json`. Both the strict parser and the encoder check the
remaining depth before recursive descent, so an over-deep payload returns a typed
`CanonicalError::DepthExceeded` rather than exhausting the process stack; a payload nested to
exactly the limit is accepted. Root containers count as one. Tagged numeric objects count as
containers, so 63 enclosing arrays accept a tagged value and 64 reject it. `Uint64` encodes as
`{"kind":"uint64","value":"<decimal>"}` and `Float64Bits` as
`{"kind":"float64_bits","value":"<16 lowercase hex>"}`, preserving exact values that the plain JSON
number grammar cannot represent. Text escaping is the restricted JCS set: `"` and `\` are escaped,
control bytes use `\b \f \n \r \t` or `\u00XX`, and other UTF-8 is unchanged.

Both entrypoints enforce the pinned 16 MiB `CHECKPOINT_CAPTURE_MAX_BYTES` limit. The parser
rejects oversized raw UTF-8 input, including whitespace, before scanning or allocating values.
Every encoder append checks its remaining byte budget before growing the output, including keys,
delimiters, and escaping expansion. Requested output capacity growth is capped at the same bound.
Exactly 16 MiB is accepted; overflow returns `CanonicalError::PayloadTooLarge`.

The strict scanner ingests restricted JSON and rejects:

| Rejection | Reason |
| --- | --- |
| duplicate object keys | an object cannot bind one key twice |
| floats and exponents | exact numeric representation only |
| negative zero (`-0`) | distinct from `0` and not representable here |
| integers above `2^53 - 1` | outside the exact double-safe range |
| keys outside `^[a-z][a-z0-9_]*$` | pinned profile key grammar |
| input or canonical output above 16 MiB | pinned profile byte bounds |
| trailing text | one value per canonical payload |
| nesting deeper than `CANONICAL_MAX_DEPTH` | recursive descent must be stack-bounded |
| malformed input, unpaired surrogates, raw control bytes, invalid escapes | strict syntax only |

Identities reuse the existing checkpoint constants: `state_id(bytes)` is
`asc-state:v1:sha256:` followed by `sha256(CHECKPOINT_STATE_DOMAIN ‖ bytes)`, and
`blob_digest(bytes)` is `sha256:` followed by `sha256(bytes)`. `tests/checkpoint_canonical.rs`
recomputes these identities, the canonical bytes, the equivalence/distinctness pairs, the reject
matrix, and the golden manifest-derived `asc-checkpoint:v1:` identity from the checked-in
`protocol-artifact/exact-state-v1` witness, which now pins the complete protocol positive,
equivalence, and distinctness set (26 positives, 2 equivalence pairs, 8 distinctness pairs).
`tests/checkpoint_canonical_regressions.rs` adds the grammar and boundary coverage a value-level
witness cannot express (Unicode values versus restricted keys, safe-integer endpoints, escaped
duplicate keys, and the parser/encoder depth boundary) against fixed expected bytes. All 14 pinned
raw rejection vectors are retained.
`tests/checkpoint_canonical_bounds.rs` covers byte boundaries, escaping expansion, key grammar,
hostile nesting, and direct typed integer rejection. `serde_json` is used in tests only to read the vector
file; ingestion itself is hand-rolled so rejections are exact.

## Evidence and limits

This record adds a source-only encoder, a strict parser, and synthetic conformance tests. It does
not inspect a host object, serialize native state, capture a phase, persist an artifact, or restore
a run. Native capture, restore, host compatibility, and phase support remain unverified and
unavailable under ADR 0037 and ADR 0042 until authorized exact-host evidence passes. The encoder and
witness are not evidence that STS2 discovers, loads, or runs the mod.
