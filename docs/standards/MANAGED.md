# Managed standards validation

The existing .NET SDK remains 9.0.317 and the language version remains C# 12.0.
global.json selects that same CI baseline even if a newer SDK is also installed;
missing SDKs fail rather than silently rolling forward.
There are thirteen actual project entry points: twelve net9.0 projects and the
net8.0 SessionWindowsBridge. Root and experiment Directory.Build.props stay in
place; presence of a props file elsewhere is not evidence of an actual C# project.
`tools/standards/managed-projects.txt` records the exact entrypoints. Additions or
removals require an explicit inventory update alongside the owner-reviewed change.

Run from this repository with the pinned SDK, Bash and jq available:

```bash
bash tools/standards/check-managed.sh settings
bash tools/standards/test-managed-settings.sh
bash tools/standards/test-managed-abi.sh
bash tools/standards/check-managed.sh format
dotnet run --project experiments/managed-rust-interop/queue-tests/RuntimeQueueProbe.csproj --configuration Release
```

The settings check uses MSBuild-evaluated properties for each project in both
Debug and Release. It requires nullable enable, AnalysisLevel 9.0-recommended,
NET analyzers, warnings as errors, deterministic output, language 12.0 and build
style enforcement. The fixture changes each evaluated property only in Release;
each weakening must fail. Missing, partially removed or zero project inventories
also fail. The fixture includes twelve negative cases and preserves source bytes.
Settings evaluation does not resolve proprietary references or compile a host.
The additional bad-format fixture must fail without modifying its source.

The formatting gate uses SDK `dotnet format whitespace --folder` and explicit
tracked or non-ignored new C# paths. It copies only admitted regular sources and
their EditorConfig/SDK files into a temporary folder, rejecting linked inputs;
the formatter cannot traverse unrelated ignored output or symlink directories.
It needs no restore and does not modify source. An explicit fix
command is `dotnet format whitespace --folder --include <owned-paths>`; never use
it on canonical JSON, external artifacts or historical checksum fixtures. Initial
adoption formatted seventeen C# files separately from behavior and gate changes.
All retained the same non-whitespace bytes. Format validation is separate from
analyzer/build/probe validation.

The retained queue probe now also exercises the actual managed ABI types and
callback: native-sized field offsets, fixed kind width, callback table size,
C calling conventions, null inputs, zero capacity, oversized text refusal before
pointer reads, exception containment, UTF-8 byte bounds and unchanged output on
rejection. Buffers are test-owned and freed in finally blocks. Layout checks
prove the current process ABI, not every architecture or the proprietary host.
They do not establish that an arbitrary nonzero foreign pointer is valid.
The ABI mutation test first runs the real probe against copied owned source, then
changes callback-table layout and calling convention separately. Both negative
versions must fail at their specific ABI assertions, not merely fail to build.

Existing queue tests still cover capacity, FIFO, expiry, cancellation before
dispatch, unknown outcomes during dispatch, immutable late results and redacted
exceptions. Source inspection shows retained managed callback fields and the
native stop/join boundary; safe live unload and concrete host-thread behavior
require separately authorized native/host evidence. Existing probes remain real
programs with assertions, not empty test projects.

The synthetic main-loop failure also exposed raw exception-message logging in
the listener-start catch before its correction. The fake logger rejected the
private marker with exit 134; the corrected production-linked path passes.
Loader/stop logs receive the same inspected removal, retaining event/type
diagnostics and UI status. See [ADR 0030](../decisions/0030-managed-standards-and-diagnostics.md).

PowerShell 7.4.13 parsed the three pre-existing scripts plus the new checker and passed the owned
selected-process guard tests with fakes. `tools/standards/check-powershell.ps1`
parses source without executing it, checks Git's native exit status and refuses
an empty inventory. Its `-SelfTest` checks valid and malformed source. Parsing is
separate from PSScriptAnalyzer or Windows runtime evidence.

## Baseline and evidence limits

At baseline 8b71150895ea95c0625afc1389bb08e4034d9350 all 26 evaluated project/config
pairs had the required settings. All twelve host-free Release projects built.
Eight existing source-linked programs passed: contract, gameplay, host-candidate,
queue, replay, settings, Workshop and Windows bridge tests. On Linux the bridge
explicitly skipped Windows process integration while testing identity and quoting.
The whitespace baseline returned exit 2 with 120 diagnostics across 132 C# files.

GameLoaderProbe requires operator-supplied sts2.dll and GodotSharp.dll and was not
built. No host installation was inspected, game launched, native Workshop operation
sent, provider called or service installed. Exact-host compatibility, Windows
process integration and live unload remain unverified by this standards change.

Rollback the mechanical formatting commit independently from the managed gate and
test commit. Keep the original probes and SDK/project pins; no ABI, wire contract,
runtime dependency or saved settings migration is introduced here.
