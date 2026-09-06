# Train GPU lifecycle native evidence, 2026-09-06

Status: native provisioning, startup admission, actual host reboot, and post-boot Astra passed.
The fixture uses Intel PF `0000:07:00.0`, VF `0000:07:00.1`, xe on kernel
`7.0.0-31-generic`, and libvirt 10.0.0. This is an operator fixture, not an addon feature.

## Confirmed native results

- The canonical debugfs root identifies the exact PF. Fifteen profile fields were read
  before mutation. VRAM and GGTT belong to tile0; scheduling, contexts, and doorbells
  are configured separately for GT0 and GT1.
- Windows completed ACPI shutdown; the agent shutdown attempt had timed out.
  The VF returned from `xe-vfio-pci` to xe before changes. The Linux VM kept running.
- Existing domain XML and the original resource values were preserved privately.
  Matching apply created no rollback file. Removing and recreating the VF passed,
  repeated apply preserved the first record, and restore returned the prior VF count.
- A real contexts quota change from 8192 to 4096 was applied back to 8192 and rolled
  back to 4096. Restoring the original record returned the full original profile.
- Root-owned helpers, configuration, service, and qemu.d gate were installed into
  previously absent destinations. Systemd verification passed. The service orders
  before libvirt-guests and is enabled; Windows autostart stays disabled.
- After a libvirt daemon restart, a real Windows start failed in the prepare hook
  because the VF was absent. An intentionally invalid debug directory failed the
  service and independently blocked VM admission. Restoring configuration allowed
  the service to recreate the VF and the real VM start to pass both admission hooks.

## Scope and preserved artifacts

The initial checks exercised cold VF removal/recreation and the installed service/start gate
within the existing host boot. The two subsequent physical host reboots are recorded below.
The physical function stayed on xe and the conflicting rootful container stayed stopped.
The host boot identifier was `d2e78ce8-feda-4a6f-8de4-eaa42b441116`.

A new qcow2 overlay preserves the pre-test Windows disk. The first overlay start stopped
at UEFI because an inherited empty backingStore element hid the original disk; removing
that stale element exposed the backing chain correctly. Windows then applied its pending
updates. No original disk overwrite or full disk copy occurred.

Private logs and rollback records remain outside Git. SHA-256 evidence:

| Artifact | SHA-256 |
| --- | --- |
| Original resource record | `b742d42d2f830f989017c16ef81596a60b449b7ddd566525d455a603b105a2a2` |
| Cold lifecycle log | `5b5b3adbeaea1b3011870caa2b16e580783ed58fcffe016c7ad8c2cdc373a58b` |
| Quota rollback log | `ea70048d45233cec5a13ab112b19b4920f968c85c661ada80c601503394046b5` |
| Actual admission refusal log | `d9686daa553c577e82117a51d96d926e79864bc2ca9f88c1c5c200a425de79da` |
| Failed service and recovery log | `f98478889ae29dc0c11fc4e10377a4aec508866779abfb701d7e7881379d3fd2` |
| Installed provision helper | `ffe03d7de9206c37806df8903cdc7bf2bab65345a36bd1c3f99e60e38fb3962a` |
| Installed boot helper | `4db61bbe22636501a0bd0fc3732b64a6933c4529a596183eab440f02c71cd049` |
| Installed admission hook | `9fd661324f25284b103ff93989ead35db44959db4a202e8bd0d4336f53599cad` |

Source checks: eleven provisioning boundaries and seven boot/hook tests passed.
Final strict policy passed with 287 sized files, zero warnings, and zero errors.
These source results do not replace native guest driver or gameplay evidence.

## Actual host reboot and workload ordering

Both STS2 guests were confirmed off before each authorized host reboot. The first new
boot, `52039314-1c6b-44b8-b5ee-daec82f3a2a1`, automatically applied all fifteen fields
from zero VFs, zero VF quotas, zero PF scheduling values, and 128 MiB PF spare VRAM.
No manual apply preceded the readback. The GPU unit completed before libvirt-guests.
Later, `podman-migrated-stacks.service` restarted the conflicting model. The read-only
profile check refused that workload; no Windows start was attempted during the conflict.

