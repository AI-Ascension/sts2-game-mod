# `runtime-v1` live host evidence

- Date: 2026-09-02
- Evidence level: `confirmed` focused runtime for one exact disposable host profile
- Scope: managed loader, Rust ABI, loopback HTTP listener, managed main-thread queue, host-visible
  overlay witness, and stale-generation handling
- Canonical schema digest: `a76086d7a68668fd4cff53999369d2b450b0d6623827393882f458f2aa1f93eb`
- Source state: target HEAD `97f3a2068452d2c1616c531a7dfad51fbd484cac` plus the uncommitted
  `runtime-v1` changes in the isolated worktree used for this test

## Host matrix

| Field | Observed value |
| --- | --- |
| Game | Slay the Spire 2 `v0.107.1` |
| Release commit | `59260271` |
| Host assembly | `sts2.dll`, SHA-256 `a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52` |
| Operating system | Windows `10.0.26200`, x86-64 |
| Engine | Godot `4.5.1-m.12` custom build |
| Game framework | .NET `9.0.7` runtime |
| Rust build | `rustc 1.97.1`, target `x86_64-pc-windows-gnu` |
| Managed build | .NET SDK `10.0.204` |

The proprietary host assembly was used only as a local build/reference input. It is not stored or
distributed by this repository.

## Runtime package

The staged package contained exactly the managed DLL, unique native companion, and manifest:

| File | SHA-256 |
| --- | --- |
| `AIAscensionSTS2Poc.dll` | `dd37873ca45a8a69058137b661a4c9dc0d7a66cafe6806b90423db12a35e9d46` |
| `ai_ascension_sts2_poc.dll` | `6d518a3f018c6f2d6553cd765b147eb3a3017457d4e55c665421794b84ba4444` |
| `AIAscensionSTS2Poc.json` | `a75717d4de14cf87d48b54b15fe45a3c58c231ef7395781b2e780d0a5e8c2985` |

## Setup and sequence

The package was built from the target's `package-runtime-addon.sh` script and installed over only
the three addon-owned files in an authorized local game install. The accepted profile baseline was
copied to a disposable profile directory before launch. The token and port were set inside the
Windows PowerShell process because WSL environment variables are not inherited by a Windows
executable. The game used normal rendering with dummy audio, compatibility rendering, and a bounded
`--quit-after` value.

The game log recorded, in order:

~~~text
Found mod manifest file ...\\mods\\AIAscensionSTS2Poc.json
Loading assembly DLL ...\\mods\\AIAscensionSTS2Poc.dll
Calling initializer method of type AiAscension.Sts2GameMod.Runtime.ModEntry
[AI-ASCENSION STS2 POC] loaded managed entry point and Rust ABI; ABI=1; 19+23=42
[AI-ASCENSION STS2 POC] authenticated runtime HTTP listener started on 127.0.0.1:15526
Finished mod initialization for 'AI-Ascension STS2 POC' (AIAscensionSTS2Poc).
--- RUNNING MODDED! --- Loaded 1 mods (1 total)
~~~

The full coordinator path then ran against the live process:

~~~text
harness -> runtime-v1-mcp -> sts2-gateway-runtime -> game-mod -> Godot host
~~~

The coordinator allocated the configured lease, initialized MCP, verified the `runtime-v1-mcp`
catalog, read generation 0, submitted `show_runtime_probe` at generation 0, repeated the stale
generation, read fresh state, and released the lease. The sanitized trace was:

~~~json
{"accepted_effect":{"generation":1,"kind":"status_overlay_visible"},"after_generation":1,"before_generation":0,"instance_id":"instance-1","observation":{"action_count":1,"host_ready":true,"overlay_visible":true,"screen":"host"},"protocol":"runtime-v1","session_id":"session-1","stale_rejection":"sts2.game-mod/stale_generation"}
~~~

The accepted response was produced only after the managed action observed the `CanvasLayer` status
overlay. A contemporaneous operator window capture visibly showed `AI-ASCENSION STS2` and
`LIVE RUNTIME | ABI 1 | effect witnessed`. The capture is not retained in the repository.

## Cleanup and limits

The controlled game process and gateway were stopped after the trace. The original profile was
restored, the three prior addon file hashes were restored, the disposable profile copies were moved
aside rather than deleted, and no STS2 process or runtime listener remained. No credential, save,
host assembly, or runtime log was added to the repository.

This confirms the focused runtime path for the exact recorded host and package. It does not confirm
gameplay-rule mutation, action legality, process supervision or restart, multi-instance isolation,
another host build, another platform, release distribution, provider execution, or full conformance.
