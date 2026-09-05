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

## Verification status

Disk conversion and destination runtime verification are in progress. VM
preparation alone is not a Windows game, menu, or full v3 gameplay pass.

Keep proprietary game assets, VM disks, credentials, private profiles, and raw
guest logs outside the repository. The native menu procedure is documented in
[the live demo guide](../LIVE_COMBAT_DEMO.md); the independent Linux results
are in [the Linux VM evidence](video-settings-linux-vm-20260905.md).
