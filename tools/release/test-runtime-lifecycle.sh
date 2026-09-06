#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -Eeuo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
repo_root=$(cd "$script_dir/../.." && pwd -P)
temp_dir=$(mktemp -d -t sts2-runtime-lifecycle-XXXXXXXX)
trap 'rm -rf -- "$temp_dir"' EXIT

make_payload() {
    local root=$1 native_name=$2 version=$3
    mkdir -p -- "$root"
    printf 'managed %s\n' "$version" > "$root/AIAscensionSTS2GameMod.dll"
    printf '{"id":"synthetic-%s"}\n' "$version" > "$root/AIAscensionSTS2GameMod.json"
    printf 'native %s\n' "$version" > "$root/$native_name"
}

stage_package() {
    local platform=$1 payload=$2 output=$3 item_id=$4
    bash "$repo_root/tools/workshop/package-platform-item.sh" \
        "$platform" "$payload" "$output" 480 "$item_id" 0.107.1 "$item_id" "commit-$item_id" \
        "$temp_dir/preview.jpg" >/dev/null
}

printf 'synthetic preview\n' > "$temp_dir/preview.jpg"
make_payload "$temp_dir/windows-v1" AIAscensionSTS2GameModNative.dll v1
make_payload "$temp_dir/windows-v2" AIAscensionSTS2GameModNative.dll v2
stage_package windows-x86_64 "$temp_dir/windows-v1" "$temp_dir/package-v1" 1001
stage_package windows-x86_64 "$temp_dir/windows-v2" "$temp_dir/package-v2" 1002

install_dir="$temp_dir/windows-install"
mkdir -p -- "$install_dir"
printf 'operator-owned file\n' > "$install_dir/user-notes.txt"
bash "$script_dir/install-runtime-addon.sh" install windows-x86_64 \
    "$temp_dir/package-v1" "$install_dir" "$temp_dir/backup-v1" >/dev/null
[[ $(< "$install_dir/AIAscensionSTS2GameMod.dll") == 'managed v1' ]]
[[ -f "$install_dir/user-notes.txt" ]]

ln -s -- "$temp_dir/package-v2" "$temp_dir/package-link"
if bash "$script_dir/install-runtime-addon.sh" install windows-x86_64 \
    "$temp_dir/package-link" "$install_dir" "$temp_dir/backup-package-link" >/dev/null 2>&1; then
    printf '%s\n' 'symlinked package root was accepted' >&2
    exit 1
fi
[[ ! -e "$temp_dir/backup-package-link" ]]
rm -- "$temp_dir/package-link"

ln -s -- "$install_dir" "$temp_dir/install-link"
if bash "$script_dir/install-runtime-addon.sh" install windows-x86_64 \
    "$temp_dir/package-v2" "$temp_dir/install-link" "$temp_dir/backup-install-link" >/dev/null 2>&1; then
    printf '%s\n' 'symlinked install root was accepted' >&2
    exit 1
fi
[[ ! -e "$temp_dir/backup-install-link" ]]
rm -- "$temp_dir/install-link"

bash "$script_dir/install-runtime-addon.sh" install windows-x86_64 \
    "$temp_dir/package-v2" "$install_dir" "$temp_dir/backup-v2" >/dev/null
[[ $(< "$install_dir/AIAscensionSTS2GameMod.dll") == 'managed v2' ]]
[[ $(< "$temp_dir/backup-v2/AIAscensionSTS2GameMod.dll") == 'managed v1' ]]
bash "$script_dir/install-runtime-addon.sh" rollback "$install_dir" "$temp_dir/backup-v2" >/dev/null
[[ $(< "$install_dir/AIAscensionSTS2GameMod.dll") == 'managed v1' ]]
[[ -f "$install_dir/user-notes.txt" ]]

mv -- "$install_dir/AIAscensionSTS2GameMod.dll" "$temp_dir/rollback-target-original"
mkdir -- "$install_dir/AIAscensionSTS2GameMod.dll"
if bash "$script_dir/install-runtime-addon.sh" rollback "$install_dir" "$temp_dir/backup-v2" >/dev/null 2>&1; then
    printf '%s\n' 'rollback accepted a non-regular target' >&2
    exit 1
fi
[[ -d "$install_dir/AIAscensionSTS2GameMod.dll" ]]
[[ -z "$(find "$install_dir/AIAscensionSTS2GameMod.dll" -mindepth 1 -maxdepth 1 -print -quit)" ]]
rmdir -- "$install_dir/AIAscensionSTS2GameMod.dll"
mv -- "$temp_dir/rollback-target-original" "$install_dir/AIAscensionSTS2GameMod.dll"

