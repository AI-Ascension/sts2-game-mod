#!/usr/bin/env bash
set -Eeuo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
fixture_root=$(mktemp -d)
trap 'rm -rf -- "$fixture_root"' EXIT
cycle_dir="$fixture_root/repo/experiments/managed-rust-interop"
mkdir -p "$cycle_dir" "$fixture_root/bin" "$fixture_root/host/data_sts2_windows_x86_64" "$fixture_root/host/mods"
cp "$script_dir/dev-cycle.sh" "$script_dir/live-authorization.sh" "$script_dir/dev-cycle-process.ps1" "$cycle_dir/"
cp "$script_dir/session-package-fixture.sh" "$cycle_dir/package-runtime-addon.sh"
for tool in powershell.exe wslpath date; do
    cp "$script_dir/dev-cycle-fixture.sh" "$fixture_root/bin/$tool"
    chmod u+x "$fixture_root/bin/$tool"
done
touch "$fixture_root/host/SlayTheSpire2.exe" "$fixture_root/host/data_sts2_windows_x86_64/sts2.dll" \
    "$fixture_root/host/data_sts2_windows_x86_64/GodotSharp.dll"
export PATH="$fixture_root/bin:$PATH"
export STS2_DEV_CYCLE_TEST_LOG="$fixture_root/process.log"
export STS2_LIVE_AUTHORIZATION_APPROVED=yes \
    STS2_LIVE_AUTHORIZATION_SCOPE='runtime-v2 live disposable trace' \
    STS2_LIVE_AUTHORIZATION_HOST_IDENTITY='synthetic-host' \
    STS2_LIVE_AUTHORIZATION_HOST_INSTALL_LABEL='synthetic-install' \
    STS2_LIVE_AUTHORIZATION_PROFILE_IDENTITY='disposable-profile-2' \
    STS2_LIVE_AUTHORIZATION_PROCESS_ACTIONS='install launch stop terminate' \
    STS2_LIVE_AUTHORIZATION_PROFILE_MUTATIONS='mutate disposable selected profile only' \
    STS2_LIVE_AUTHORIZATION_LISTENER_ACTIONS='bind loopback connect loopback' \
    STS2_LIVE_AUTHORIZATION_NETWORK_ACTIONS='loopback only' \
    STS2_LIVE_AUTHORIZATION_CLEANUP_OWNER='synthetic-test' \
    STS2_LIVE_AUTHORIZATION_RESTORE_POINT='synthetic-backup' \
    STS2_LIVE_AUTHORIZATION_EXPIRY_EPOCH=$((EPOCHSECONDS + 300)) \
    STS2_LIVE_AUTHORIZATION_PUBLICATION_AUTHORITY='none' \
    STS2_LIVE_AUTHORIZATION_PROVIDER_CALLS=prohibited
cycle() { bash "$cycle_dir/dev-cycle.sh" --game-dir "$fixture_root/host" --no-launch "$@"; }
fail() { printf 'dev-cycle test failed: %s\n' "$1" >&2; exit 1; }
artifact=AIAscensionSTS2GameMod.dll
printf 'original\n' > "$fixture_root/host/mods/$artifact"
cp "$fixture_root/host/mods/$artifact" "$fixture_root/expected"
if cycle --stage-dir "$fixture_root/host/mods" >/dev/null 2>&1; then fail 'stage aliases installation'; fi
cmp "$fixture_root/expected" "$fixture_root/host/mods/$artifact"
if STS2_RUNTIME_ADDON_BACKUP_DIR="$fixture_root/host/backups" cycle >/dev/null 2>&1; then fail 'backup overlaps installation'; fi
if STS2_RUNTIME_ADDON_BACKUP_DIR="$fixture_root/repo/.sts2-dev/runtime-addon/backup" cycle >/dev/null 2>&1; then fail 'backup overlaps stage'; fi
if STS2_DEV_CYCLE_TEST_RUNNING=yes cycle --no-kill >/dev/null 2>&1; then fail 'running installation accepted'; fi
cmp "$fixture_root/expected" "$fixture_root/host/mods/$artifact"
if STS2_DEV_CYCLE_TEST_INSPECTION_FAIL=yes cycle --no-kill >/dev/null 2>&1; then fail 'failed inspection accepted'; fi
cmp "$fixture_root/expected" "$fixture_root/host/mods/$artifact"
cycle --no-kill >/dev/null
cycle --no-kill >/dev/null
backups=("$fixture_root/repo/.sts2-dev/backups/"*)
[[ ${#backups[@]} == 2 && ${backups[0]} != "${backups[1]}" ]] || fail 'same-second backup collision'
cmp -s "$fixture_root/expected" "${backups[0]}/$artifact" \
    || cmp "$fixture_root/expected" "${backups[1]}/$artifact"
mv "$fixture_root/host/mods/$artifact" "$fixture_root/untouched"
ln -s "$fixture_root/untouched" "$fixture_root/host/mods/$artifact"
if cycle --no-kill >/dev/null 2>&1; then fail 'installed symlink accepted'; fi
cmp "$fixture_root/repo/.sts2-dev/runtime-addon/$artifact" "$fixture_root/untouched"
rm "$fixture_root/host/mods/$artifact"
ln "$fixture_root/untouched" "$fixture_root/host/mods/$artifact"
printf 'outside hardlink\n' > "$fixture_root/untouched"
cycle >/dev/null
[[ $(<"$fixture_root/untouched") == 'outside hardlink' ]] || fail 'copy modified unrelated hardlink'
[[ $(<"$STS2_DEV_CYCLE_TEST_LOG") == *Stop* ]] || fail 'selected stop guard not invoked'
printf '%s\n' 'PASS: no-kill refusal, inspection errors, unique backups, symlink refusal, hardlink preservation'
