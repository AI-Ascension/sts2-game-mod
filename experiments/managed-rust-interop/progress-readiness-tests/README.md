# Profile progress readiness observer probe

Run the source-linked managed probe with the repository-pinned .NET 9 SDK and a task-owned output
directory outside the checkout:

~~~text
dotnet run --project experiments/managed-rust-interop/progress-readiness-tests/ProgressReadinessProbe.csproj --configuration Release -p:ManagedBuildRoot=<task-owned-external-directory>
~~~

The probe links the production reducer, lifecycle callback seam, patch-registration coordinator,
owned-method verifier, and exact host-pin comparison. Synthetic cases cover every read-result
status combination, the fresh missing-file path, stale and replaced identities, nested-load refusal
and recovery, thread/runtime fences, prefix invalidation, exact owner/method verification, optional
Harmony-load failure, exception preservation, and the pinned host digest. It does not compile the
Harmony adapter, load proprietary assemblies, establish loader order, touch a profile, invoke a host
callback, or prove native readiness.

The add-on compatibility build is separate and accepts an operator-supplied exact host data folder:

~~~text
dotnet restore experiments/managed-rust-interop/game-loader/GameLoaderProbe.csproj --property:STS2GameDataDir=<operator-supplied-host-data-directory> -p:ManagedBuildRoot=<task-owned-external-directory>
dotnet build experiments/managed-rust-interop/game-loader/GameLoaderProbe.csproj --configuration Release --property:STS2GameDataDir=<operator-supplied-host-data-directory> -p:ManagedBuildRoot=<task-owned-external-directory> --no-restore
~~~

For this source allocation, the authorized host identity is STS2 v0.107.1 with `sts2.dll` SHA-256
`a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52`, and host-provided 0Harmony
2.4.2.0 with SHA-256 `ef1898322c9f5c86dc1b0758b272a9c440823b4a41ca9a0b82a3aa6b3d206387`. Both are
validated from their external locations and are excluded from the package. A successful build would
only establish source compilation against those public metadata symbols; native hook execution,
loader ordering, existing-profile access, and game behavior remain separate evidence.
