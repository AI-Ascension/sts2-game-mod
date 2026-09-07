# Isolated visible combat demo

This opt-in host adapter runs one real single-player Ironclad combat against the first
alphabetically ordered weak encounter. It uses a fixed visible seed and runtime-v3 gameplay.
It is a combat demonstration, not full-run navigation or a release compatibility claim.

The operator must supply a disposable Windows host directory with its own `override.cfg`
and Godot user directory. Keep proprietary files, logs, saves, and generated addons outside
this repository. The original Steam installation is not the demo install target.

`experiments/managed-rust-interop/live-combat-demo.ps1` accepts `HostDirectory`,
`UserDirectory`, `LogPath`, `StopFile`, `Seed`, and `Port`. A fresh runtime credential is
read from stdin. The launcher owns only its spawned process and stops it on stop-file,
exit, or the 15-minute deadline.

Before launch, select `-Display -1 -Width 1280 -Height 720 -WindowMode windowed`.
Display indexes are zero-based Godot screen indexes; -1 (default) selects the primary display.
Available modes are `windowed`,
`fullscreen`, `borderless`, and `maximized`. Fullscreen/maximized dimensions follow the
display; resolution is the requested window size. The host checks screen availability,
sets its isolated settings, and reports the actual screen, dimensions and mode.

The native settings menu's AI-Ascension tab also offers display, resolution, and window mode.
Apply updates the running window and atomically saves preferences in the active Godot user
directory. Omitted launcher video options reuse those preferences; explicit options override
individual saved fields. A disconnected saved display falls back to the current primary display.
On Windows the resolution dropdown enumerates monitor-compatible modes for the selected
display, deduplicates refresh-rate variants, and limits window sizes to its current desktop
dimensions. Changing displays refreshes the list. On Linux the fallback uses the detected
usable window area, excluding desktop panels, instead of advertising the full desktop as
a window size. The current observed window size is also retained when it fits that area.
There is no fixed resolution preset list. Fullscreen and maximized use the desktop
dimensions and disable the resolution selector.

Apply waits for the window manager before saving, verifies the resulting mode and display,
and persists the actual client size when the desktop adjusts a window. The status text and
resolution selector reflect that adjustment. For example, a 1024x768 GNOME desktop can
allow a 958x736 borderless window but only a 958x699 decorated client area.

For an authorized disposable-host menu check, build `GameLoaderProbe.csproj` with
`-p:EnableVideoMenuProbe=true` and the usual `STS2GameDataDir`. This opt-in build opens the
native menu instead of starting combat, checks each display's resolution choices, emits the
actual controls' selection/apply signals, checks persisted and observed window settings, and
saves `user://video-menu-probe.png`. Normal builds exclude this probe. Rebuild without the
property before the combat/relaunch check; retain external logs and compare the actual window
report with the preferences saved by the menu. Source-only preference tests do not prove
native menu behavior or monitor compatibility.
Set `STS2_VIDEO_PROBE_VERIFY_SAVED=1` for a subsequent probe launch to verify the previously
saved window size and mode before exercising the controls again.

The game must have accepted this mod in the disposable profile. The local-only save
backend preserves that consent, including after patch-triggered first-launch prompts.
It never silently grants consent to arbitrary installed mods. A patch that prevents the
addon from loading still requires the host's normal mod confirmation and relaunch.

The adapter checks single-player state and the host thread, projects owned values, and
uses the current host action catalog. Completion requires the exact queued action to finish
successfully and a visible effect. Unknown outcomes reconcile under the same operation ID.
Seed visibility is intentional. Card descriptions, powers and enemy intent details are
not yet projected; the current intent value is explicitly unknown.

Confirmed on 2026-09-05 with host v0.107.1 (59260271): the Rust Ollama bridge selected
18 actions, each received a host completion witness, and the combat reached Reward.
Fresh-process replay repeated all 18 choices and checked each visible pre-action observation.
Fullscreen replay also checked the terminal observation. Windowed 1280x720 on display 0
and fullscreen 2560x1440 on primary display 2 were observed. A changed seed was rejected
before the first replay dispatch.
Borderless 1024x768 on display 1 and maximized 1920x1009 on display 0 were also observed.

Use `bash experiments/managed-rust-interop/live-combat-session.sh --help` for the complete
repeatable operator entrypoint. Supply explicit host/user/artifact directories and gateway,
MCP, harness and provider executable paths. It creates fresh role-separated credentials,
launches the visible host, runs the configured model through the harness, and retains bounded
external logs. Use `--display`, `--width`, `--height`, and `--window-mode` before launching.
`--replay-trajectory` replays a completed model trajectory without inference; use the same seed.
The selected semantic action must exist in the fresh catalog and visible game content must
match. Live observation generation numbers are deliberately not compared across processes.
`--hold-seconds` keeps the result visible after completion (default 300; maximum 600).

