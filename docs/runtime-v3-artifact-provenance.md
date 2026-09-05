# Consumer provenance

The eight upstream artifact files, source schema, and conformance case are copied
byte-for-byte from [sts2-protocol commit be0f3f2](https://github.com/AI-Ascension/sts2-protocol/tree/be0f3f230911f119dbe8e19c71e8249b22f53e59).
`SHA256SUMS` is unchanged and resolves its two relative source paths in this tree.
The schema digest is
`b37c80f583aeaf4f81ede2083bcfb4129196baf5eb092470e8738173c4b7226c`.

The Rust contract mirror preserves upstream field, decoding, and semantic validation.
Local extensions expose the envelope construction helper, a wait constructor, and
an authorization identity value; unused upstream convenience types are omitted.
These do not alter serialized fields or grant authority. The local artifact tests
hash actual bytes and exercise every upstream golden with the consumer parser.
Upstream ADR 0009's required cross-field and UTF-8 byte bounds remain mandatory.

This is an unpublished proposal migration, not a released contract replacement.
It is not wire-compatible with the older same-named card-play proposal. Source and
inert contract tests do not establish licensed-host or complete-system readiness.
