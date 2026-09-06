# Video settings: Windows VM regression

## Test artifact

The test addon uses the merged Linux window-transition correction from
`a80e1ee0e70359c6fbee99108f64f11ab4ed07a9`. The opt-in native-menu probe DLL has
SHA-256 `76ecef257e24ff093ab7399d243507d55ac84924e2316d5cdad72b690ca31348`.
It was built against the installed Windows STS2 v0.107.1 host API. The normal
addon excludes the probe; its DLL has SHA-256
`988731bea7b7e5c734aad8f7d54857fb87b06d1df88eb08b4d98036bc29cd2b7`.

## VM preparation

The authorized Windows fixture was copied from a powered-off generation-2
Hyper-V VM into a standalone dynamic VHDX. The source differencing disk and
parent were retained. Preparation ran only in the copy: Windows 11 Pro, the
current game in an isolated test directory, the addon, Steam, VirtIO drivers,
and the QEMU guest agent. Disk encryption was removed from the copy before
hardware migration. The source VM was not booted during this operation.

An initial conversion was rejected when its comparison source changed: the
temporary copy had been restarted interactively. The replacement conversion
reads the frozen parent disk of a checkpoint created with `ProductionOnly`
checkpoint policy. The temporary VM writes to a separate AVHDX. This avoids
depending on the temporary VM remaining off throughout a long conversion.

The destination configuration uses KVM/Q35, two virtual CPUs, 8 GiB RAM,
Microsoft-key Secure Boot firmware, a SATA boot disk, VirtIO networking and
guest-agent channel, and a virtual display. Its VNC listener is loopback-only;
the test console uses an SSH tunnel. No host GPU is reassigned.

The VNC console also has a live and persistent `qemu-vdagent` clipboard channel.
The Windows SPICE agent installer matched its publisher's SHA-256 checksum and
exited successfully. The service, interactive agent, and channel connection were
confirmed; user clipboard contents were not inspected. The operator completed
Steam sign-in through this console.

## Verification status

The replacement QCOW2 passed `qemu-img check`. Its complete 80 GiB virtual
address space matched the frozen VHDX checkpoint in eight contiguous 10 GiB
`qemu-img compare` ranges, all with exit status zero. The frozen source's file
size and modification timestamp also remained unchanged during comparison.
The transferred 15,994,975,744-byte image matched SHA-256
`2348464ed6329e5e34949d69ceea873959fe4b269aae6b3253672aa3c5cfe395`
before boot and passed another `qemu-img check` on the destination. This is a
pre-boot transfer hash; a running guest subsequently changes its disk.

Windows 11 Pro 10.0.26200 booted on KVM. The guest agent confirmed two CPUs,
approximately 8 GiB RAM, active VirtIO networking, a running guest-agent
service, and the expected probe DLL hash. The Red Hat VirtIO GPU DOD driver
`100.103.104.30200` reported a 1280x800 desktop. Steam started in the interactive
desktop session. After operator sign-in, the game initialized Steam successfully
and installed the isolated local-only save backend.

Graphics preflights in both the temporary Hyper-V copy and the KVM guest started
the actual game using `D3D12 12_0`, Forward+, and `Microsoft Basic Render Driver`.
Hyper-V returned `ConnectToGlobalUser failed` without a signed-in account. KVM
reached the game's Steam error screen while the Steam client was still verifying
its installation. Both owned game processes were stopped afterward. These are
software-renderer startup results, not menu passes.

The uncapped KVM software-rendering launch made guest command execution
unresponsive. With a 15 FPS cap and below-normal process priority, the native
menu probe completed, although the operator still reported poor performance.
This cap is not a measured frame rate or a hardware-acceleration result.

## Confirmed native-menu results

The display reported a 1280x800 desktop and 1280x752 usable area. Its detected
catalog contained 640x480, 800x600, 1024x768, 1280x720, 1280x768, and 1280x800.
The probe confirmed borderless and windowed 1024x768, fullscreen 1280x800,
maximized 1280x729, and a return to windowed mode. Actual mode, display, and
client size matched persisted preferences; the selector and scrolling content
remained visible. Independent fresh launches restored both windowed and
fullscreen preferences and completed the menu probe again.

Private evidence SHA-256 digests:

- Initial menu log: `85126e5ceda147994688e5e06d09052341f9028013f00ecc91e4d12c72671593`.
- Native viewport image: `c6cdca977b10cc582926f80deb31b067d0731592622554b06985e5b7154de78a`.
- Windowed restoration log: `e68a3fe0e5c9533d81497b853529f7dca52c1fa5838be92e00a300eab79a030d`.
- Fullscreen restoration log: `32973df2cdbc7b7200bddcbe50e5edc45781144d3d1c244f704c6b4ce4a13491`.

These are Windows native-menu passes using the software renderer. They do not
establish GPU acceleration, LLM-controlled gameplay, or replay on this VM.

Keep proprietary game assets, VM disks, credentials, private profiles, and raw
guest logs outside the repository. The native menu procedure is documented in
[the live demo guide](../LIVE_COMBAT_DEMO.md); the independent Linux results
are in [the Linux VM evidence](video-settings-linux-vm-20260905.md).