For a bounded standard campaign/map handoff, add `--campaign-map` together with a pinned
`--map-renderer-binary` and its lowercase `--map-renderer-sha256`. This mode requires the OpenAI
Astra provider, starts a host-generated standard run, requests the complete current map graph and
verified PNG, and admits exactly one `start_run` followed by exactly one current legal
`select_map_node`. The managed host guard withholds every later gameplay mutation, and the
launcher requires a zero harness exit and an ordered trace containing one settled setup action,
one settled map action, and no other action kind. The runner allows two
model decision steps, one read-only recovery attempt, and a 90-second provider deadline. The
option does not resume a save, accepts no seed or replay trajectory, and is preparation for an
authorized live run; it does not claim campaign completion or a played combat.

The launcher also enables `STS2_CAMPAIGN_MAP_BOUND=true` in the harness. That policy rejects a
different action kind before dispatch and advances only after runner-verified settlement. Before
lease cleanup it records a fresh read-only observation without a third model decision. A map
outside the current map-decision stage is explicitly unavailable in that final capture; the
earlier image remains historical. The durable `trajectory.jsonl` contains bounded decisions,
rationales, receipts, and bundle links, while `map-artifacts/` holds immutable bundles and the feed.
Run `bash experiments/managed-rust-interop/live-campaign-trace.test.sh` to exercise trace rejection
and both synchronous and waited settlement records without a host or provider. Trace checks bind
the image, bundle, action receipt, and fresh observation to their execution and generation, and
reject reused operation IDs or records placed before their prerequisites.

Campaign/map mode also verifies the disposable host before starting anything. Its
`data_sts2_windows_x86_64/sts2.dll` must be the supported v0.107.1 release `59260271` with
SHA-256 `a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52`; the matching
`GodotSharp.dll` hash is `0e4897ecdfb31456a97c7d8028dfb8d7dbdc632e2f73fc9b438d7b266a139289`.
The accepted addon directory defaults to `<host-dir>/mods` and must contain regular files named
`AIAscensionSTS2GameMod.dll`, `AIAscensionSTS2GameModNative.dll`, and
`AIAscensionSTS2GameMod.json`; pass `--addon-dir` when the staged addon is kept elsewhere.
The launcher records the host, executable, override, and addon hashes under the external run
artifact directory as `baseline.sha256` and `addon.sha256`, and verifies that baseline after the
owned host stops. The user directory must be an explicit absolute Windows path outside the host,
repository, addon, and artifact directories; a pre-existing directory is allowed only when it is
already a disposable copy whose baseline is retained by the operator.

Prepare the run by copying the supported installation to a disposable host directory, hashing the
original host and addon files, retaining the copy's baseline, and placing a separate disposable
Godot user directory at the path passed to `--user-dir`. Stage and hash the three addon files
outside this repository, then run the bounded handoff with explicit binaries:

~~~text
bash experiments/managed-rust-interop/live-combat-session.sh \
  --host-dir /path/to/disposable-sts2 \
  --user-dir 'C:\Temp\sts2-map-user-20260907' \
  --addon-dir /path/to/disposable-sts2/mods \
  --artifacts-dir /tmp/sts2-map-artifacts \
  --gateway-binary /path/to/sts2-gateway-runtime \
  --mcp-binary /path/to/sts2-mcp-server \
  --harness-binary /path/to/sts2-harness-runtime \
  --provider-binary /path/to/sts2-astra-bridge \
  --map-renderer-binary /path/to/map-visualizer \
  --map-renderer-sha256 LOWERCASE_SHA256 \
  --powershell-binary /mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe \
  --campaign-map
~~~

Do not use `dev-cycle.sh` or the active Steam installation for this handoff. The campaign/map
launcher does not install an addon, copy a save, or select a game screen; it only accepts an
already prepared disposable host and records the hashes needed for post-run restoration.

Select the harness's `sts2-astra-bridge` as `--provider-binary` to play with OpenAI
`gpt-6-astra` using an existing Codex login. The launcher reads `--describe` before launch,
records the provider/model identity, and explicitly inherits only HOME/PATH for this provider.
The Ollama bridge remains selectable. Additional providers are future work and require a
provider adapter, without changing the game action or replay contracts.

During initial isolation setup before the addon loaded, the host wrote two Steam local-cache
files. Both were restored from the original local saves and byte-checked. Full external
backups were retained. Steam app cloud sync was disabled; remote cloud state was not verified.
Subsequent loaded-addon runs use local-only storage. This incident means a fresh host copy
alone must not be claimed to prevent Steam-cache writes before mod initialization.
