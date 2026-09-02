# Runtime addon load-smoke evidence

Date: 2026-09-02  
Status: `confirmed` for load-smoke only; gameplay and HTTP behavior remain `unverified`.  
Source revision: `3e8ccabb6bc138ea962aa399b0d61de1f1c13587`

## Host matrix

| Field | Observed value |
| --- | --- |
| Game | Slay the Spire 2 `v0.107.1` |
| Release commit | `59260271` |
| Host assembly | `sts2.dll`, SHA-256 `a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52` |
| Operating system | Windows `10.0.26200`, x86-64 |
| Engine | Godot `4.5.1-m.12` custom build |
| Game framework | .NET `9.0.7` runtime, from the installed game runtime configuration |
| Rust build | `rustc 1.97.1`, `cargo 1.97.1`, target `x86_64-pc-windows-gnu` |
| Managed build | .NET SDK `10.0.204` |

The proprietary host assembly was used only as a local build/reference input. It is not stored or
distributed by this repository.

## Package

The package contains the managed loader, its unique native companion, and the manifest:

| File | SHA-256 |
| --- | --- |
| `AIAscensionSTS2Poc.dll` | `14da817e7a031c4968becd63e6dedbeb607ca9764d83b1d0c58ba6936d07f862` |
| `ai_ascension_sts2_poc.dll` | `30650a13b1748f0a27312f390013394de2f379adb880f2ab7acee3e1dbb8d8cd` |
| `AIAscensionSTS2Poc.json` | `a75717d4de14cf87d48b54b15fe45a3c58c231ef7395781b2e780d0a5e8c2985` |

## Reproduction

From the target worktree, the package was built and staged with:

~~~text
bash experiments/managed-rust-interop/package-runtime-addon.sh \
  "/mnt/c/Program Files (x86)/Steam/steamapps/common/Slay the Spire 2/data_sts2_windows_x86_64" \
  /tmp/sts2-runtime-addon-20260902-v5
~~~

The three staged files were copied to the installed game's `mods/` directory. The final launch used
the installed Windows executable with headless rendering and a bounded frame count:

~~~text
timeout 45s SlayTheSpire2.exe --headless --audio-driver Dummy \
  --rendering-method gl_compatibility --quit-after 180 \
  --log-file C:\\Users\\<operator>\\AppData\\Local\\Temp\\sts2-runtime-addon-20260902-v2.log
~~~

The process exited `0`.

## Observed result

The game log recorded all of the following in order:

~~~text
Found mod manifest file ...\\mods\\AIAscensionSTS2Poc.json
Loading assembly DLL ...\\mods\\AIAscensionSTS2Poc.dll
Calling initializer method of type AiAscension.Sts2GameMod.Runtime.ModEntry
[AI-ASCENSION STS2 POC] loaded managed entry point and Rust ABI; ABI=1; 19+23=42
Finished mod initialization for 'AI-Ascension STS2 POC' (AIAscensionSTS2Poc).
--- RUNNING MODDED! --- Loaded 3 mods (3 total)
~~~

No addon initialization failure or managed exception was observed. The headless run did emit
Godot resource-leak diagnostics during shutdown; those are not attributed to the addon and do not
establish gameplay behavior.

## Side effects and limits

The game used the existing Steam-backed modded profile during startup and reported normal writes to
its profile/save history. This was therefore a real host load-smoke, not a disposable profile test;
no in-game action was requested. The final package uses the unique native filename
`ai_ascension_sts2_poc.dll` and leaves the existing unrelated mods in place.

This evidence confirms manifest discovery, managed initializer invocation, adjacent native loading,
ABI version validation, and the bounded native smoke call. It does not confirm host object access,
main-thread dispatch, HTTP serving, game-state mutation, action legality, effect settlement, or
compatibility with another game build or platform.
