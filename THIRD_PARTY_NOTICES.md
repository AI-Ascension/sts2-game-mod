# Third-party notices

This foundation currently contains original project documentation, repository policy tooling, and
the pre-existing managed/native interop experiment. No proprietary game assembly, game asset, save,
credential, or copied implementation source is distributed.

Cargo dependencies must be locked and reviewed before they are added. Their license and notice
requirements belong here or in a generated release notice derived from the exact lockfile. The
current policy tool has no third-party Cargo dependency.

The optional mod-loading operator tool uses the locked `serde`/`serde_json` packages (MIT or
Apache-2.0) and RustCrypto `sha2` 0.10.9 (MIT or Apache-2.0), with their locked transitive
dependencies. No dependency implementation is copied into project source. Release notices must
include the exact locked dependency licenses with the distributed operator binary.

The source-only exact-restore consumer additionally uses `jsonschema` 0.55.0 (MIT),
`getrandom` 0.3.4 (MIT or Apache-2.0), and `libc` 0.2.189 (MIT or Apache-2.0), with the exact
locked transitive dependency licenses included in any distributed operator binary. The copied
`exact-restore-v1` artifact is MIT-licensed and retains the protocol owner's checksum inventory.

The optional runtime-only ModConfig-STS2 API integration is based on the authoritative repository
at https://github.com/xhyrzldf/ModConfig-STS2, inspected at commit
639eb97fa7824e94a43339913c51433117207d05. Attribution: PiPiFanDev. The upstream project is
licensed under the MIT License. This project bundles neither its source nor binary and has no hard
dependency on ModConfig-STS2.

The names and assets of Slay the Spire and Slay the Spire 2 remain the property of their respective
owners. This project is independent and does not grant rights to game files or trademarks.

## Pure Core catalog dependency

The game-facts catalog admission links `sts2-game-core` from
https://github.com/AI-Ascension/sts2-game-core at exact revision
`5070070784d6b62dabf535a230bcdb46a054e3ef`. Its dependency graph reuses the already locked
serde_json 1.0.151 and sha2 0.11.0 packages (MIT or Apache-2.0). No Core implementation is copied
into this repository. Core's license notice at the reviewed revision follows:

MIT License

Copyright (c) 2026 sts2-game-core contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

The managed progress-readiness observer references host-provided Harmony 2.4.2.0 (the installed
assembly name is `0Harmony`). Upstream Harmony v2.4.2.0 is MIT-licensed by Andreas Pardeike; see
the [upstream license](https://github.com/pardeike/Harmony/blob/v2.4.2.0/LICENSE). The project
does not bundle Harmony or claim that the inspected local DLL was compared byte-for-byte with the
upstream release asset. Operators must supply the exact game data directory at build time.
