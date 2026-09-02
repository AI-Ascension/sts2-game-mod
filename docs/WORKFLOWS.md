# Development and automation workflows

## Lifecycle

Design decision -> focused change -> local evidence -> pull request -> required checks and review
-> explicitly authorized merge -> release candidate -> authorized publication -> post-release
verification. A green check is not a merge, release, install, deployment, or compatibility claim.

## Foundation workflows

- policy.yml checks the target policy tool and strict policy from pull requests and main pushes.
- ci.yml runs Rust format, Clippy, tests, and a source-only managed probe.
- The managed host-loader build is intentionally not a CI lane because it needs an operator-owned
  proprietary sts2.dll.
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
