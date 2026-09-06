# Windows Arc acceleration test

## Confirmed host configuration

The Windows menu regression used CPU rendering; see
[the completed menu results](video-settings-windows-vm-20260905.md).
The subsequent acceleration test detected an Intel Arc Pro B60 (PCI ID
8086:e211) using the host `xe` driver on kernel 7.0.0-31-generic. The driver
reported SR-IOV PF mode and seven supported virtual functions, initially with
none enabled.

One VF was enabled and attached to the Windows test domain using managed
VFIO passthrough. Its IOMMU group contained only that VF. The physical function
remained with the host driver. The VF received 6 GiB local memory, 640 MiB GGTT,
8192 contexts and 60 doorbells per GT, a 25 ms execution quantum, and a
500000 microsecond preemption timeout. These are tested provisioning writes,
not a validated performance recommendation or a reboot-persistent setup.

The operator authorized stopping workloads for the test. Stopping the GPU Llama
container reduced reported GPU memory usage from approximately 21973 MiB to
485 MiB. The Linux test VM was shut down gracefully to free host resources.
Private recovery notes retain the exact container identity and VM configuration
backup. Windows subsequently confirmed four CPUs with a one-socket, four-core
topology. KVM visibility was disabled to match Intel's Windows VF example.

## Windows driver status

Windows enumerated the VF initially as Microsoft Basic Display Adapter.
Intel Arc Pro driver 32.0.101.8805 was downloaded from Intel; its SHA-256 matched
`11360f491d21b68a02983bd5209e44f42ddae593fa5efae07fbd04b2d43b86cb`,
and Windows verified its Intel Authenticode signature.

The self-extracting and direct silent installers stalled before driver
installation. Their owned process trees were stopped. Windows PnP then accepted
the signed `iigd_dch_d.inf` package and its dependencies, but device configuration
remained pending after a restart-required result. An orderly guest shutdown
stalled for approximately five minutes, requiring a forced stop of the isolated
fixture. After cold boot, a second PnP installation succeeded. Windows reported
Intel Arc Pro B60, driver 32.0.101.8805, and device error code zero. This remained
true after another orderly shutdown and cold boot.

## Confirmed rendering and display results

The actual game selected the Arc with both D3D12 and Vulkan, and the native-menu
probe passed on both backends. The original VirtIO/VNC display path remained
slow: approximately 7-9 FPS with adaptive V-sync and 10-12 FPS with V-sync off.
The host's `VSyncType` enum uses `Off`, not `Disabled`; the isolated settings were
restored after the initial invalid value was rejected.

Windows GPU counters showed the game's software-adapter engine saturated while
Arc rendering activity was low. DXGI adapter enumeration independently mapped
those counters to Microsoft Basic Render Driver and the Arc. Matching the PF
and VF scheduling values did not resolve the low frame rate.

A Remote Desktop session with hardware graphics policy enabled removed this
bottleneck in the tested configuration. The game continued to select D3D12 on
the Arc and logged a sustained 60 FPS at a 60 FPS cap. RDP was limited by the
test firewall rule to the virtualization host, reached through an SSH tunnel,
and the client pinned the guest certificate fingerprint. Its desktop screenshot
confirmed the visible reward screen.

## Confirmed Astra and replay results

The normal addon, not the menu-probe build, ran real combat through the gateway,
MCP server, harness, and OpenAI Astra bridge configured for `gpt-6-astra`.
With seed `AIASCENSIONREPLAY1`, Astra made 17 decisions and all 17 actions
settled. The combat reached the reward state with 29/80 HP.

A fresh game process replayed those 17 recorded decisions with the same seed
and no model decisions. All actions settled, the reward state again had 29/80
HP, and the terminal player, reward state, seed, and legal-action content
matched. The replay used the accelerated RDP session at 60 FPS. These are real
isolated-combat results, not a full campaign or Linux v3 gameplay pass.

A subsequent fresh Astra-controlled run in the RDP session also completed with
17 model decisions, 17 settled actions, and 29/80 HP at the reward screen, while
the game logged 60 FPS. The Linux test VM was returned to its running state;
the GPU Llama container remained stopped for the Windows GPU allocation.

Private evidence SHA-256 digests:

- Arc D3D12 menu log: `422911dc66d4ff1e7b4cb3e9d622b4f59cb3d9becfd3e19b6b4301fa27f2eef8`.
- Arc Vulkan menu log: `134b4792f94c82d36ea9d746771a01ec019e58966a8a5ae2752e23eaa0f33fe9`.
- Astra trajectory: `7428af0c22e34735f1cdcd8adaf3d74dd4db0e9b9f74d862c9fc842ea3af8e25`.
- Replay trajectory: `b861436bd441d890215008e243827b3757a2b10f9935a3118d9548131b319d68`.
- RDP replay game log: `e261a01d5ef50e077e1d4bcd42d62d6884c7c4ff421f6bc5f618a5bd369129e4`.
- RDP reward screenshot: `fdcdf82c2b889d5d577d4bb504b8dcd6ba9b2c3a40fb45fb4d0fb8a72b31d60b`.
- Fresh RDP Astra trajectory: `77fca659eb81a96b01caad4f46cfcdd60efa3dcf313edd0ae5d811119f1b4cc3`.
- Fresh RDP Astra game log: `d0538851b74ca3db4259701e561efbb81ee5ea31350cbcf2cd809ea7856086cc`.

The setup follows [Intel's SR-IOV toolkit](https://github.com/intel/GFX-SRIOV-Toolkit)
and Microsoft's documented
[RDP hardware rendering policy](https://learn.microsoft.com/en-us/windows/client-management/mdm/policy-csp-admx-terminalserver#ts_dx_use_full_hwgpu).

The VF provisioning is currently runtime-only. The stopped Llama workload needs
its original GPU memory capacity restored before restart. Source CI does not
establish either of these host lifecycle properties.

Hardware configuration, VM disks, downloaded drivers, private profiles, raw
logs, and credentials remain outside this repository.
