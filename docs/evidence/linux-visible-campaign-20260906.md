# Confirmed Linux campaign presentation, 2026-09-06

The authorized Linux KVM fixture has four vCPUs in both its persistent definition and live
allocation. During the fresh seeded replay, the native game reported 59–60 FPS after its
verified window received focus. An earlier background sample reported 30 FPS. Those are game
log measurements, not rates inferred from a video decoder or successful runtime requests.

The visible Linux window uses a read-only stream of the PID-verified game window: X11 capture,
JPEG frames in a streamable Matroska transport, an SSH tunnel, and a local GTK/GStreamer viewer.
Its transport port remains restricted to the host bridge. A separate viewer sample decoded
54.46 frames per second with zero reported dropped frames. Decoder throughput is not a count
of distinct game frames and is not a substitute for the native FPS measurement.

The stream intentionally does not forward mouse or keyboard events. Moving the local cursor
does not move the guest game's cursor. Window activation used the window manager after matching
the game PID; no keyboard or mouse events drove the game. The runtime replay remained the action
source. A fresh visible capture was required before any replay/provider process started.

Fresh visible combat capture SHA-256:
`b9e211749e4a25ce79d7789eb63e22b4e01ce046dae5b5658bec3afc258caae6`.

On a preceding fresh launch, the stream started before the game finished changing its window
geometry and failed with an X11 capture error. The visibility check refused to start gameplay.
Restarting the owned stream after geometry stabilized restored the viewer; no game action was
retried. A stream or viewer restart is separate from a host/runtime restart.

This records current presentation and operational boundaries. It does not establish full-campaign
completion, complete seeded replay, victory handling, or post-host-reboot GPU readiness.
