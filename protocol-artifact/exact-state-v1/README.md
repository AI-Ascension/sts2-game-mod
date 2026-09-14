# `asc-jcs-state-v1` owner vectors

This directory pins a small, synthetic subset of the exact-state vectors supplied by
`sts2-protocol` at revision
`8a2e66f5d2190a0fca7f146dc3508e8d55515ea7`. The selected vectors exercise object-key ordering,
safe and tagged numeric values, signed zero, unchanged public observation with changed hidden
state, and exact-state/blob identities. `golden-manifest.json` separately pins the
`ascension.checkpoint_manifest.v1` envelope and its manifest-derived checkpoint identity,
including compatibility, coverage, restore, boundary, and origin references.

The fixture is a consumer witness, not a second canonicalization implementation. The protocol
target owns restricted RFC 8785 canonicalization and its complete conformance suite. This copy
only checks that the game-mod boundary keeps the pinned canonical bytes and domain-separated
identities intact.

`selected-vectors.json` includes all 14 `reject_raw` entries from
`conformance/fixtures/exact-state-v1/canonical-vectors.json` at the pinned revision, including
uppercase/dash keys, nonfinite numbers, a lone surrogate, trailing comma, and leading BOM.
Key/depth/byte boundary tests follow that revision's independent
`tools/exact-state/canonical.mjs` witness and `schemas/exact-state-v1.schema.json`.

All values are synthetic. No game host, native assembly, save, profile, credential, or live
capture is included. Native capture remains unavailable until the exact-host evidence and
coverage gates in ADR 0037 and ADR 0043 pass.