cp -a -- "$temp_dir/package-v1" "$temp_dir/tampered-package"
printf 'tampered\n' >> "$temp_dir/tampered-package/AIAscensionSTS2GameMod.dll"
if bash "$script_dir/install-runtime-addon.sh" install windows-x86_64 \
    "$temp_dir/tampered-package" "$install_dir" "$temp_dir/backup-failed" >/dev/null 2>&1; then
    printf '%s\n' 'tampered package was installed' >&2
    exit 1
fi
[[ $(< "$install_dir/AIAscensionSTS2GameMod.dll") == 'managed v1' ]]
[[ ! -e "$temp_dir/backup-failed" ]]

cp -a -- "$temp_dir/package-v1" "$temp_dir/missing-checksum-package"
sed -i '$d' "$temp_dir/missing-checksum-package/SHA256SUMS"
if bash "$script_dir/install-runtime-addon.sh" install windows-x86_64 \
    "$temp_dir/missing-checksum-package" "$install_dir" "$temp_dir/backup-missing-checksum" >/dev/null 2>&1; then
    printf '%s\n' 'incomplete checksum inventory was installed' >&2
    exit 1
fi
[[ $(< "$install_dir/AIAscensionSTS2GameMod.dll") == 'managed v1' ]]
[[ ! -e "$temp_dir/backup-missing-checksum" ]]

cp -a -- "$temp_dir/package-v1" "$temp_dir/mismatched-manifest-package"
sed -i '0,/"size_bytes": [0-9]*/s//"size_bytes": 1/' \
    "$temp_dir/mismatched-manifest-package/sts2-workshop-manifest.json"
manifest_digest=$(sha256sum "$temp_dir/mismatched-manifest-package/sts2-workshop-manifest.json" | awk '{print $1}')
sed -i "s/^[0-9a-f]*  sts2-workshop-manifest.json$/$manifest_digest  sts2-workshop-manifest.json/" \
    "$temp_dir/mismatched-manifest-package/SHA256SUMS"
if bash "$script_dir/install-runtime-addon.sh" install windows-x86_64 \
    "$temp_dir/mismatched-manifest-package" "$install_dir" "$temp_dir/backup-mismatched-manifest" >/dev/null 2>&1; then
    printf '%s\n' 'manifest size mismatch was installed' >&2
    exit 1
fi
[[ $(< "$install_dir/AIAscensionSTS2GameMod.dll") == 'managed v1' ]]
[[ ! -e "$temp_dir/backup-mismatched-manifest" ]]

cp -a -- "$temp_dir/package-v1" "$temp_dir/hidden-entry-package"
printf 'unexpected\n' > "$temp_dir/hidden-entry-package/.unexpected"
if bash "$script_dir/install-runtime-addon.sh" install windows-x86_64 \
    "$temp_dir/hidden-entry-package" "$install_dir" "$temp_dir/backup-hidden-entry" >/dev/null 2>&1; then
    printf '%s\n' 'hidden package entry was installed' >&2
    exit 1
fi
[[ $(< "$install_dir/AIAscensionSTS2GameMod.dll") == 'managed v1' ]]
[[ ! -e "$temp_dir/backup-hidden-entry" ]]

cp -a -- "$temp_dir/backup-v2" "$temp_dir/malformed-backup"
printf 'malformed\n' > "$temp_dir/malformed-backup/AIAscensionSTS2GameMod.dll.state"
if bash "$script_dir/install-runtime-addon.sh" rollback "$install_dir" "$temp_dir/malformed-backup" >/dev/null 2>&1; then
    printf '%s\n' 'malformed rollback state was accepted' >&2
    exit 1
fi
[[ $(< "$install_dir/AIAscensionSTS2GameMod.dll") == 'managed v1' ]]

cp -a -- "$temp_dir/backup-v2" "$temp_dir/missing-state-backup"
rm "$temp_dir/missing-state-backup/AIAscensionSTS2GameMod.dll.state"
if bash "$script_dir/install-runtime-addon.sh" rollback "$install_dir" "$temp_dir/missing-state-backup" >/dev/null 2>&1; then
    printf '%s\n' 'incomplete rollback state was accepted' >&2
    exit 1
fi
[[ $(< "$install_dir/AIAscensionSTS2GameMod.dll") == 'managed v1' ]]

make_payload "$temp_dir/linux" libAIAscensionSTS2GameModNative.so linux
stage_package linux-x86_64 "$temp_dir/linux" "$temp_dir/linux-package" 2001
linux_install="$temp_dir/linux-install"
bash "$script_dir/install-runtime-addon.sh" install linux-x86_64 \
    "$temp_dir/linux-package" "$linux_install" "$temp_dir/linux-backup" >/dev/null
[[ -f "$linux_install/libAIAscensionSTS2GameModNative.so" ]]
[[ ! -e "$linux_install/AIAscensionSTS2GameModNative.dll" ]]

printf '%s\n' 'Runtime package install, update, checksum refusal, rollback, and Windows/Linux filename tests passed.'
