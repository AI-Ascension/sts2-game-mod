# Video settings: Linux VM regression

## Confirmed scope

The native AI-Ascension menu was exercised in an Ubuntu 24.04.4 libvirt/KVM guest with
a GNOME Wayland desktop, virtual display, native Linux STS2 v0.103.2 (89765e1e), and
the OpenGL rendering path. The opt-in menu probe uses the actual host controls and
shared production video-setting methods. The managed probe was built against the
v0.107.1 host API; the native Rust library was built for Linux. No proprietary
assemblies, screenshots, saves, or raw host logs are included here.

The initial probe failed: the desktop reported 1024x768, but its usable area was
958x736. A borderless window was constrained to that area, while the menu had
already saved 1024x768. A decorated window needed a further 37 pixels for its frame.
Switching directly from fullscreen to maximized also failed before the desktop had
finished leaving fullscreen.

The corrected apply path waits for the desktop transition, verifies mode/display,
saves the actual client size for windowed modes, and updates the resolution selector.
The Linux fallback offers usable area and the current observed window size.

| Actual mode | Observed client size | Result |
| --- | --- | --- |
| Borderless | 958x736 | Confirmed applied and persisted |
| Windowed | 958x699 | Confirmed adjustment shown and persisted |
| Fullscreen | 1024x768 | Confirmed; resolution selector disabled |
| Maximized | 958x699 | Confirmed after leaving fullscreen; selector disabled |
| Windowed after maximized | 958x699 | Confirmed |

Fresh-process probes also confirmed restoration of saved windowed and fullscreen
settings. The final probe checked mode identity, resolved primary display, saved
client size, selector contents, visibility, and scrolling. The Windows desktop
regression on v0.107.1 also passed all four modes and the return to windowed.

The 435 original Linux save files and 15 Steam remote-save files remained unchanged.
Starting the previously inactive Steam client updated its client package and the
`remotecache.vdf` metadata file. This is not a claim that Steam's remote cloud state
was unchanged or independently verified.

## Limits

Full v3 gameplay on this Linux installation is unverified. Building the complete
loader against v0.103.2 failed with ten missing-API errors, including
`SaveManager.IsProfileInitialized`, `ModelDb.ActsByIndex`, and player-combat phase/turn
members. Passing the narrower menu probe does not resolve that version mismatch.

A Windows VM was not present in the inspected Train libvirt inventory; all four
registered guests identified Ubuntu 24.04. Windows VM testing remains unverified.
The Windows regression above was on the existing disposable desktop host, not a VM.

Use the opt-in probe procedure in [the live demo guide](../LIVE_COMBAT_DEMO.md).
Keep its output outside the source checkout and rebuild without the probe property
for a normal addon. Retain exact source and binary hashes with external run evidence.
