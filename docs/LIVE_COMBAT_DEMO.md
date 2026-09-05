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

During initial isolation setup before the addon loaded, the host wrote two Steam local-cache
files. Both were restored from the original local saves and byte-checked. Full external
backups were retained. Steam app cloud sync was disabled; remote cloud state was not verified.
Subsequent loaded-addon runs use local-only storage. This incident means a fresh host copy
alone must not be claimed to prevent Steam-cache writes before mod initialization.
