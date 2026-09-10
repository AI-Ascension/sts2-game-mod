# ADR 0034: Rust release provenance tooling boundary

## Status

Accepted for the release tooling implementation. Publication, installation, Workshop upload,
and host or game compatibility remain separate evidence states.

## Context

The release lane needs one locked implementation for runtime receipts, the paired build-artifact
manifest, platform-specific Workshop staging, and source-distribution path policy validation.
Repository policy prohibits Python production source, while the historical shell entry points are
used by operators and existing checks.

## Decision

The `sts2-release-tool` workspace member owns the four release commands:

- `build-runtime-receipt` records one production or explicitly fixture-scoped runtime payload;
- `build-artifact-manifest` binds the Windows and Linux production receipts to one Git commit,
  source tree, loader manifest, version pair, and payload inventory; and
- `package-platform-item` validates that binding for one platform before staging the package and
  its adjacent VDF.
- `validate-source-policy` validates the checked-in source-distribution policy against the resolved
  Git path inventory and writes the allow, exclusion, and required path lists used by the source
  bundle builder.

The shell entry points remain compatibility wrappers. They resolve the repository root, export it
as `STS2_RELEASE_REPO_ROOT`, and invoke Cargo with the repository manifest path so a caller's
working directory does not select a different workspace. No release command imports or executes
Python production code. `--legacy-unbound` is retained only for synthetic or historical fixture
callers.

Production provenance is established from the current clean checkout, the exact selected Git
loader bytes, both receipt roots, host-reference size and SHA-256 records, toolchain identities,
and the staged payload digests. Receipt metadata must preserve the requested platform string
exactly; a missing, non-string, or different platform value fails closed. Publication uses private
staging, exclusive no-replace moves, and cleanup guarded by the identity of the process-owned
staging path.

## Evidence and limits

Locked Rust compilation, policy, fixture tests, and package checks establish source and contract
behavior. They do not establish Steam configuration, Workshop publication, installation into a
host, loader discovery, or game compatibility. Those require their own authorized disposable
environment and exact host evidence.
