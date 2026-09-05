# Local launcher safety boundary

The session launcher and development installer require explicit authorization before live work.
This is a source-level guard, not permission supplied by this document. Use only a separately
authorized disposable host/profile. No game, provider, installation, or save mutation is part of
ordinary repository validation. These controls do not establish gameplay or host compatibility.

## Authorization record

Set every field below only after an operator has approved the actual scope. Replace all angle-bracket
values; they are descriptive attestations, not discovered identities. The expiry is an absolute Unix
epoch in seconds, chosen by that operator, not an automatically renewable session allowance.

~~~bash
export STS2_LIVE_AUTHORIZATION_APPROVED=yes
export STS2_LIVE_AUTHORIZATION_SCOPE='live runtime-v2'
export STS2_LIVE_AUTHORIZATION_HOST_IDENTITY='<exact approved host/build>'
export STS2_LIVE_AUTHORIZATION_HOST_INSTALL_LABEL='<approved installation label>'
export STS2_LIVE_AUTHORIZATION_PROFILE_IDENTITY='<selected disposable profile>'
export STS2_LIVE_AUTHORIZATION_PROCESS_ACTIONS='install launch stop terminate'
export STS2_LIVE_AUTHORIZATION_PROFILE_MUTATIONS='mutate disposable selected profile only'
export STS2_LIVE_AUTHORIZATION_LISTENER_ACTIONS='bind loopback; connect loopback'
export STS2_LIVE_AUTHORIZATION_NETWORK_ACTIONS='loopback only'
export STS2_LIVE_AUTHORIZATION_CLEANUP_OWNER='<responsible operator>'
export STS2_LIVE_AUTHORIZATION_RESTORE_POINT='<verified external restore point>'
export STS2_LIVE_AUTHORIZATION_EXPIRY_EPOCH='<approved future epoch seconds>'
export STS2_LIVE_AUTHORIZATION_PUBLICATION_AUTHORITY='none'
export STS2_LIVE_AUTHORIZATION_PROVIDER_CALLS=prohibited
~~~

Check record syntax without host access:

~~~text
bash experiments/managed-rust-interop/session-launcher.sh --authorization-check
~~~

Validation checks required fields, bounds, permission markers, exact network/provider scope, and expiry. It does not authenticate
the operator, inspect a restore point, select a disposable profile, verify the attested host, or
enforce a provider/network sandbox. Ensure the correct profile is selected before authorizing a
launch; startup itself may write profile data. The launcher removes this metadata from subsequent
child environments while retaining a non-exported local deadline. It rechecks admission after
builds and during supervision; bounded commands use the remaining authorization time. Expiry
stops admission and initiates owned cleanup; it is not evidence that shutdown or restoration succeeded.
Provider model calls remain prohibited; starting local gateway/harness/MCP components is not
authorization to invoke an external or paid model service.

## Processes and input

Owned game launches request `--headless --audio-driver Dummy` and do not drive mouse, keyboard,
window focus, or desktop input. Static source checks detect known input-automation APIs; neither
those checks nor headless rendering is an OS sandbox or a guarantee about external binaries.

The session launcher refuses an existing game and starts providers in recorded POSIX sessions/groups.
Windows cleanup checks the launched process's PID, creation time, and executable identity through
the bridge. It does not adopt unrelated processes. The development cycle separately inspects the
selected installation's executable and stops only matching inspected processes; `--no-kill` requires
that installation to be stopped. Neither path uses image-wide termination as proof of ownership.
Failed process inspection or uncertain termination is a failure, not an idle-host observation.

## Installation and restoration

Installation replaces only its named addon artifacts and intentionally retired legacy addon names.
Unrelated mods remain untouched. Session path checks reject symbolic-link ancestry; the development
cycle additionally checks Windows reparse points and overlapping staging/backup/install locations.
Use trusted paths without concurrent modification; unique backups preserve replaced files. This is selective addon
recovery, not a backup or rollback of game saves, profiles, settings, or provider state.

Session cleanup restores the previous addon files only after confirmed game termination and a
successful check that no game remains. It validates backup completeness and paths before writing,
then checks restored contents. If stop, inspection, backup, path, or restore validation fails, cleanup
reports failure and retains the backup for operator recovery; do not erase it or claim restoration.
Session backups are under the ignored `.sts2-dev/session-backups/`; development-cycle backups use
their configured backup directory. The development cycle intentionally leaves a successful install
in place. Abrupt machine failure is not covered by a transactional crash-recovery guarantee.

The persistent Windows Job guardian and its bounded launch receipt are inherited from reviewed
main. Guardian cancellation/stop failure also makes game termination unconfirmed, even if a
subsequent process lookup is empty; restoration is deferred and backups retained. Eight synthetic
restoration cases include this guardian-failure boundary. Main's Cargo-reported executable paths
and bridge-handle lifetime protections remain unchanged by the candidate's launcher additions.

## Credentials and diagnostic output

Fresh runtime/mod and gateway credentials are passed through child environments or bridge stdin,
not launcher arguments or deliberate credential files. Normal provider output is discarded. External
components can still expose inherited secrets; local process environments are not secret isolation.

The launcher discards gateway and harness stdout/stderr, including diagnostic output. It does not
offer a raw log sink: inherited credentials, private paths, and observations could otherwise be
written without redaction or a byte limit. This preserves ADR 0013's credential and output boundary;
it does not prevent an external component from writing its own files.

See [TESTING.md](TESTING.md#ephemeral-session-launcher) for isolated negative tests. Source and
synthetic results remain distinct from separately authorized exact-host evidence.
