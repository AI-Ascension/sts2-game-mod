# Seeded-run criterion-to-evidence map at exact pins

Date: 2026-09-17
Issue: `AI-Ascension/sts2-game-mod#79` — "Independently verify native standard seeded-run admission
and settlement at exact pins" (open, `status:blocked`).
Status: AC1 `confirmed-source`; AC2, the native half of AC3, and the native half of AC4
`unverified-native`. No game was launched, no addon installed, no profile mutated, no Steam
account touched, and no provider called for this record.
Scope: a documentary map from every acceptance criterion and sub-clause of #79 to the artifact,
test, or receipt that discharges it, at the exact pins below. Every native row stays `unverified`
until the bounded campaign in `coordination/native-acceptance-20260916.md` (private coordination
record, outside this repository) has run and been independently reviewed.

Labels: `confirmed` (executed while writing this record), `source-derived` (read from source at the
pinned commit), `inferred` (recorded elsewhere, not re-observed here), `unverified` (no evidence).

## Exact pins

| Component | Pin | Basis |
| --- | --- | --- |
| `sts2-game-mod` source (this record) | `46b1ac6ed4f57ae62dcd2f6694a5792fdeba95c0` (`origin/main`, 2026-09-17) | confirmed |
| Staged guest addon source | `a8ccb8b7f7b8f061ee336f777dab9ade5ad88e54` (older mod main) | inferred |
| Staged `AIAscensionSTS2GameMod.dll` | `5951ee5d5dbb0b50e154fcdcf0f8efab137187a6632cdcc0143a73a06cf352c6` | inferred |
| Staged `AIAscensionSTS2GameMod.json` | `559e177f0b6e5d82fc44f6b086b1e728353b2f6e437f5e8fae98983d85659984` | inferred |
| Staged `AIAscensionSTS2GameModNative.dll` | `52d2dfc62dd21050532bee48d53fb6c82eb912241ce6dfb30cebd71170955942` | inferred |
| Host game | Slay the Spire 2 v0.107.1, release label `59260271`, Windows x86-64 | source-derived (ADR 0002) |
| Host `sts2.dll` SHA-256 | `a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52` (product `0.1.0+59260271157f76a2896f0eab5bc6ea1245d8b314`) | confirmed against the operator-supplied fixture; matches `runtime-map-v1-host-build-20260907.md` |
| Host `GodotSharp.dll` SHA-256 | `0e4897ecdfb31456a97c7d8028dfb8d7dbdc632e2f73fc9b438d7b266a139289` (product `4.5.1+f62fdbde15035c5576dad93e586201f4d41ef0cb`) | confirmed against the operator-supplied fixture |
| `sts2-protocol` head | `bfe28e455de48d6d9db466bbcf6062ab5d85e9af` | confirmed (GitHub API, 2026-09-17) |
| `seeded-run-v1` schema digest | `5c659f344be78f84e8d783986925d462714f933cac95d18943358992f7d3e2b8` — identical in `protocol-artifact/seeded-run-v1/schema.json`, `schemas/seeded-run-v1.schema.json`, and `protocol-artifact/seeded-run-v1/manifest.json` | confirmed (`sha256sum`) |
| `seeded-run-v1` conformance cases | `84db53f07c515de4743fad73cce11da2aef56991baf340c9300c7dd6cc55ea4b` (`conformance/cases/seeded-run-v1.json`) | confirmed |
| `seeded-run-v1` manifest | `1b504a25c1f1f0cd8bf4a61d07b8457c32e3f481c0810f407d750510c692fc09`; goldens per `protocol-artifact/seeded-run-v1/SHA256SUMS` | confirmed |
| `sts2-harness` head | `f673658065d3f7ec5087afa221536799fe30aa13` (`crates/harness/src/bin/runtime_support/runtime_v3_seeded*.rs`) | confirmed head; seeded sources source-derived |
| `sts2-gateway` head | `804691c5e1b3121075d83b3e31b86606facb5e62` (`crates/gateway/src/seeded_run/*.rs`, `crates/gateway/src/bin/runtime_support/service_seeded_run.rs`) | confirmed head; seeded sources source-derived |
| `sts2-mcp-server` head | `65cb405616ecddfb7bf7b76edf341be30a442ca2` (`crates/mcp-server/src/catalog_seeded_run.rs`, `mapping_seeded_run*.rs`, `protocol_artifact_seeded_run.rs`) | confirmed head; seeded sources source-derived |
| Staged guest gateway binary | source `b6b94bf1f1d5dd9a2144e0f83cd5c2785b8a6161`; SHA-256 `f6d33a883bad60c525b515213fbf695b29ca0e859e41814243212b93811bc118` | inferred |
| Staged guest MCP binary | source `2567cd336bd440ba9e4b22f6e5320e1c01139058`; SHA-256 `73e41fa56a0c55ee9747f180c743c09ac0538433ee23cf119c01645ee7d4ed89` | inferred |
| Staged guest harness runtime | source `4a3301f` (harness PR #227 later merged as `6e2af572093e2f29b96d00ac708cc35b46155d77`); SHA-256 `f612102f02f0c366c725de3aa93718a0f46b33cd247c92351d63a4d7e24edea3` | inferred |
| Staged guest harness platform tests | SHA-256 `772f73dc25482c8b373ce4bd4e6fb95f5d09e00c8ceeacb4d503f908d0f913f2` | inferred |

### Staged set versus current heads

The staged guest set is behind the current default-branch SHAs on every axis: the addon was built
from mod `a8ccb8b7…` (current `46b1ac6e…`), and the companion binaries from gateway `b6b94bf1…`
(current `804691c5…`), MCP `2567cd33…` (current `65cb4056…`), and harness `4a3301f…` (current
`f6736580…`). AC2 requires proof "at exact pins", which is ambiguous while the two sets differ.
Before the native campaign runs, the operator must do exactly one of:

1. Re-pin the campaign to the staged set explicitly, recording the four staged-source SHAs and the
   eight staged binary hashes above in the admission receipt, so that every later claim names
   that set and not the current heads; or
2. Rebuild the addon and the three companion binaries at the current heads (`46b1ac6e…`,
   `804691c5…`, `65cb4056…`, `f6736580…`), record the new built hashes inside the guest, and
   replace the staged files in the disposable campaign root only.

A campaign that mixes the two sets, or that does not record which set ran, does not discharge AC2.
The `seeded-run-v1` schema digest `5c659f34…` is the same in both sets and in protocol head
`bfe28e45…`, so the protocol pin does not need to change.

## Acceptance criteria and evidence

Status column values: `confirmed-source` (repository-owned source, test, or CI evidence exists and
was executed or observed at the pins above) and `unverified-native` (requires the native lane; the
exact gate text is quoted in the last column).

Gate text, quoted from the issue triage of 2026-09-13: "An operator must provide explicit
native-test authorization, an exact supported host/addon build and an isolated disposable
profile." Current state of that gate is in the section "External gate" below.

### AC1 — "A static/disposable probe test establishes argument/authority and data-redaction checks without requiring a game in default CI."

| Sub-clause | Evidence | Status | Basis and remaining |
| --- | --- | --- | --- |
| AC1.a argument/authority: the standard context is admitted, unsupported contexts are refused before host access | PR #113 (merge `b77ed8d396e36b0c6b9a8f9eabb277e13ce7f7c6`): `experiments/managed-rust-interop/seeded-run-tests/StandardAdmissionProbe.cs` over `game-loader/SeededRunStandardAdmission.cs` `HasSupportedStaticContext`; 6 checks — fresh saving-enabled ascension-zero standard admitted; existing-profile, save-disabled, non-standard selection policy, nonzero ascension, and modifier-bearing contexts rejected with `unsupported_standard_context` | `confirmed-source` | confirmed: 6 PASS, exit 0 (this record); CI job "Managed source-only boundary" on run `35184549562` at `46b1ac6e…` success |
| AC1.b data binding and redaction: the selected-context digest binds the context, act order is preserved, mismatches are rejected | `SeededRunContextProbe` over `game-loader/SeededRunStandardContext.cs`; 10 checks (canonical digest matches Rust field order; top-level digest binds selected context; digest mismatch rejected; non-canonical modifier order rejected; native act order preserved rather than sorted) | `confirmed-source` | confirmed: 10 PASS, exit 0; CI step "Run managed seeded-run context digest probe" |
| AC1.c data redaction of the profile baseline: cache/telemetry excluded, settings/progress/save/unknown files included | `SeededRunProfileBaselineProbe` over `game-loader/SeededRunProfileBaseline.cs`; 10 checks | `confirmed-source` | confirmed: 10 PASS, exit 0; CI step "Run managed seeded-run profile baseline inventory probe" |
| AC1.d wire authority: request generation and fingerprint are carried unchanged through start/reconcile responses | `SeededRunProtocolSerializationProbe` over `game-loader/SeededRunProtocolSerialization.cs`; 59 checks | `confirmed-source` | confirmed: 59 PASS, exit 0; CI step "Run managed seeded-run serializer generation probe" |
| AC1.e launcher authority: live authorization and disposable-launcher refusals | `experiments/managed-rust-interop/live-authorization.test.sh`, `session-launcher.test.sh`, `session-launcher.sh --self-test` in CI job "Rust foundation gates" | `confirmed-source` | source-derived here; CI run `35184549562` success. Not rerun for this record |
| AC1.f "without requiring a game in default CI" | `experiments/managed-rust-interop/managed/ManagedInteropSpike.csproj` and the four seeded-run probe projects carry no host assembly reference; the managed-source CI job runs on `windows-latest` with no `STS2GameDataDir` | `confirmed-source` | source-derived; CI success on `46b1ac6e…`. Nothing remains; the AC1 checkbox in the issue body is simply not ticked |

### AC2 — "Authorized native lane proves supported standard setup, canonical seed, causal first state and settled operation at exact pins."

| Sub-clause | Evidence | Status | Basis and remaining |
| --- | --- | --- | --- |
| AC2.a authorized native lane | Owner authorization recorded 2026-09-16 in the private coordination record; addon staged in the disposable campaign root with `host`, `profile`, `addon`, `evidence` children | `unverified-native` | inferred; gate: "explicit native-test authorization, an exact supported host/addon build and an isolated disposable profile" — authorization and staging exist, the interactive Steam session does not |
| AC2.b supported standard setup (Ironclad, ascension 0, no modifiers, `standard_default`, saving enabled, fresh baseline) | Source: `game-loader/SeededRunStandardHost.Preparation.cs` (null-`RunState` pre-guard, selected-context and profile-baseline validation); ADR 0031. Receipt: admission receipt placeholder below | `unverified-native` | source-derived only; gate as above |
| AC2.c canonical seed | Source: `SeededRunStandardHost.Preparation.cs` `DebugSeedOverride` with canonical seed readback; `tools/seeded-run-client` computes the canonical request. Receipt: admission receipt `canonical_seed` field | `unverified-native` | source-derived only; campaign seed `A79TEST20260916` |
| AC2.d causal first state | Source: first non-null `RunState` witness in `SeededRunStandardHost.Settlement.cs`. Receipt: settlement receipt `run_started` witness correlated to the original operation and lease/epoch | `unverified-native` | source-derived only |
| AC2.e settled operation (one legal action through MCP → gateway → mod, settled successor observed) | Companion sources at the pins above. Receipt: action receipt and settlement receipt placeholders | `unverified-native` | source-derived only; "A request acknowledgement is not settlement" |
| AC2.f "at exact pins" | Pin table and "Staged set versus current heads" above | `unverified-native` | the operator must re-pin or rebuild before the run; otherwise AC2 cannot be discharged |

### AC3 — "Duplicate/uncertain/stale attempts do not start another run and preserve explicit outcome evidence."

| Sub-clause | Evidence | Status | Basis and remaining |
| --- | --- | --- | --- |
| AC3.a duplicate: an exact replay returns the original receipt and starts nothing | Source analogue: `SeededRunStandardHost.Idempotency.cs` `IsExactReplay`; probe lines "same fingerprint and request generation are an exact replay", "changed request generation is an idempotency conflict", "changed request fingerprint is an idempotency conflict" (`SeededRunProtocolSerializationProbe`) | `confirmed-source` (analogue) / `unverified-native` (proof) | confirmed probe; gate for the native duplicate attempt with preserved evidence |
| AC3.b uncertain: timeout or lost reply settles to `Unknown` with bounded late readback and is never re-issued as a new mutation | Source: `SeededRunStandardHost.Settlement.cs` `MarkUnknown(pending, "run_start_timeout", allowLateReadback: true)`; no retry path after `Unknown`. No managed synthetic-host settlement probe exists yet | `unverified-native` | source-derived; optional source lane (settlement probe over `queue-tests/SeededRunStandardHostStub.cs`) would add an analogue, not proof |
| AC3.c stale: a stale baseline or generation is refused before mutation | Source: identity fence and generation check in `SeededRunStandardHost.Preparation.cs`; gateway `crates/gateway/src/seeded_run/ledger*.rs` at `804691c5…` | `unverified-native` | source-derived; gate for the native stale-baseline attempt |
| AC3.d explicit outcome evidence is preserved for each attempt | Receipt placeholders below (one per attempt kind: duplicate, lost-reply, stale, uncertain) | `unverified-native` | nothing exists yet |

### AC4 — "Pre-existing profiles/processes remain unchanged and cleanup/rollback is recorded; an unavailable native lane remains unverified and open."

| Sub-clause | Evidence | Status | Basis and remaining |
| --- | --- | --- | --- |
| AC4.a pre-existing profiles unchanged | Source: `SeededRunProfileBaseline.cs` + probe (AC1.c); ADR 0041 disposable boundary. Guest record: original install/profile baseline receipt `bfde2573484d289f9c10b8f61fcebc140d11966430cc1cba009cdc2c5ff64ce2` (356 files, retained privately in the guest) | `unverified-native` | inferred (recorded, not re-observed); remaining: post-campaign re-verification against the same receipt |
| AC4.b pre-existing processes unchanged | Campaign plan: record campaign-owned PIDs; stop only those | `unverified-native` | nothing observed yet |
| AC4.c cleanup/rollback recorded | Cleanup receipt placeholder below; `session-launcher.sh` cleanup tests are the source analogue | `unverified-native` | nothing observed yet |
| AC4.d an unavailable native lane remains `unverified` and open | This record; ADR 0031 status "Proposed pending independent live-host verification"; issue #79 open with `status:blocked` | `confirmed-source` (documentary) | satisfied while the issue stays open and no native row is upgraded without a receipt |

## Commands

### Managed probes (confirmed 2026-09-17 at `46b1ac6e…`)

Run from the repository root with the pinned .NET 9 SDK (`9.0.317`). On a Linux host without
libicu the runtime fail-fasts unless the invariant-globalization switch is set:

~~~text
export DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1
dotnet run --project experiments/managed-rust-interop/seeded-run-tests/SeededRunStandardAdmissionProbe.csproj -c Release
dotnet run --project experiments/managed-rust-interop/seeded-run-tests/SeededRunContextProbe.csproj -c Release
dotnet run --project experiments/managed-rust-interop/seeded-run-tests/SeededRunProfileBaselineProbe.csproj -c Release
dotnet run --project experiments/managed-rust-interop/seeded-run-tests/SeededRunProtocolSerializationProbe.csproj -c Release
~~~

| Probe | Result (this record) | Terminal line |
| --- | --- | --- |
| `SeededRunStandardAdmissionProbe` | 6 PASS, exit 0 | `seeded standard admission checks passed` |
| `SeededRunContextProbe` | 10 PASS, exit 0 | `seeded-run context checks passed` |
| `SeededRunProfileBaselineProbe` | 10 PASS, exit 0 | `seeded-run profile baseline inventory checks passed` |
| `SeededRunProtocolSerializationProbe` | 59 PASS, exit 0 | `seeded-run serializer generation checks passed` |

These are `confirmed` source-only results. They are not native evidence.

### Seeded-run client preparation (operator-run; prints, does not place)

~~~text
cargo run --locked --offline --package sts2-seeded-run-client -- \
  --host 127.0.0.1 --port <listener-port> --token "$STS2_RUNTIME_TOKEN" \
  --caller-id <caller> --instance-id <instance> --session-id <session> \
  --lease-id <lease> --lease-epoch <epoch> --generation <generation> \
  --correlation-id <corr> --operation-id <stable-operation-id> \
  --seed A79TEST20260916 --run-mode seeded_training \
  --schema-file schemas/seeded-run-v1.schema.json \
  --context-id <ctx> --ascension 0 --act <act-1> \
  --selection-policy standard_default --save-policy enabled \
  --profile-baseline-kind fresh --profile-baseline-identity <id> \
  --profile-baseline-digest <sha256-read-back-from-the-mod> \
  --game-identity <simple-name/version> --game-digest a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52 \
  --mod-identity AIAscensionSTS2GameMod/1.0.0.0 --mod-digest <sha256-of-the-installed-mod-dll> \
  --print
~~~

Host-derived values (instance, session, lease, epoch, generation, profile-baseline digest) are read
back from the running mod and are never fabricated. `--mod-digest` must equal the hash of the
addon actually installed in the campaign root (staged `5951ee5d…` or the rebuilt value). Replace
`--print` with `--send` only inside the authorized campaign; the composed harness → MCP → gateway
path is the accepted route for the settled action, and the client is preparation only.

### Steam usability preflight (read-only, run inside the Windows guest)

~~~text
powershell -NoProfile -NonInteractive -File experiments\managed-rust-interop\steam-usability-preflight.ps1
~~~

The script emits exactly one JSON object `{"state": "<token>"}` and never starts, stops, logs
into, or configures Steam. Tokens: `absent_client`, `process_only_unready`,
`process_present_account_unverified`, `process_present_account_indicated`. Last observed (guest
agent, 2026-09-17, recorded in the coordination record): `absent_client`, with a separate
interactive-session classifier reporting `interactive_session: not_observed`. The campaign may
start only after the preflight returns `process_present_account_indicated` and the classifier
reports an interactive session.

### Receipt placeholders (to be filled by the native campaign; all currently `unverified`)

Each receipt is archived under the campaign root's `evidence` child, sanitized (no account names,
private paths, save contents, or model output), and its SHA-256 is recorded here by a follow-up
change. Until then every field below reads `pending`.

