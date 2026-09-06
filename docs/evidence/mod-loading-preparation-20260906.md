# Intended-addon loading preparation, 2026-09-06

## Confirmed native fixture checks

The authorized isolated Windows and Linux v0.107.1 fixtures each had their selected game stopped
and their existing profile, addon, launcher, and logs backed up. The Windows snapshot contained
226 files; the Linux snapshot contained 128 files. Backup paths and profile contents remain private.

Each test explicitly selected the fixture's existing settings file with the intended addon entry,
cleared only `mod_settings` to reproduce the missing-consent form, and invoked the Rust operator
tool. Read-only plan reported a needed change. Apply verified the input digest, created an exact
settings backup, enabled `AIAscensionSTS2GameMod`, replaced the settings, and verified the new
bytes. A subsequent plan reported unchanged. The Linux check also compared ownership and mode
before and after preparation.

| Evidence | Windows | Linux |
| --- | --- | --- |
| Tested operator binary SHA-256 | `2308236b45953d2f96fd50857408cb1185a279f5349a9bb4cd19df9418fc8b8a` | `a606d5f68634300f9c415b6e4abf76677bebcbaa23874c0243854e3a5208fe80` |
| Cleared-consent input SHA-256 | `0b96d6c109181c083b7581c597135ee3c4a77347b625039e5ea8fdf59fe7c44e` | `8bfb46a4a66fc884248a10d2587d9dd8fb92c31c7a659e7fa2b84d507be7dbd7` |
| Prepared settings SHA-256 | `201ae682b7817a33c7f9976a25fbc0e40708954fb5e7cfa3e5ae3ee3b7b0514e` | `b895402c905fce38751628a7c7052ebd7d633d2a176bdfb3f127a9baa6df816f` |
| Visible native menu capture SHA-256 | `8105450904dbe392e5f59eb0d763aaf22e7ed79e0d2861c347f05d370aa999f7` | `a9f3cf7daf57493759202306708cd4f02537b9e20557582e446c56168465743c` |

Fresh native launches reached the game menu showing exactly one loaded mod. Windows used the
connected RDP desktop; Linux used its read-only video stream. Windows was relaunched after the
old RDP client was found disconnected, so the visible capture belongs to the subsequent fresh
launch with the same prepared consent. No OS keyboard or mouse automation was used.

## Limits

This reproduces the known cleared-consent settings form; it is not a newly downloaded Steam
patch test. Future host settings formats fail closed and need separate compatibility work.
Workshop subscriptions and mixed-mod installations are not covered. The tool does not discover
an active profile, grant consent for other addons, or bypass host version compatibility checks.
The native checks used guest-agent launch owners; the Windows/WSL `dev-cycle.sh` integration has
source and synthetic process-guard coverage, not a native run of that wrapper inside these VMs.

The tests concern loader preparation, not campaign completion, gameplay replay, or current
video frame-rate guarantees. Those have separate evidence. The Windows binary predates a Unix-only
ownership-preservation addition, which was included in the Linux test and does not alter the Windows
preparation path. The binary hashes above identify the actual native test artifacts.
