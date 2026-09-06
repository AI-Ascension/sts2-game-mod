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
   This is not yet a persistent service configuration.
4. Exercise apply/check/restore with real readbacks, including a failed preflight,
   repeated apply, and cold VF removal/recreation. Verify the Windows driver and
   visible game after the guest restarts.
5. Add and verify boot ordering plus a libvirt start gate. The gate must use the
   read-only check under the same lock; it must never call apply or recursively invoke
   virsh from inside a libvirt hook. A failed check must prevent the fixture starting.
   Provisioning must precede fixture startup and must fail closed if the conflicting
   GPU workload starts first. Inspect existing hooks/units before installing anything.
6. Verify actual host reboot or an explicitly equivalent cold lifecycle, workload
   ordering, failed-service behavior, rollback, and subsequent visible Astra control.
   Source checks or a manually reapplied VF do not prove boot persistence.

Host sudo is currently unavailable in the continuation session. No helper, service,
hook, or resource change has been installed. Boot integration remains required work.