| Receipt | Required fields | Value |
| --- | --- | --- |
| Admission | pin set used (staged or rebuilt) with all source SHAs and binary hashes; host `sts2.dll`/`GodotSharp.dll` digests; disposable profile identity and baseline digest; original baseline receipt digest; operation id; lease id/epoch; host generation; `canonical_seed` read back; selected-context digest; preflight state; classifier state; UTC start | pending |
| Action | operation id of the one legal action; MCP request id; gateway ledger entry; mod receipt; pre-action generation | pending |
| Settlement | `run_started` witness; first non-null `RunState` witness; settled successor observation; post-action generation; correlation to the admission operation | pending |
| Duplicate / lost-reply / stale / uncertain attempts | one entry per attempt: request fingerprint, generation, host response (`exact replay`, `idempotency conflict`, `Unknown` with late-readback outcome, or refusal before mutation), and proof that no second run started | pending |
| Cleanup | campaign PIDs stopped; lease released; original install/profile baseline re-verified against `bfde2573…`; disposable root removed after review; evidence archive digest; UTC end; elapsed time within the ten-minute bound | pending |

## External gate

Quoted from the 2026-09-16 comment on #79: "The Windows/Steam account owner must establish a
normal logged-in session with Steam running. After that, rerun the existing preflight and execute
the prepared seeded campaign, capturing native admission, action, settlement, and cleanup
evidence."

This is external to every repository in the organization. No repository change can discharge it.
When it clears, the order is: preflight and classifier, re-pin or rebuild (section above), bounded
campaign, receipts, independent review of receipts and negative cases, then this record is updated
row by row from `unverified-native` to the observed result.

## Related records

- [ADR 0031: native standard seeded-run adapter](../decisions/0031-native-standard-seeded-run-adapter.md)
- [ADR 0041: save-profile selection and disposable boundary](../decisions/0041-save-profile-selection-and-disposable-boundary.md)
- [Runtime-map-v1 host/build evidence (host pin)](runtime-map-v1-host-build-20260907.md)
- [Compatibility matrix](../COMPATIBILITY.md)
- [Seeded-run client](../../tools/seeded-run-client/README.md)
