# Managed .NET 9 to Rust runtime addon

This game-mod-owned directory contains the narrow runtime addon proof: a managed loader-compatible
assembly calls a Rust native library through a versioned C ABI and emits a visible load marker from
the actual STS2 initializer. It is a load-smoke package, not the gameplay implementation, and must
only be installed in an explicitly authorized test environment.

The managed project references the operator-supplied `sts2.dll` and `GodotSharp.dll` only at build
time, exposes the host's `ModInitializer`, loads `ai_ascension_sts2_poc.dll`, verifies ABI version
1, and checks that the native `19 + 23` smoke call returns `42`. The companion is a Windows x86-64
Rust `cdylib`. `package-runtime-addon.sh` builds and stages the three files required by the game:
the managed DLL, the unique native DLL, and `AIAscensionSTS2Poc.json`.

The source remains in this directory to preserve its existing ownership and workspace placement.
Generated `bin/`, `obj/`, and `target/` output is excluded. The host assembly, game files, saves,
profiles, credentials, and runtime logs are never copied into the repository or package.
