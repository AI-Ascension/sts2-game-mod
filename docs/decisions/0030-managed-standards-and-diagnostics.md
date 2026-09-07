# ADR 0030: Managed standards and bounded native diagnostics

- Status: Proposed local implementation; owner review and remote adoption pending
- Date: 2026-09-07
- Scope: Existing managed/native boundary, diagnostics and local validation

## Evidence and decision

The existing managed build uses SDK 9.0.317, nullable/analyzer/warning controls and
C# 12.0. Evaluate those properties in each actual project's Debug and Release
configuration; a properties filename alone does not prove inheritance. Pin the
same SDK with global.json, use its check-only whitespace formatter, and retain
every existing source-linked probe. No target framework or runtime dependency
changes. Later accepted host/settings/replay ADRs qualify the initialization-era
managed language description; this decision does not expand those boundaries.

The source-linked queue probe's synthetic main-loop failure reproduced raw
exception-message logging in StartRuntimeServer: the private sentinel reached
the fake logger and failed the test (exit 134). Remove raw exception messages
from listener start, listener stop and loader initialization logs. Keep the
fixed event text, exception type, existing UI status and failure control flow.
The same source-linked fixture then passes. No real credential leak was observed.
The stop and loader call sites share that inspected logging pattern; a live host
was not used to reproduce those paths.

Extend the actual queue/ABI probe with field widths/offsets, C calling convention,
null and oversized-pointer metadata rejection before dereference, bounded UTF-8
output and preservation of caller bytes on refusal. Disposable copies of the
production source mutate callback layout and calling convention; both must fail
the probe. Tests own and release their buffers. Native-sized pointers/lengths
remain the existing ABI and are not changed to new wire fields.

## Limits and rollback

Source/probe validation does not prove arbitrary foreign-pointer validity,
exact-host signatures, native library unload, Windows process behavior on Linux,
or gameplay. Successful initialization retains the native library and callback
roots for the existing process lifetime; this work adds no unload lifecycle.
The native listener's stop/join contract and no-unwind callback requirement stay
in force. Host builds still require authorized external proprietary assemblies.

Mechanical whitespace, bounded diagnostics/probe changes and gate configuration
are reviewable separately. Revert the relevant local commit to roll back source;
no settings, saves, deployment or service rollback is needed.
