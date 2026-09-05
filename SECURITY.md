# Security Policy

## Scope

The optional live-combat session uses separate ephemeral gateway and game credentials.
Its host receives the game credential through stdin. Logs and trajectories belong in external
operator storage. A disposable game copy alone does not isolate Steam cache writes before
the mod loads; see `docs/LIVE_COMBAT_DEMO.md` for the observed incident and local-only backend.

Report vulnerabilities in the managed loader, native ABI, host boundary, local HTTP adapter, policy
tooling, or packaging documentation. Do not include game binaries, saves, credentials, personal
paths, or unredacted logs in a report.

## Reporting

Use a private maintainer security channel when one is configured for the hosted repository. Until
then, do not disclose suspected vulnerabilities publicly; ask the repository owner for a private
reporting route.

Include a concise impact statement, affected revision or file, reproduction using synthetic data,
and any mitigation already applied. Maintainers will acknowledge receipt, investigate, coordinate
disclosure, and publish a fix only after affected users can update.

## Runtime session handling

The persistent guardian's bounded handoff, kill-on-close Job ownership, platform requirements and
failure evidence are specified in [LAUNCHER_PROCESS_HANDOFF.md](docs/LAUNCHER_PROCESS_HANDOFF.md).

The disposable session launcher generates a fresh runtime/mod credential and a distinct gateway
credential from the operating system CSPRNG for every launch. Credentials are held only in process
memory, passed to child environments or the Windows bridge stdin, and never accepted as launcher
arguments or written to files, logs, screenshots, Steam options, URLs, or CI artifacts. The owned
launcher and bridge request `--headless --audio-driver Dummy` and contain no system-input API
calls. These flags are not a sandbox: whether a particular host honors them without creating a
window or capturing the cursor requires exact-host validation. The launcher refuses to adopt
an already-running game, defaults the game listener to loopback, bounds readiness, and terminates
only recorded child process groups and the Windows process whose PID, creation time, and executable
path match the launch receipt. The bridge's child inherits only NUL standard handles, not the
caller's credential or captured-output pipes. Independently launched hosts
and unrelated desktop processes are outside this guarantee.

Non-dry-run owned launch paths fail closed unless the operator supplies a complete non-secret
`LIVE_AUTHORIZATION` record. It names the exact host/install, disposable profile, permitted process
and profile actions, loopback listener/network scope, cleanup owner, restore point, future deadline,
publication authority, and provider-call status. The record is checked before host inspection,
installation, profile access, listener setup, or child-process creation, then removed from child
environments. The launcher has no provider-call path; it rejects any value other than
`STS2_LIVE_AUTHORIZATION_PROVIDER_CALLS=prohibited`.

The record is an operator attestation, not a credential or an automatic verification of the
selected game profile, restore point, or provider executable. The operator must supply trusted
binaries and verify the disposable profile externally; arbitrary supplied binaries are not
sandboxed. Authorized build commands are deadline-bounded, admission is rechecked after builds,
and session supervision begins cleanup on expiry, including in keep-alive mode. Cleanup itself
can take additional bounded time and is reported as failed if it cannot be confirmed.

The project does not promise a response time or a bounty. This policy is not permission to access
another person's game profile, host installation, network, or data.

The native HTTP listener bounds each accepted connection's socket I/O with one absolute
10-second deadline and checks listener shutdown between short socket waits. This is a transport
resource bound, not a host callback cancellation mechanism or a guarantee of real-time shutdown.
Only loopback synthetic connections are used in the regression suite.

## Workshop package boundary

The Runtime-v2 candidate enforces an instance/caller/session/lease fence. An
unresolved queued mutation excludes further gameplay mutations through this profile; restarting
the listener does not clear uncertain receipts. Independent host completion and authoritative lease
handoff remain unimplemented and must be established before autonomous gameplay use.

Workshop packages are first-party executable content, not an extension point for arbitrary
third-party code. The checked-in package tool stages an exact allowlist, records sizes and SHA-256
digests, and emits a manifest and checksum inventory. The managed loader rejects unexpected files,
reparse points, unsafe paths, identity mismatches, and digest failures before native loading.

Manifest reads are bounded before allocation; duplicate JSON fields and reparse points in the
install-root ancestry are rejected. Keep installed bytes quiescent and owner-controlled through
native loading: these checks do not authenticate the publisher or prevent concurrent replacement.
The managed assembly has already been admitted by the host before its package validation runs.

Steam credentials, Steamworks binaries, host assemblies, game data, and local installation paths
must remain outside the repository and CI. Do not add arbitrary DLL loading, dynamic dependency
resolution, or upload credentials to the package or workflow. Report a suspected bypass with a
synthetic package and the exact source revision.
