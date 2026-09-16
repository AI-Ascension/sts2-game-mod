# Isolated ModelDb registry probe

This project builds a separate diagnostic addon; it is not referenced by the game-mod loader
project, native library, or HTTP adapter. Its initializer is inert unless
`STS2_MODELDB_REGISTRY_PROBE=1` is set for the selected game process.

Build it against the exact host references:

```text
STS2GameDataDir=/path/to/data_sts2_windows_x86_64 \
  /path/to/dotnet build experiments/managed-rust-interop/content-registry-probe/ContentRegistryProbe.csproj \
  --configuration Release
```

Before installing the output as a separate diagnostic mod in an authorized disposable game
directory, verify that its `sts2.dll` hash is
`a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52`. The probe also hashes the
loaded `sts2.dll` itself before reading `_contentById`. It requires
`ReleaseInfo.Version == "v0.107.1"`; that value is an expected identity and has not been observed
in a live game during this work.

The probe captures the owner thread from the first `SceneTree.ProcessFrame` callback. It waits for
`ModManager.State == Initialized`, a nonempty ModelDb registry, and two identical bounded captures
on consecutive frames before printing one bounded JSON report. It refuses field/type drift,
wrong builds, cancellation, timeout, or data above the fixed bounds. The report contains only
ModelDb IDs and runtime/category type names. It does not call constructors or definition getters,
write files, enter a run, or expose a route.

Run the synthetic guard and bound tests without host files:

```text
/path/to/dotnet run --project experiments/managed-rust-interop/content-registry-probe-tests/ContentRegistryProbeTests.csproj \
  --configuration Release
```

Successful builds and synthetic tests do not prove a live registry read. The exact-host runtime
probe remains pending the unchanged disposable Steam session and a separate owner review of its
bounded report. The resulting IDs and grouped observations are not independent per-family totals
and cannot be used as a complete manifest.
