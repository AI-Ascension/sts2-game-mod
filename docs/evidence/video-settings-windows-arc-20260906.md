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
backup. The Windows domain was configured for four CPUs on its next cold boot.

## Windows driver status

Windows enumerated the VF initially as Microsoft Basic Display Adapter.
Intel Arc Pro driver 32.0.101.8805 was downloaded from Intel; its SHA-256 matched
`11360f491d21b68a02983bd5209e44f42ddae593fa5efae07fbd04b2d43b86cb`,
and Windows verified its Intel Authenticode signature.

The self-extracting and direct silent installers stalled before driver
installation. Their owned process trees were stopped. Windows PnP then accepted
the signed `iigd_dch_d.inf` package and its dependencies, but device configuration
remained pending after a restart-required result. An orderly guest shutdown was
requested. This is not yet a successful driver-start or accelerated game result.

## Remaining verification

Confirm the driver starts without a device error after boot, then verify the
actual game renderer and measured FPS. Run the native-menu probe and real Astra
control separately; neither is implied by VF enumeration or package staging.

Hardware configuration, VM disks, downloaded drivers, private profiles, raw
logs, and credentials remain outside this repository.
