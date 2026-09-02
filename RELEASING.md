# Release policy and procedure

This target has no release artifact yet. Preparing a build, publishing a package, installing a
mod, and verifying a release are separate states. None is authorized by ordinary foundation work.

## Authority and versioning

Only an explicitly authorized maintainer may publish or deploy. Agents may prepare and inspect a
candidate, but must not create or move tags, upload artifacts, install a mod, mutate a profile, or
deploy without explicit authorization.

Repository, HTTP, ABI, managed host, game compatibility, and package versions are separate facts.
Do not infer one from another. A host ABI adaptation can require a compatibility release even when
the local HTTP contract is unchanged.

## Release readiness

A release may claim only the highest executed evidence level in
[docs/COMPATIBILITY.md](docs/COMPATIBILITY.md). Before publication, the exact commit, review,
policy, formatting, lint, tests, contract fixtures, security checks, packaging contents, and
host/runtime evidence must be recorded.

No release may contain:

- proprietary game assemblies or assets;
- personal saves, profiles, credentials, or machine-specific paths;
- source checkout metadata, target directories, debug output, or unrelated files; or
- an unreviewed dependency or fixture with unknown licensing.

The managed loader and each native artifact must be paired for the exact supported
operating-system and architecture. The package must include the manifest, applicable notices,
license, and user documentation while keeping host assemblies outside the package.

## Prepare and verify

1. Record the intended source revision and release classification.
2. Update the changelog, compatibility matrix, migration notes, and all authoritative versions.
3. Run the policy, format, lint, test, conformance, security, and package checks from
   [docs/TESTING.md](docs/TESTING.md).
4. Build from the exact approved source and record checksums.
5. Inspect the unpacked bytes and run install/start smoke tests only in an authorized disposable
   environment.
6. Obtain maintainer publication approval and preserve the artifact-to-commit mapping.

The host-loader smoke test must use an operator-supplied exact host installation and disposable
data. CI must not download or redistribute proprietary host files.

## Failure and post-release checks

After publication, verify checksums, package allowlists, documented startup, the HTTP index, and
the claimed host matrix from freshly downloaded bytes. Record this separately from publication.

Never rewrite an immutable tag or silently replace an artifact. Withdraw or mark a defective
release where supported, preserve diagnostic evidence without private data, and issue a corrective
version through the same gates.
