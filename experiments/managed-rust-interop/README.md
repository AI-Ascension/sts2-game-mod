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

## Optional ModConfig settings

The addon can register settings with the optional ModConfig-STS2 framework. When that framework is
installed and exposes its compatible public API, the addon appears under the game's `Settings ->
Mods` page. The bridge discovers `ModConfig.ModConfigApi`, `ModConfig.ConfigEntry`, and
`ModConfig.ConfigType` by reflection, so this addon has no hard DLL or PCK dependency on ModConfig.
The registration uses the stable mod ID `AIAscensionSTS2Poc`. If ModConfig is absent or
incompatible, the addon continues through its normal loader and ABI path without a settings page.

The bridge was implemented against the public ModConfig-STS2 API inspected at commit
[`639eb97fa7824e94a43339913c51433117207d05`](https://github.com/xhyrzldf/ModConfig-STS2/tree/639eb97fa7824e94a43339913c51433117207d05)
in the authoritative [ModConfig-STS2 repository](https://github.com/xhyrzldf/ModConfig-STS2).
That framework is attributed to PiPiFanDev and is licensed under MIT. No ModConfig framework
source or DLL is bundled with this addon; only the narrow reflection bridge is included.

The available controls are:

| Label | Key | Type | Default | Behavior |
| --- | --- | --- | --- | --- |
| `Show debug overlay on launch` | `show_debug_overlay` | Toggle | `false` | Queues the existing ABI diagnostic overlay after successful loader and Rust ABI initialization. It does not change gameplay. |
| `Unlock all profile content on next launch` | `unlock_all_on_next_launch` | Toggle | `false` | Requests one full profile unlock after the active profile is initialized. This changes profile progress and should be enabled only deliberately. |
| `Apply full profile unlock now` | `apply_full_profile_unlock_now` | Button | Not persisted | Available only when the detected framework supports a button with a safe callback. It schedules the same guarded unlock operation immediately. |

The debug toggle is diagnostic only. The existing standalone, case-sensitive `--debug` argument
remains an explicit command-line override and continues to show the overlay even when ModConfig is
not installed. Values are read only after the deferred framework registration point, so startup
does not depend on settings being synchronously available.

The launch unlock is a one-shot request. After the active profile is ready, the addon uses the
host-equivalent progress APIs, saves through the host save manager, and clears the setting through
the settings API only after that save succeeds. A failed or not-yet-ready attempt retains the
request and emits a bounded diagnostic so a later launch can retry. The optional Apply button uses
the same profile-readiness, main-thread, queued-attempt path; it does not enable the persistent
launch toggle, edit save files directly, or create a concurrent second attempt. If the framework
cannot safely support that callback, the button is omitted rather than rendered as a no-op.

The framework's restore-defaults action owns reset behavior. It returns both persistent toggles to
`false`; the Apply button has no persisted value. The addon does not create a competing reset file
or duplicate persistence system.

### Profile mutation boundary

The full unlock marks all cards, relics, potions, events, acts, monsters, and epochs as discovered;
sets every character's maximum ascension to `10`; and sets the multiplayer maximum ascension to
`10`. It does not unlock achievements, change preferred ascension values, select an ascension for
the user, edit arbitrary save fields, or expose content-category subsets.

The settings feature does not add controls for runtime tokens, ports, bind addresses, HTTP routes,
MCP actions, AI policy, seeds, or native mod enablement. The game's native Installed Mods checkbox
continues to own enablement, and environment credentials remain outside the settings system.

The existing standalone, exact `--ai-ascension-unlock-all` command-line argument remains available
as an explicit fallback without ModConfig. The argument comparison is case-insensitive, but the
argument must still match the complete standalone value; forms such as `--ai-ascension-unlock-all=x`
do not enable it. It performs the same guarded one-shot profile operation, and the
`dev-cycle.sh --unlock-all` shorthand continues to pass that canonical argument for an authorized
local cycle.

ModConfig registration is optional and fail-open: its absence or an incompatible API does not
prevent the managed initializer, native ABI smoke call, `--debug` overlay, or command-line unlock
fallback from operating. In that situation no in-game settings UI is claimed. The settings
registration, UI rendering, callback behavior, and profile mutation remain separately unverified
until exercised against an authorized exact STS2 host and compatible ModConfig installation; the
existing load-smoke evidence does not by itself prove settings UI or game-profile compatibility.

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
