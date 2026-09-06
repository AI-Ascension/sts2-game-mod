# Train GPU fixture lifecycle

Status: **installed; native lifecycle, two host reboots, and post-boot Astra passed on 2026-09-06**.
This directory is an operator fixture,
not part of the addon package. See the [host evidence](../../docs/evidence/train-gpu-lifecycle-20260906.md)
for the distinction between cold VF/service checks and a physical host reboot.

The isolated Windows domain retains a VF in its persistent XML, but xe VF creation and
quotas require reapplication after host reboot. `provision.sh` provides guarded apply,
check, and restore operations for the exact tested Intel PF/VF pair. It preserves
the PF's xe driver, requires the Windows domain shut off, refuses a VF still bound
to VFIO, and checks the conflicting GPU container before changing allocations.
It never stops containers or domains itself.

Apply reads every resource before mutation, saves an exclusive rollback record,
reserves PF resources, configures the VF, enables it last, and reads values back.
An already matching profile is read-only. A partial failure keeps its rollback
record and does not automatically retry. Restore validates the exact record paths,
requires inactive guests, disables the VF, and restores its prior allocations.
Rollback does not restart workloads or change persistent domain XML.

Run the boundary tests from this directory:

```sh
bash test-provision.sh
bash test-boot.sh
```

## Required before deployment

1. With authorized sudo, inspect the exact kernel debugfs directory and both GTs.
   Confirm `name` identifies the PF and validate every quota path, units, tile-wide
   memory ownership, and supported ordering against the running xe driver.
2. Stop the owned Windows fixture gracefully, confirm no domain owns the VF, and
   check the conflicting container's current exact identity/state. Preserve domain
   XML and current values outside Git.
3. Install the reviewed helper root-owned, with an explicit debugfs directory and
   root-owned rollback directory. Use a new rollback record for each boot/application
   cycle; never overwrite a partial-failure record. Invoke with exactly three
   arguments: `bash provision.sh check|apply|restore DEBUG_DIRECTORY RECORD`.
   The boot service and start gate below have separate native verification requirements.
4. Exercise apply/check/restore with real readbacks, including a failed preflight,
   repeated apply, and cold VF removal/recreation. Verify the Windows driver and
   visible game after the guest restarts.
5. Install and verify boot ordering plus the libvirt start gate. The gate uses the
   read-only check under the same lock; it must never call apply or recursively invoke
   virsh from inside a libvirt hook. A failed check must prevent the fixture starting.
   Provisioning must precede fixture startup and must fail closed if the conflicting
   GPU workload starts first. Inspect existing hooks/units before installing anything.
6. Verify actual host reboot or an explicitly equivalent cold lifecycle, workload
   ordering, failed-service behavior, rollback, and subsequent visible Astra control.
   Source checks or a manually reapplied VF do not prove boot persistence.

## Boot and startup configuration

`sts2-gpu-lifecycle.service` runs `boot.sh` before the distribution's `libvirt-guests`
service. It creates a private state directory and uses the kernel boot ID for the
exclusive rollback filename. Repeating a failed application in the same boot cannot
overwrite that record. A successful application is followed by a read-only check.
There is no automatic shutdown rollback or automatic restart after failure.

`qemu-start-gate.sh` checks only the named Windows fixture on prepare/start and
incoming restore/migrate/attach. It emits no XML, performs no libvirt calls, and
never provisions resources. A failed check blocks admission. Reconnect of an already
running guest is excluded because failing that hook can kill the guest. See the
[libvirt hook contract](https://libvirt.org/hooks.html) for hook ordering and failures.
Other domains and release events are unaffected. Both check and matching apply also
reject the identified conflicting GPU workload if it is running or its state is unknown.

After the required real quota tests, install the three scripts root-owned and executable
under `/usr/local/libexec/sts2-gpu-lifecycle/`. Create the root-owned regular file
`/etc/sts2-gpu-lifecycle/debug-directory` containing exactly the reviewed absolute xe
debugfs directory. It is data, not shell configuration. Install the service under
`/etc/systemd/system/` and install the gate as the distinct executable
`/etc/libvirt/hooks/qemu.d/50-sts2-gpu-lifecycle`. Refuse existing destination files
until ownership and backups are reviewed; never replace an existing main qemu hook.
Inspect the installed libvirt version and its hook discovery/reload requirements first.

Keep the fixture's libvirt autostart disabled: daemon autostart can precede the boot
service. Provisioning at boot restores GPU readiness; it does not start the VM itself.
After `systemctl daemon-reload`, start and check the GPU unit while the fixture is off,
then verify that the gate blocks a deliberately nonmatching profile and permits a
matching one. Enable the unit only after those checks. Boot-service ordering alone
is not the startup fence; the hook must be independently exercised.

Rollback requires the fixture off: disable/stop the unit, restore the exact boot's
record using the reviewed helper, and remove only the installed files whose hashes
still match the recorded deployment. Preserve the record and journal. Do not re-enable
the conflicting workload or change domain autostart as an incidental cleanup step.

The authorized native test installed the helpers, service, and distinct qemu.d hook
after confirming no destination existed. Live xe uses `sriov/{pf,vf1}/tile0/` for
VRAM/GGTT and GT subdirectories for scheduling, contexts, and doorbells. The profile
uses these canonical tile-level controls rather than legacy per-GT memory aliases.
The ownership guard also rejects the host's `xe-vfio-pci` driver.

The native test verified quota readbacks, VF removal/recreation, idempotence, rollback,
actual libvirt admission refusal, service failure, and recovery. The service is enabled;
Windows autostart remains disabled. After the second actual host reboot, automatic
provisioning passed before libvirt-guests, and visible Windows Astra completed twenty
settled actions to Reward at 24 HP with 60 FPS reported by the game.

The host's migrated-stack startup script also explicitly started the conflicting model,
despite its separate compose unit being disabled. The authorized deployment removed only
that model's startup argument and its proxy dependency; the model definition remains.
Review the evidence record for exact configuration hashes and rollback requirements.
Disabling a compose unit alone does not establish this workload hold.
