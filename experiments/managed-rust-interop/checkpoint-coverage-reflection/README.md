# Exact-build checkpoint coverage metadata probe

This opt-in probe reads the metadata tables of the operator-supplied exact host assembly through
`PEReader`/`MetadataReader` and resolves every ADR 0037 coverage family and every ADR 0040 RNG
audit row to concrete host members: type, member name, member kind, declared type, and visibility.
It does **not** load the assembly, resolve its dependencies, instantiate a type, run a static
constructor, invoke a method, or read a value. Its output records names and hashes only; no host
byte, IL, string constant, or install path appears in it.

Every row matches ordinally on the full type name, member name, and member kind. A row that does
not match is reported `unresolved` and its family fails closed; nothing is relaxed, substituted, or
omitted. A family whose rows all resolve is `metadata-observed`; a row that records the absence of
a member (for example the ADR 0037 `keys` item) is `metadata-absent`. Every family's serialization,
ordering, restore, and unknown-value statements are policy derived from the observed shapes and
stay `runtime-unverified`. Nothing this probe emits is native-verified.

Review warning: keep this tool metadata-only. Do not replace it with `Assembly.Load*`, `GetTypes`,
`MetadataLoadContext`, `AssemblyLoadContext`, or `FieldInfo.GetValue`; those can resolve or execute
code from the reserved host assembly. The companion tests guard those APIs in the probe sources.

The probe fails closed at build time without `STS2GameDataDir`; it is not part of hosted CI, and
the host assemblies stay outside the repository. With read-only access to the exact pinned host
data directory, record the inventory with an external build root:

```text
DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1 dotnet run \
  --project experiments/managed-rust-interop/checkpoint-coverage-reflection/CheckpointCoverageReflection.csproj \
  -c Release -p:STS2GameDataDir=/path/to/data_sts2_windows_x86_64 -p:ManagedBuildRoot=/tmp/sts2-game-mod-build/managed \
  -- --output-dir /tmp/checkpoint-inventory --output-stem checkpoint-coverage-inventory-YYYYMMDD --recorded-on YYYY-MM-DD
```

It writes `<stem>.json` (all families), `<stem>.md` (ADR 0037 families), and `<stem>-rng.md`
(ADR 0040 rows), prints the `sts2.dll` SHA-256 and whether it matches the pinned v0.107.1 build,
and exits non-zero when any family is unresolved. Without `--output-dir` the JSON goes to stdout.

The opt-in regressions run against the same pinned assembly:

```text
DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1 dotnet run \
  --project experiments/managed-rust-interop/checkpoint-coverage-reflection-tests/CheckpointCoverageReflectionTests.csproj \
  -c Release -p:STS2GameDataDir=/path/to/data_sts2_windows_x86_64 -p:ManagedBuildRoot=/tmp/sts2-game-mod-build/managed
```

`EveryInventoryFamilyResolvesToPinnedMember` requires the supplied hashes to equal the pin and every
family to be `metadata-observed`; `UnknownFamilyFailsClosed` proves that a missing type, a missing
member, a wrong kind, a case or prefix mismatch, an empty family, and a false absence claim all stay
`unresolved` while a known control row still resolves; the remaining checks prove that no host
directory reaches the reports and that the probe sources use no loading or executing reflection API.

The recorded result for the pinned build is
[`docs/evidence/checkpoint-coverage-inventory-20260917.md`](../../../docs/evidence/checkpoint-coverage-inventory-20260917.md)
with its JSON and RNG companions; ADR 0037 and ADR 0040 carry the dated amendments that cite it.
