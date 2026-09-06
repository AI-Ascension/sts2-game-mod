# Train GPU fixture lifecycle candidate

Status: **undeployed and not host-verified**. This directory is an operator fixture,
not part of the addon package. Source tests cover refusal and write ordering only.
They do not establish that xe accepts the proposed resource paths/order.

The isolated Windows domain retains a VF in its persistent XML, but VF creation and
quotas currently disappear at host reboot. `provision.sh` prepares guarded apply,
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
   The boot service and start gate below are source candidates, not installed configuration.
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

## Boot and startup candidates

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

On the read-only host inspection, the hooks directory existed but the main qemu hook,
qemu.d directory, and STS2 service did not. Sudo remained unavailable. No helper,
service, hook, or resource change has been installed. The source tests cover failure
propagation, boot-record identity, and hook scope; real deployment, hook rejection,
driver acceptance, reboot ordering, and post-reboot visible gameplay remain unverified.
