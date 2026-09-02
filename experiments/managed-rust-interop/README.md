# Managed .NET 9 to Rust interop experiment

This source-only owner copy belongs to `sts2-game-mod/experiments`. It preserves the narrow proof
that a managed loader-compatible assembly can call a Rust native library through a versioned C ABI.
It is not production code and must not be installed into a valued game profile.

The experiment contains the managed probe, loader metadata probe, Rust native crate, and the
game-mod-owned `Directory.Build.props`. Its native manifest is included in the target-local Cargo
workspace created in Wave 2 so it remains a valid workspace member. The experiment remains
source-only; managed-host integration, runtime loading, and game behavior are unverified. Generated
`bin/`, `obj/`, and `target/` output was excluded.

The experiment source is retained in this directory and is owned by the game-mod target.
