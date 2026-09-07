# Release bundle tooling

`build-source-bundle.sh` creates a deterministic, source-only release archive from an exact Git
commit. It records the resolved commit and tree identity, a platform compatibility label, a fixed
`SOURCE_DATE_EPOCH`, a checksum inventory, and the fact that proprietary host files are excluded.
The archive is suitable for Windows and Linux source distribution; it is not a native game addon and
does not promote host, Workshop, or gameplay compatibility.

Build the two platform-labelled archives from a clean checkout:

```text
SOURCE_DATE_EPOCH=0 bash tools/release/build-source-bundle.sh 0.4.0 windows-x86_64 /tmp/release/windows HEAD
SOURCE_DATE_EPOCH=0 bash tools/release/build-source-bundle.sh 0.4.0 linux-x86_64 /tmp/release/linux HEAD
```

Each output directory contains a `.tar.gz` archive and a sidecar SHA-256 file. The archive contains
`RELEASE-MANIFEST.json` and `SHA256SUMS`; verify both before distribution:

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

The installer verifies the manifest platform, exact native filename, and `SHA256SUMS`, preserves
unrelated files, and records replaced files in the supplied backup directory. Run
`bash tools/release/test-runtime-lifecycle.sh` for a synthetic Windows/Linux install/update/rollback
check. These fixture checks do not mutate a host or contact Steam.
