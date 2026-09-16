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
`ModManager.State == Initialized`, a nonempty ModelDb registry, and two identical bounded owned
snapshots on consecutive frames before printing one bounded JSON report. The report labels this
as an observed partial-registry snapshot and stability interval; it does not prove that
`ModelDb.Init` or all content loading has completed, or that the game is at the main menu. It
refuses field/type drift, wrong builds, cancellation, timeout, or data above the fixed bounds. The
report contains only ModelDb IDs, runtime/category type names, and this explicit readiness basis.
It does not call `ModelDb.Init`, construct definitions, or call semantic getters, write files, enter
a run, or expose a route.

The exact pinned host is marked `BeforeFieldInit`; its type initializer only creates and assigns
the empty registry dictionary, while `ModelDb.Init` separately constructs model definitions.
`FieldInfo.GetValue` may run that empty-dictionary initializer, which the bounded probe permits
after exact binary and field-shape checks. The pinned `GetCategoryType` call-target inventory only
uses runtime type hierarchy accessors and comparisons. `ModManager.Initialized` is a loader-state
observation, not proof that `ModelDb.Init` or all content loading completed. Never interpret a
successful partial snapshot as a complete catalog.

Run the synthetic guard and bound tests without host files:

```text
/path/to/dotnet run --project experiments/managed-rust-interop/content-registry-probe-tests/ContentRegistryProbeTests.csproj \
  --configuration Release
```

Successful builds and synthetic tests do not prove a live registry read. The entrypoint remains
fail-closed pending an independent owner witness and separate owner review; the unchanged
disposable Steam session is also unavailable. Any future captured IDs and grouped observations
would not be independent per-family totals and could not be used as a complete manifest.
