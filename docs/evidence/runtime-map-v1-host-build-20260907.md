# Runtime-map-v1 host/build evidence

Date: 2026-09-07
Status: `confirmed` for source, exact-host compilation, managed probe, package, and local gates; live map extraction and settled map navigation remain `unverified`.
Scope: the isolated `sts2-game-mod` map-visibility worktree and an operator-supplied exact host. No
game launch, provider call, addon installation, save mutation, or active Steam profile change was
performed for this record.

## Supported host tuple

| Field | Value |
| --- | --- |
| Game | Slay the Spire 2 v0.107.1 |
| Release label | `59260271` |
| Platform | Windows x86-64 |
| `sts2.dll` SHA-256 | `a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52` |
| `GodotSharp.dll` SHA-256 | `0e4897ecdfb31456a97c7d8028dfb8d7dbdc632e2f73fc9b438d7b266a139289` |
| Managed host | .NET 9; Windows SDK 10.0.204 and Linux SDK 9.0.317 both compiled the loader |
| Native target | Rust 1.97.1, `x86_64-pc-windows-gnu` |

The proprietary assemblies remain outside the repository and are used only as compatibility
inputs. The supported tuple is enforced by `live-combat-session.sh` before the host guardian,
gateway, harness, or provider is started.

## Commands and results

The exact host build used the external data directory containing the two assemblies:

~~~text
/home/timot/.dotnet/dotnet restore experiments/managed-rust-interop/game-loader/GameLoaderProbe.csproj \
  -p:STS2GameDataDir="<operator-supplied-host-data>"
/home/timot/.dotnet/dotnet build experiments/managed-rust-interop/game-loader/GameLoaderProbe.csproj \
  --configuration Release -p:STS2GameDataDir="<operator-supplied-host-data>" --no-restore
~~~

Result: exit `0`, zero warnings, zero errors. The same project was compiled with Windows .NET SDK
`10.0.204` against the same assembly tuple with exit `0`, zero warnings, and zero errors.

The focused managed probe passed all checks, including bounded identity-registry exhaustion/reset,
stable graph-ID ordering and rewiring, `Ancient` normalization, and final-generation observation
fencing:

~~~text
/home/timot/.dotnet/dotnet run \
  --project experiments/managed-rust-interop/map-tests/RuntimeMapV1Probe.csproj \
  --configuration Release
~~~

Result: exit `0`; `RuntimeMapV1Probe: PASS`. The source-only managed interop build and workshop
probe also passed with zero warnings/errors.

An external package build produced these three expected files. The staging directory is disposable
and outside the repository:

| File | SHA-256 |
| --- | --- |
| `AIAscensionSTS2GameMod.dll` | `5cf1500990199288ea46ca47bc8069fa74b6d95a54a8036c1b938dcd322d8bed` |
| `AIAscensionSTS2GameModNative.dll` | `5082cb71025bba2a2ad07949fd1f83630924f0a76580ff4b9b11382e367519cb` |
| `AIAscensionSTS2GameMod.json` | `559e177f0b6e5d82fc44f6b086b1e728353b2f6e437f5e8fae989083d85659984` |

## Local gates

All commands used a worktree-specific `CARGO_TARGET_DIR=/tmp/sts2-map-visibility-20260907-rust-target`:

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | exit `0` |
| `cargo run --locked --offline --package repo-policy -- --strict` | exit `0`; 304 files, 0 warnings, 0 errors |
| `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` | exit `0` |
| `cargo test --locked --offline --workspace --all-targets --all-features` | exit `0`; all workspace tests passed, including 16 native tests |
| `cargo build --locked --offline --workspace --all-targets --all-features` | exit `0` |
| `git diff --check` | exit `0` |

These gates establish source/build/component behavior only. They do not establish loader discovery,
live topology completeness, gateway/MCP/provider delivery, image comprehension, or gameplay
settlement.

## Baseline and bounded-live procedure

Before an authorized run, copy the supported installation to a disposable host directory and keep
the original host, active Steam `mods/`, and valued saves untouched. Hash the copied
`sts2.dll`, `GodotSharp.dll`, executable, `override.cfg`, and the three accepted addon files.
Prepare a separate disposable Godot user directory, retaining its pre-run backup when it already
contains the accepted-mod consent/profile fixture. The launcher repeats the host/addon checks,
writes `baseline.sha256` and `addon.sha256` under the external run directory, and verifies the
baseline after its owned host exits.

Use `live-combat-session.sh --campaign-map` only with explicit gateway, MCP, harness, approved
OpenAI Astra provider, and pinned map-renderer binaries. The launcher verifies the renderer SHA-256,
sets `STS2_LIVE_COMBAT=1`, `STS2_LIVE_CAMPAIGN=1`, `STS2_MAP_MODE=graph-image`, and the standard
campaign/map bound in the Windows child, then admits one host-legal `start_run` and one host-legal
`select_map_node`. The episode has two model decision steps, one read-only recovery attempt, and a
90-second provider deadline. Trace acceptance requires those two settled decisions, a verified PNG
attachment, and no other action kind. This record does not claim that the live command was executed.
