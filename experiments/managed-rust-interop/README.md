# Managed .NET 9 to Rust runtime addon

This game-mod-owned directory contains the narrow runtime addon proof: a managed loader-compatible
assembly calls a Rust native library through a versioned C ABI and emits a visible load marker from
the actual STS2 initializer. After the ABI smoke call succeeds, it adds a top-layer in-game debug
banner only when the game is launched with the exact `--debug` argument. The banner reads
`AI-ASCENSION STS2 POC` and `DEBUG | Rust ABI 1 | 19 + 23 = 42`. Normal launches retain the
bounded log marker but do not add a visible overlay. It also has an explicitly opt-in full profile
unlock action. Normal launches do not change profile progress. It is a development package and must
only be installed in an explicitly authorized test environment.

The managed project references the operator-supplied `sts2.dll` and `GodotSharp.dll` only at build
time, exposes the host's `ModInitializer`, loads `ai_ascension_sts2_poc.dll`, verifies ABI version
1, and checks that the native `19 + 23` smoke call returns `42`. The companion is a Windows x86-64
Rust `cdylib`. `package-runtime-addon.sh` builds and stages the three files required by the game:
the managed DLL, the unique native DLL, and `AIAscensionSTS2Poc.json`.

The source remains in this directory to preserve its existing ownership and workspace placement.
Generated `bin/`, `obj/`, and `target/` output is excluded. The host assembly, game files, saves,
profiles, credentials, and runtime logs are never copied into the repository or package.

## Built-in profile settings

The addon owns its settings panel and injects one `AI-Ascension` tab into the game's native settings
screen. It uses the host `NSettingsTabManager` and `NSettingsPanel` seam directly; no ModConfig or
other settings-framework mod is required or loaded. The existing General, Graphics, Sound, and
Input tabs are left intact. If ModConfig is still installed, it may add its separate Mods tab, but
AI-Ascension does not use or modify it.

The built-in panel contains:

| Label | Behavior |
| --- | --- |
| `Runtime API` | Enables the authenticated listener when a token is configured. The default is on; changes apply on the next launch. |
| `Bind address` | Selects the local hostname or IP address for the runtime listener. The dropdown includes loopback, all interfaces, the detected machine hostname, and detected local IPv4 addresses. |
| `Network port` | Selects a port from `1024` through `65535`; the default is `15526`. |
| `Target profile` | Selects Profile 1, Profile 2, or Profile 3. The choice is persisted in the mod's own user-data settings file. |
| `Apply on next launch` | Saves the staged runtime API, bind address, and port values. A game restart is required before they become active. |
| `Reset` | Restores the runtime API default, loopback address, and port `15526`. |
| `Apply full profile unlock` | Switches to the selected profile through the host save manager, then queues the guarded unlock operation. Only the selected profile is modified. |

Network values are staged in the panel and saved by `Apply on next launch` in the mod's own
user-data settings file. `STS2_RUNTIME_PORT` and `STS2_RUNTIME_BIND_ADDRESS` remain available as
explicit environment-variable overrides for automation. The listener still requires
`STS2_RUNTIME_TOKEN`; choosing `0.0.0.0` exposes the authenticated listener on all local interfaces
and should only be used with an intentionally configured firewall and trusted network. The panel
shows whether the token is configured and the listener's current startup status without displaying
the token.

The standalone, case-sensitive `--debug` argument remains an explicit developer diagnostic and
continues to show the overlay. The profile selector and Apply action are available from the
AI-Ascension tab without an additional mod. Settings UI construction is limited to the new tab and
its panel; it does not change global settings values, other mod registrations, or other panels.

The Apply button uses a profile-readiness and main-thread queued-attempt path. It does not edit save
files directly or create a concurrent second attempt. A failed or not-yet-ready attempt emits a
bounded diagnostic and does not report success.

The profile selection can be returned to Profile 1 from the dropdown. The Apply button has no
separate persisted value and the addon does not create a competing reset system.

### Profile mutation boundary

The full unlock marks all cards, relics, potions, events, acts, monsters, and epochs as discovered;
sets every character's maximum ascension to `10`; and sets the multiplayer maximum ascension to
`10`. It does not unlock achievements, change preferred ascension values, select an ascension for
the user, edit arbitrary save fields, or expose content-category subsets. The in-game action
targets the selected profile (1, 2, or 3) by using the host `SaveManager.SwitchProfileId` API before
applying the guarded mutation. The standalone command-line argument continues to target the active
profile.

The settings feature does not add controls for runtime tokens, HTTP routes, MCP actions, AI policy,
seeds, or native mod enablement. The game's native Installed Mods checkbox continues to own
enablement, and environment credentials remain outside the settings system.

The existing standalone, exact `--ai-ascension-unlock-all` command-line argument remains available
as an explicit command-line path without any settings-framework mod. The argument comparison is case-insensitive, but the
argument must still match the complete standalone value; forms such as `--ai-ascension-unlock-all=x`
do not enable it. It performs the same guarded one-shot profile operation, and the
`dev-cycle.sh --unlock-all` shorthand continues to pass that canonical argument for an authorized
local cycle.

The built-in settings registration is fail-open: if the host settings seam is unavailable, the
managed initializer, native ABI smoke call, `--debug` overlay, and command-line unlock path still
operate. The settings registration, UI rendering, callback behavior, and profile mutation remain
separately unverified until exercised against an authorized exact STS2 host; the existing load-smoke
evidence does not by itself prove settings UI or game-profile compatibility.

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

When `STS2_RUNTIME_TOKEN` is supplied and `Runtime API` is enabled, initialization starts the bounded
runtime adapter on the saved bind address and port (default `127.0.0.1:15526`).
`STS2_RUNTIME_PORT` and `STS2_RUNTIME_BIND_ADDRESS` override the saved values when present. It exposes `/health/ready`,
`/api/v1/runtime/state`, and `/api/v1/runtime/action` with bearer authentication. Requests are
copied into a bounded managed queue and processed on the Godot main thread. The only admitted action
is `show_runtime_probe`; it displays the live status overlay and returns a fresh
`status_overlay_visible` witness. The action is an integration probe, not a gameplay mutation.

The exact STS2 v0.107.1 Windows x86-64 host probe is recorded in the target evidence report. The
package remains scoped to that focused runtime proof: the runtime token, host assemblies, game files,
saves, and logs are not stored or packaged, and gameplay mutation is not implemented.