After stopping that exact model, the operator preserved and changed two startup files:
removed `gemma4-26b-a4b` from its explicit `compose_up` argument in
`/usr/local/sbin/podman-migrated-stacks`, and removed the same service from the proxy's
`depends_on` in `/srv/openclaw/deploy/docker-compose.yml`. The model definition remains.
Parsed configuration comparison confirmed that only that dependency changed; recursive
startup closure decreased from 29 services to 28, losing only the conflicting model.
Shell syntax passed. Private backups preserve original contents, owners, and modes.

| Operational artifact | SHA-256 |
| --- | --- |
| Startup script before | `be058c40a0b7cb1145bddb22dc236cf63153475ab53a0be15cd093fc1636d2d3` |
| Startup script installed | `3e538d284e2fdd2e6ae0df0113a5d7a7bbe3dbb7034ed9bfeece9c701622ca5c` |
| Compose before | `5491c42f4715467cca2bb2d3584b2f3b3e952b2291c06b036791d607ee82f564` |
| Compose installed | `5316b78580950388aa04c4e60189a1bf7cbac06984bc57372ba525282e853169` |
| Dependency comparison | `9a7db6afaf7f5fce3c135fd7d5d4709820094fa1a6998e407526306a09a3dc1b` |

Rollback of this workload hold requires the GPU fixture off and its allocation reconciled
first. Compare installed hashes before restoring the private originals with preserved
ownership/modes; do not overwrite later operator changes or start the model incidentally.
Raw deployment configuration is private and is not included in this repository.

The second new boot, `ca31b6f2-7e0b-4e6e-8273-5a845e11c95f`, again automatically
provisioned and verified the profile. GPU ActiveEnterTimestampMonotonic was 11177226,
before libvirt-guests at 11211531. Migrated-stack startup completed at 278445148 with
the conflicting model still stopped. Read-only profile checks passed after stack startup
and after Windows started. No manual apply or service restart occurred before this proof.
Windows retained Intel driver 32.0.101.8805 with device error 0.

## Post-boot visible gameplay and restoration

A fresh normal Windows addon, including the combat-only settlement regression fix,
completed real `gpt-6-astra` control: twenty model decisions, twenty settled actions,
one combat completion, exit 0, native Reward at 24 HP. A fresh screenshot independently
confirmed Reward; game logs reported 60 FPS. An earlier immediate startup attempt timed
out observing the warming game with zero events/actions and is retained as a failure.
This is an isolated combat run, not another full campaign or campaign win.

| Post-boot artifact | SHA-256 |
| --- | --- |
| Automatic boot journal | `b505a9f0a0cccdccba287734f10e94953b7ef60886569157535439c5b955464c` |
| Boot ordering | `9a9c8830656e45a9f5b1e3e64d448094f12937e983073416c61b564e8478b43d` |
| Boot rollback record | `d6a7fe50f2fc29945c2bd8a85f1faf4ae8263137c622461981839eb3a904276a` |
| Normal Windows addon | `32278a3cc8480f421a446a1cf51ddec6acf33b706b28a8fdc2658e751f216e59` |
| Astra trajectory | `88c085b078718b796691e646b25ca48c1121c018188129b76e64b4f356d36f17` |
| Native Reward capture | `c0b0e0500fdb95da8777088a17c1a0dc3961681498a867f7230a720bbde5a107` |
| Preserved Windows game log | `08c6b6e7f8c0121be56993c026ed0409ca71beb73e3a8b1d9f5b68565929cb4e` |
| Restored Linux native menu capture | `91654a145acaeabc6459d9981f4f66333bb8c401be21734e7dc5057a2be21ff3` |

Both STS2 guests and their read-only viewers were restored. Linux required its existing
Steam client to start before relaunch; a fresh visible v0.107.1 menu confirmed one addon
loaded. The pre-existing service-desk autostart guest was returned to its prior off state.
All 62 prior rootful container names were running. Rootless Redis/Postgres/Kafka services
were restored from exact prior IDs. Clawdle web's missing writable layer required an
exact-image clone preserving application environment, mounts, ports, and network;
the damaged original remains preserved. HTML, script asset, and API health requests passed.

OpenMeter API, balance worker, and billing worker were unhealthy in the pre-reboot
inventory. They now cycle under their existing always-restart policy because ClickHouse
DNS is absent; that dependency was last stopped on September 4 and was left unchanged.
This is a classified existing service fault, not a healthy fleet claim. Three additional
pre-existing containers started automatically across rootful/rootless startup and remain
running. Private inventories and restoration records preserve the exact identities.
