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

bash "$script_dir/install-runtime-addon.sh" install windows-x86_64 \
    "$temp_dir/package-v2" "$install_dir" "$temp_dir/backup-v2" >/dev/null
[[ $(< "$install_dir/AIAscensionSTS2GameMod.dll") == 'managed v2' ]]
[[ $(< "$temp_dir/backup-v2/AIAscensionSTS2GameMod.dll") == 'managed v1' ]]
bash "$script_dir/install-runtime-addon.sh" rollback "$install_dir" "$temp_dir/backup-v2" >/dev/null
[[ $(< "$install_dir/AIAscensionSTS2GameMod.dll") == 'managed v1' ]]
[[ -f "$install_dir/user-notes.txt" ]]

cp -a -- "$temp_dir/package-v1" "$temp_dir/tampered-package"
printf 'tampered\n' >> "$temp_dir/tampered-package/AIAscensionSTS2GameMod.dll"
if bash "$script_dir/install-runtime-addon.sh" install windows-x86_64 \
    "$temp_dir/tampered-package" "$install_dir" "$temp_dir/backup-failed" >/dev/null 2>&1; then
    printf '%s\n' 'tampered package was installed' >&2
    exit 1
fi
[[ $(< "$install_dir/AIAscensionSTS2GameMod.dll") == 'managed v1' ]]
[[ ! -e "$temp_dir/backup-failed" ]]

make_payload "$temp_dir/linux" libAIAscensionSTS2GameModNative.so linux
stage_package linux-x86_64 "$temp_dir/linux" "$temp_dir/linux-package" 2001
linux_install="$temp_dir/linux-install"
bash "$script_dir/install-runtime-addon.sh" install linux-x86_64 \
    "$temp_dir/linux-package" "$linux_install" "$temp_dir/linux-backup" >/dev/null
[[ -f "$linux_install/libAIAscensionSTS2GameModNative.so" ]]
[[ ! -e "$linux_install/AIAscensionSTS2GameModNative.dll" ]]

printf '%s\n' 'Runtime package install, update, checksum refusal, rollback, and Windows/Linux filename tests passed.'
