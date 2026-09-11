# Release bundle tooling

`build-source-bundle.sh` creates a deterministic production-source release archive from an exact Git
commit. It applies the checked-in `tools/release/source-distribution-policy-v1.json` exact tracked-path
allowlist and fails closed when the resolved tree differs from that policy. The policy omits diagnostic
and gameplay fixture sources from the distributed production source while retaining the production
loader, native, protocol, and release inputs. The manifest preserves the resolved commit and original
full Git tree identity and records the policy hash, excluded paths, included path count, and included
content digest. The archive is suitable for Windows and Linux source distribution; it is not a native
game addon and does not promote host, Workshop, or gameplay compatibility.

Build the two platform-labelled archives from a clean checkout:

```text
SOURCE_DATE_EPOCH=0 bash tools/release/build-source-bundle.sh 0.4.0 windows-x86_64 /tmp/release/windows HEAD
SOURCE_DATE_EPOCH=0 bash tools/release/build-source-bundle.sh 0.4.0 linux-x86_64 /tmp/release/linux HEAD
```

Each output directory contains a `.tar.gz` archive and a sidecar SHA-256 file. The archive contains
`RELEASE-MANIFEST.json`, the versioned source policy, and `SHA256SUMS`; verify the policy, manifest,
and embedded checksums before distribution:

```text
(cd /tmp/release/linux && sha256sum --check --strict ai-ascension-sts2-game-mod-0.4.0-linux-x86_64-source.tar.gz.sha256)
tar -xzf /tmp/release/linux/ai-ascension-sts2-game-mod-0.4.0-linux-x86_64-source.tar.gz
(cd ai-ascension-sts2-game-mod-0.4.0-linux-x86_64-source && sha256sum --check --strict SHA256SUMS)
```

`test-source-bundle.sh` builds the same source twice and checks byte parity, the archive checksum
inventory, the manifest identity, and unsafe-label refusal. A native addon still requires the
operator-supplied STS2 host assemblies and the separate `package-runtime-addon.sh` path; those files
remain outside this bundle and every public release archive. The build script keeps the existing
Windows invocation and accepts `--platform linux-x86_64` to select the Linux Rust target and `.so`
filename. It defaults `SOURCE_DATE_EPOCH` to `0` so native linker timestamps are reproducible;
release operators may set the approved source timestamp explicitly. Stage the resulting three runtime files with `tools/workshop/package-platform-item.sh`,
which emits a platform-specific five-file Workshop package. Keep Windows and Linux packages under
separate first-party published-file IDs until a shared Workshop contract is reviewed.

For a disposable install, update, or rollback, use the file-scoped operator tool after inspecting
the package and stopping the selected game installation:

```text
bash tools/release/install-runtime-addon.sh install windows-x86_64 PACKAGE MODS BACKUP
bash tools/release/install-runtime-addon.sh rollback MODS BACKUP
```

The installer verifies the complete canonical first-party manifest identity, bounded manifest and
checksum metadata, platform, exact native filename, and `SHA256SUMS`; it preserves unrelated files
and records replaced files in the supplied backup directory. It binds the consumer App ID to the
current first-party policy; it cannot establish a live Steam/Workshop or installed-game identity.
Run
`bash tools/release/test-runtime-lifecycle.sh` for a synthetic Windows/Linux install/update/rollback
check. These fixture checks do not mutate a host or contact Steam.
