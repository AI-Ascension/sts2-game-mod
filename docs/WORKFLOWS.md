# Development and automation workflows

For an already prepared disposable host, `live-combat-session.sh --help` under
`experiments/managed-rust-interop` describes the repeatable visible model/replay launcher.
It requires explicit executable paths and external artifact storage; it does not install game files.
See [LIVE_COMBAT_DEMO.md](LIVE_COMBAT_DEMO.md) before using that exact-host experiment.

## Lifecycle

Design decision -> focused change -> local evidence -> pull request -> required checks and review
-> explicitly authorized merge -> release candidate -> authorized publication -> post-release
verification. A green check is not a merge, release, install, deployment, or compatibility claim.

## Foundation workflows

- policy.yml checks the target policy tool and strict policy from pull requests and main pushes.
- ci.yml runs Rust format, Clippy, tests, the source-only native interop probe, and the synthetic
  ephemeral-session launcher checks.
- The managed runtime-addon build is intentionally not a CI lane because it needs an
  operator-owned proprietary `sts2.dll` and `GodotSharp.dll`.
- The authorized host load-smoke is a manual lane using
  `experiments/managed-rust-interop/package-runtime-addon.sh`; its exact game input, artifact
  hashes, launch command, log marker, and cleanup belong in a dated evidence report.
- The authorized authenticated session lane uses
  `experiments/managed-rust-interop/session-launcher.sh`; it generates in-memory per-launch
  credentials, crosses WSL to Windows over stdin, starts only explicit provider binaries, and
  cleans up only recorded child processes. Its synthetic checks are local; live readiness and
  cleanup remain a separate disposable-host claim.
- Future host, security, conformance, and release workflows must be added with real commands and
  evidence; no empty success job is allowed.

## Trust and permissions

Workflows use pull_request for untrusted changes, top-level contents read permission, explicit
timeouts, immutable third-party action commits, and no secrets. pull_request_target is prohibited.
Pull-request code must never run with write-capable tokens, private host assemblies, saves, or
trusted self-hosted networks.

Scheduled or manually dispatched work, if added, must record the exact source revision. A user
supplied ref must not select arbitrary code for a privileged job.

## Commands and artifacts

Reusable behavior belongs in checked-in tools and must be callable locally. Commands must expose
failures and avoid unconditional success, blanket retries, and ignored results. Keep workflows
under the policy's 200 nonblank-line hard limit.

Cache only reproducible dependencies and build intermediates. Artifacts have deterministic names,
bounded retention and size, source revision metadata, sanitization, and no secrets, host files,
saves, credentials, or release output from an unapproved source.

## Pull requests and releases

Pull requests describe ownership, architecture, public-contract effects, compatibility, security,
exact checks, and unverified limits. Required checks must report on every applicable event; path
filters must not strand a required check in pending state.

Release automation follows [RELEASING.md](../RELEASING.md). It validates exact tags, package
allowlists, checksums, provenance, compatibility evidence, and approval before publication.
Branch protection and protected environments remain external controls.

## Workflow review

Any change to events, permissions, actions, refs, secrets, environments, runners, caches, or
artifacts is an authority change and must be called out in review. Validate workflow syntax and
security with the repository policy checker and dedicated tools when available; a missing tool is
reported as unverified.

## Workshop workflow

Workshop content is prepared only from an explicit operator command after the normal build and
compatibility gates. package-item.sh writes its VDF beside the content directory; the VDF is not
part of the Workshop payload. Pull-request and ordinary push workflows never call SteamCMD,
ISteamUGC, or any upload endpoint. An authorized publisher must supply the consumer App ID,
published file ID, preview image, and credentials outside the repository, then verify the exact
staged bytes and Steam result separately.
