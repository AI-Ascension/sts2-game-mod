# Licensing

## Project terms

Original target code and documentation are licensed under the MIT License in [LICENSE](../LICENSE).
Contributors must have the right to submit their material under those terms.

MIT does not grant rights to STS2 binaries, game data, art, music, trademarks, platform files,
personal saves, or host assemblies. An operator may use an authorized local host installation for
build or compatibility work, but those files remain outside this tree and all release archives.

The copied `protocol-artifact/exact-restore-v1/` consumer input is MIT-licensed and retains its
protocol-owner checksums and provenance. The source-only Rust consumer adds locked `jsonschema`
0.55.0 (MIT), `getrandom` 0.3.4 (MIT or Apache-2.0), and `libc` 0.2.189 (MIT or Apache-2.0), plus
their locked transitive dependencies.

## Greenfield and runtime-package rules

This target is original preparation work. Do not copy, vendor, transliterate, or use another
harness implementation as a source plan. The managed/native directory contains an original thin
loader and native companion; it is not permission to distribute the host or to claim gameplay
support. The package is built against an operator-supplied host assembly, which remains outside
the repository and every release archive.

New Rust and managed source includes an SPDX MIT header. Imported, generated, or adapted material
records its origin, license, hash, and regeneration path. Unknown or incompatible licenses block
release until resolved.

The Workshop staging tool packages only the first-party managed assembly, native companion, and
loader manifest. It does not package Steamworks binaries, host assemblies, game data, or preview
assets; operators must verify the licensing and provenance of any externally supplied preview
before publication.

## Dependencies and notices

Review every Cargo or managed dependency before adding it. Keep the exact lockfile, license
information, and required notices aligned with a release. Update
[THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md) when a dependency or fixture introduces notice
obligations.
