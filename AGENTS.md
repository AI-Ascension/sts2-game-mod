# Agent operating contract

## Target

sts2-game-mod owns the game-facing boundary of the STS2 system: the managed loader entry point,
host integration, main-thread dispatch, authoritative local HTTP adapter, and the narrow Rust/native
seam. The mod captain owns this target tree only.

Wave 1 established the repository foundation. Wave 2 initializes only the target-owned host,
HTTP-adapter, and composition seams with deterministic fakes; it does not add game behavior,
public routes, or a real loader. The existing managed-rust-interop experiment is preserved as
source-only evidence and is not product behavior. Do not add further crates or placeholders merely
to satisfy a directory plan.

## Boundary rules

- The game host remains authoritative for state, legal mutations, and thread affinity.
- Managed code is limited to loader metadata, host callbacks, native-library lifetime, and ABI
  translation.
- Host objects must become owned values before they cross a thread or process boundary.
- The HTTP adapter owns request decoding, bounded responses, route mapping, and sanitized errors.
- The Rust/native seam uses fixed-width types, explicit ownership, versioned ABI data, and reviewed
  unsafe blocks.
- The core target is host-, transport-, process-, and filesystem-independent.
- The gateway owns lifecycle, routing, leases, isolation, and control-plane policy.
- The MCP server is a thin protocol adapter and never links to host code.
- The harness owns coordination, explicit instance context, experiments, and artifacts.
- The accepted sixth target, sts2-protocol, owns only genuinely shared language- and
  transport-neutral contracts after its own decision and conformance work.

Do not move a concern across these boundaries for convenience. An ownership or dependency change
requires a decision record under docs/decisions.

## Safety and provenance

Never add proprietary host assemblies, game binaries, game assets, saves, credentials, private
profiles, personal paths, generated build output, or copied implementation source. The local host
installation may be used only by an explicitly authorized compatibility test outside this tree.
Do not use the interop experiment as permission to install anything into a valued profile.

Claims must be labeled as confirmed, source-derived, proposed, inferred, or unverified. A compile
or generic ABI result is not proof that the game discovers, loads, or safely runs the mod.

Preserve unrelated files. Do not initialize Git, stage broadly, commit, push, deploy, publish,
install, launch a game, contact a provider, or modify a sibling target from this directory.

## Editing and validation

Read the applicable docs before changing a boundary or public contract. Use apply_patch for
repository edits. Keep handwritten files within policy budgets and split files by responsibility.
Do not hide failures with unconditional success, blanket retries, or ignored results.

## Build process

Run commands from the `sts2-game-mod` target root. The repository pins Rust 1.97.1 in
`rust-toolchain.toml`; the managed source-only CI job uses .NET SDK 9.0.317. Build output belongs in
Cargo's `target/` or the .NET projects' `bin/` and `obj/` directories and must not be committed.

### Rust workspace

Use the locked dependency graph. Offline mode is preferred after dependencies and the pinned
toolchain are available locally:

~~~text
cargo build --locked --offline --workspace --all-targets --all-features
cargo run --locked --offline --package repo-policy -- --strict
cargo fmt --all --check
cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings
cargo test --locked --offline --workspace --all-targets --all-features
~~~

The policy command is the repository gate, not a replacement for compilation or tests. CI mirrors
the format, Clippy, and workspace-test commands (without `--offline`); a local offline failure is
an environment/dependency-cache limitation and must be reported as such.

### Managed source-only build

This build has no proprietary host dependency and is the managed CI boundary:

~~~text
dotnet build experiments/managed-rust-interop/managed/ManagedInteropSpike.csproj --configuration Release
~~~

Use the pinned .NET 9 SDK. This proves the source-only managed probe compiles; it does not prove
that STS2 discovers the loader, that a host assembly is compatible, or that the game runs it.

### Host-dependent Windows addon build

The loader project references an operator-supplied exact host installation at build time. Keep
`sts2.dll` and `GodotSharp.dll` outside the repository and use only an authorized disposable host
when this evidence is required. The Windows GNU Rust target must be installed before packaging:

~~~text
rustup target add x86_64-pc-windows-gnu
~~~

The canonical packaging command accepts either a WSL path or a Windows path for the directory
containing those two assemblies and stages exactly three addon files in the output directory:

~~~text
bash experiments/managed-rust-interop/package-runtime-addon.sh \
  "/path/to/Slay the Spire 2/data_sts2_windows_x86_64" \
  /tmp/sts2-runtime-addon
~~~

The script builds the native `x86_64-pc-windows-gnu` release library, restores and builds
`GameLoaderProbe.csproj` for `net9.0` with `STS2GameDataDir`, verifies the expected artifacts, and
prints their SHA-256 digests. Do not copy the host assemblies, game files, saves, or staged output
back into the repository.

For a complete authorized WSL install/relaunch cycle, use the target-owned wrapper with an explicit
game directory:

~~~text
bash experiments/managed-rust-interop/dev-cycle.sh \
  --game-dir "/path/to/Slay the Spire 2"
~~~

After a successful build it stops only `SlayTheSpire2.exe`, backs up replaced addon files under the
ignored `.sts2-dev/backups/` directory, copies and compares the three staged files in `mods/`, then
relaunches the same installation. Use `--dry-run` to inspect the cycle, `--no-launch` for an
install-only operation, or `--no-kill` only when skipping the stop is known to be safe because the
game is stopped or the installation is not locked. A game launch, load smoke, or runtime probe is separate host evidence and
must be recorded as confirmed or unverified; it is never implied by a successful build.

### CI and host-boundary rule

Pull-request CI runs the Rust gates and the managed source-only build. It deliberately does not run
the host-loader package build because that requires proprietary `sts2.dll` and `GodotSharp.dll`.
When that prerequisite is unavailable, skip only the host-dependent command and record the exact
reason; never replace it with an unconditional success or claim runtime compatibility.

The required local entrypoint is:

~~~text
cargo run --locked --offline --package repo-policy -- --strict
~~~

Run the applicable Rust gates as well:

~~~text
cargo fmt --all --check
cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings
cargo test --locked --offline --workspace --all-targets --all-features
~~~

The managed loader project needs an operator-supplied exact host assembly and is not a general
local prerequisite. The source-only managed probe can be built without that assembly. Record every
skipped host/runtime check and never convert it into a pass.

## Documentation

Update architecture, compatibility, security, testing, licensing, workflow, and release documents
when their subject changes. Public routes, ABI symbols, host callbacks, queue behavior, and package
contents need a project-owned requirement and a deterministic test before implementation.

The final handoff must state the target, changed paths, exact command results, unverified limits,
and the next owner action. Do not claim a merge, release, deployment, or runtime compatibility
without its separate evidence.
