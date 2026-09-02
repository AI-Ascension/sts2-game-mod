# Managed .NET 9 to Rust runtime addon

This game-mod-owned directory contains the narrow runtime addon proof: a managed loader-compatible
assembly calls a Rust native library through a versioned C ABI and emits a visible load marker from
the actual STS2 initializer. After the ABI smoke call succeeds, it adds a top-layer in-game debug
banner only when the game is launched with the exact `--debug` argument. The banner reads
`AI-ASCENSION STS2 POC` and `DEBUG | Rust ABI 1 | 19 + 23 = 42`. Normal launches retain the
bounded log marker but do not add a visible overlay. It is a load-smoke package, not the gameplay
implementation, and must only be installed in an explicitly authorized test environment.

The managed project references the operator-supplied `sts2.dll` and `GodotSharp.dll` only at build
time, exposes the host's `ModInitializer`, loads `ai_ascension_sts2_poc.dll`, verifies ABI version
1, and checks that the native `19 + 23` smoke call returns `42`. The companion is a Windows x86-64
Rust `cdylib`. `package-runtime-addon.sh` builds and stages the three files required by the game:
the managed DLL, the unique native DLL, and `AIAscensionSTS2Poc.json`.

The source remains in this directory to preserve its existing ownership and workspace placement.
Generated `bin/`, `obj/`, and `target/` output is excluded. The host assembly, game files, saves,
profiles, credentials, and runtime logs are never copied into the repository or package.

## Optional debug overlay

Launch the exact game executable with `--debug` to show the in-game ABI smoke details:

~~~text
SlayTheSpire2.exe --debug
~~~

The argument is matched as a standalone, case-sensitive command-line value. Arguments such as
`--debug=true` do not enable the overlay.

## Repeated local build/install cycle

For the Windows game under WSL, `dev-cycle.sh` builds the Rust companion and managed loader, stops
`SlayTheSpire2.exe` after the build succeeds, copies the three package files into the game's `mods/`
directory, and relaunches the game:

```bash
export STS2_GAME_DIR='/mnt/c/Program Files (x86)/Steam/steamapps/common/Slay the Spire 2'
./experiments/managed-rust-interop/dev-cycle.sh
```

The script only targets the exact `SlayTheSpire2.exe` process and the three `AIAscensionSTS2Poc`
package files. Existing installed files are backed up under the ignored `.sts2-dev/backups/`
directory. Use `--no-launch` for an install-only cycle, `--dry-run` to inspect the actions, or
`--no-kill` only when the game is already stopped and file locking is not a concern. The script does
not change profile settings or enable the addon in the game's Mods menu; that remains a one-time
manual step if the profile has not already accepted the addon.
