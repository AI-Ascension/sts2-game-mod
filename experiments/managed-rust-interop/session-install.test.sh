#!/usr/bin/env bash
set -Eeuo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
source "$script_dir/session-launcher.sh"
fixture_root=$(mktemp -d)
trap 'rm -rf -- "$fixture_root"' EXIT
session_launcher_repo_root=$fixture_root
session_launcher_package_script="$script_dir/session-package-fixture.sh"
tasklist_cmd="$script_dir/session-launcher-fixture.sh"
live_authorization_deadline=$((EPOCHSECONDS + 60))
mkdir -p "$fixture_root/host/mods"
install_addon "$fixture_root/host/data" "$fixture_root/host" "$fixture_root/host/mods"
for artifact in AIAscensionSTS2GameMod.dll AIAscensionSTS2GameModNative.dll AIAscensionSTS2GameMod.json; do
    cmp "$fixture_root/.sts2-dev/runtime-session-addon/$artifact" "$fixture_root/host/mods/$artifact"
done
# An inspection failure is not evidence of a stopped host.
tasklist_cmd=/bin/false
if (install_addon "$fixture_root/host/data" "$fixture_root/host" "$fixture_root/host/mods") >/dev/null 2>&1; then
    printf 'failed inspection admitted installation\n' >&2; exit 1
fi
tasklist_cmd="$script_dir/session-launcher-fixture.sh"
mv "$fixture_root/host/mods/AIAscensionSTS2GameMod.dll" "$fixture_root/untouched"
ln -s "$fixture_root/untouched" "$fixture_root/host/mods/AIAscensionSTS2GameMod.dll"
if (install_addon "$fixture_root/host/data" "$fixture_root/host" "$fixture_root/host/mods") >/dev/null 2>&1; then
    printf 'symbolic link admitted installation\n' >&2; exit 1
fi
cmp "$fixture_root/.sts2-dev/runtime-session-addon/AIAscensionSTS2GameMod.dll" "$fixture_root/untouched"
live_authorization_deadline=$EPOCHSECONDS
if (assert_live_authorization_current) >/dev/null 2>&1; then
    printf 'expired authorization admitted work\n' >&2; exit 1
fi
printf 'Synthetic package names, inspection failure, symlink refusal, expiry=TRUE\n'
