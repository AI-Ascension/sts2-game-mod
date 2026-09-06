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
inventory, the manifest identity, and unsafe-label refusal. A native Windows addon still requires
the operator-supplied STS2 host assemblies and the separate `package-runtime-addon.sh` path; those
files remain outside this bundle and every public release archive.
