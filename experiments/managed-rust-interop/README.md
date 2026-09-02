# Managed .NET 9 to Rust runtime addon

This game-mod-owned directory contains the narrow runtime addon proof: a managed loader-compatible
assembly calls a Rust native library through a versioned C ABI and emits a visible load marker from
the actual STS2 initializer. After the ABI smoke call succeeds, it adds a top-layer in-game debug
banner only when the game is launched with the exact `--debug` argument. The banner reads
`AI-ASCENSION STS2 POC` and `DEBUG | Rust ABI 1 | 19 + 23 = 42`. Normal launches retain the
bounded log marker but do not add a visible overlay. It also has an explicitly opt-in launch mode
that applies the host-equivalent full profile unlock automatically. Normal launches do not change
profile progress. It is a development package and must only be installed in an explicitly authorized
test environment.

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
not enable the addon in the game's Mods menu; that remains a one-time manual step if the profile has
not already accepted the addon.

## Optional automatic full unlock

Pass `--ai-ascension-unlock-all` to `SlayTheSpire2.exe` when starting the game. Once the host has
loaded the active profile, the mod calls the same progress APIs as the in-game `unlock all` command,
saves the profile, and exits the one-shot operation. No keyboard input, console focus, or direct save
file editing is involved.

The flag marks all cards, relics, potions, events, acts, monsters, and epochs as discovered, sets
every character's maximum ascension to 10, and sets the multiplayer maximum ascension to 10. It is
idempotent. It does not unlock achievements or change the preferred ascension values, matching the
host command's behavior.

For the repeated WSL build/install/relaunch flow, use the wrapper's shorthand:

```bash
./experiments/managed-rust-interop/dev-cycle.sh --unlock-all
```

That shorthand passes the canonical `--ai-ascension-unlock-all` argument only on that relaunch. A
normal invocation of `dev-cycle.sh` installs and starts the addon without changing profile progress.

## Runtime probe

When `STS2_RUNTIME_TOKEN` is supplied, initialization also starts the bounded loopback runtime
adapter on `STS2_RUNTIME_PORT` (default `15526`). It exposes `/health/ready`,
`/api/v1/runtime/state`, and `/api/v1/runtime/action` with bearer authentication. Requests are
copied into a bounded managed queue and processed on the Godot main thread. The only admitted action
is `show_runtime_probe`; it displays the live status overlay and returns a fresh
`status_overlay_visible` witness. The action is an integration probe, not a gameplay mutation.

The exact STS2 v0.107.1 Windows x86-64 host probe is recorded in the target evidence report. The
package remains scoped to that focused runtime proof: the runtime token, host assemblies, game files,
saves, and logs are not stored or packaged, and gameplay mutation is not implemented.
