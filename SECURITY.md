# Security Policy

## Scope

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

The project does not promise a response time or a bounty. This policy is not permission to access
another person's game profile, host installation, network, or data.

## Workshop package boundary

Workshop packages are first-party executable content, not an extension point for arbitrary
third-party code. The checked-in package tool stages an exact allowlist, records sizes and SHA-256
digests, and emits a manifest and checksum inventory. The managed loader rejects unexpected files,
reparse points, unsafe paths, identity mismatches, and digest failures before native loading.

Steam credentials, Steamworks binaries, host assemblies, game data, and local installation paths
must remain outside the repository and CI. Do not add arbitrary DLL loading, dynamic dependency
resolution, or upload credentials to the package or workflow. Report a suspected bypass with a
synthetic package and the exact source revision.
