#!/usr/bin/env bash

set -euo pipefail

readonly SCRIPT_NAME=$(basename "$0")
readonly PACKAGE_ID='ai-ascension.sts2-game-mod'
readonly SCHEMA_VERSION='sts2-workshop-manifest-v1'
readonly LOADER_CONTRACT='sts2-managed-loader-v1'
readonly ENTRYPOINT='AIAscensionSTS2GameMod.json'
readonly MANIFEST_NAME='sts2-workshop-manifest.json'
readonly CHECKSUM_NAME='SHA256SUMS'
readonly TITLE='AI-Ascension STS2 Game Mod'
readonly DESCRIPTION='First-party AI-Ascension STS2 game-process adapter package.'
readonly WORKSHOP_VISIBILITY='0'

die() {
    printf '%s: %s\n' "$SCRIPT_NAME" "$1" >&2
    exit 1
}

usage() {
    printf 'usage: %s <payload-dir> <output-dir> <consumer-app-id> <published-file-id> <game-version> <package-version> <source-revision> <preview-file>\n' "$SCRIPT_NAME" >&2
    printf 'published-file-id may be 0 for a new Workshop item; rebuild with the assigned ID before release.\n' >&2
    exit 2
}

[[ $# -eq 8 ]] || usage
export LC_ALL=C

payload_dir=$(realpath -e "$1") || die "payload directory does not exist: $1"
output_dir=$(realpath -m "$2") || die "could not resolve output directory: $2"
consumer_app_id=$3
published_file_id=$4
game_version=$5
package_version=$6
source_revision=$7
[[ ! -L "$1" && ! -L "$8" ]] || die 'payload root and preview must not be symlinks'
preview_file=$(realpath -e "$8") || die "preview file does not exist: $8"
[[ -f "$preview_file" && ! -L "$preview_file" ]] || die 'preview file must be a regular non-symlink file'

bounded_decimal() {
    local value=$1 maximum=$2
    [[ "$value" =~ ^(0|[1-9][0-9]*)$ ]] || return 1
    [[ ${#value} -lt ${#maximum} || (${#value} -eq ${#maximum} && ! "$value" > "$maximum") ]]
}
bounded_decimal "$consumer_app_id" 4294967295 && [[ "$consumer_app_id" != 0 ]] || die 'consumer app ID must be a positive uint32 decimal'
bounded_decimal "$published_file_id" 18446744073709551615 || die 'published file ID must be a uint64 decimal without leading zeros'
[[ "$game_version" =~ ^[A-Za-z0-9._-]+$ ]] || die 'game version contains unsupported characters'
[[ "$package_version" =~ ^[A-Za-z0-9._-]+$ ]] || die 'package version contains unsupported characters'
[[ "$source_revision" =~ ^[A-Za-z0-9._/-]+$ ]] || die 'source revision contains unsupported characters'
[[ "$source_revision" != *..* ]] || die 'source revision may not contain parent traversal'
for token in "$game_version" "$package_version" "$source_revision"; do
    [[ ${#token} -le 256 && "$token" != *..* ]] || die 'metadata token exceeds contract bounds'
done

case "$output_dir/" in
    "$payload_dir/"*) die 'output directory must not be inside the payload directory' ;;
esac
[[ -e "$output_dir" ]] && die "output directory already exists: $output_dir"
vdf_path="${output_dir}.vdf"
[[ ! -e "$vdf_path" && ! -L "$vdf_path" ]] || die "VDF output already exists: $vdf_path"

readonly payload_files=(
    'AIAscensionSTS2GameMod.dll'
    'AIAscensionSTS2GameMod.json'
    'AIAscensionSTS2GameModNative.dll'
)

is_allowed_payload() {
    local candidate=$1
    local allowed
    for allowed in "${payload_files[@]}"; do
        [[ "$candidate" == "$allowed" ]] && return 0
    done
    return 1
}

while IFS= read -r -d '' path; do
    name=$(basename "$path")
    [[ -f "$path" && ! -L "$path" ]] || die "payload entry is not a regular file: $name"
    is_allowed_payload "$name" || die "unexpected payload entry: $name"
done < <(find "$payload_dir" -mindepth 1 -maxdepth 1 -print0)

for name in "${payload_files[@]}"; do
    source_file="$payload_dir/$name"
    [[ -f "$source_file" && ! -L "$source_file" ]] || die "required payload file is missing: $name"
    [[ -s "$source_file" ]] || die "required payload file is empty: $name"
    [[ $(wc -c < "$source_file") -le 268435456 ]] || die "payload file exceeds its byte bound: $name"
done

mkdir -p "$output_dir"
for name in "${payload_files[@]}"; do
    source_file="$payload_dir/$name"
    cp -- "$source_file" "$output_dir/$name"
done

canonical_file=$(mktemp)
cleanup() {
    rm -f -- "$canonical_file"
}
trap cleanup EXIT

file_size() {
    wc -c < "$1" | tr -d '[:space:]'
}

file_digest() {
    sha256sum "$1" | awk '{print $1}'
}

canonical_record() {
    local name=$1
    local path="$output_dir/$name"
    printf '%s\t%s\t%s\n' "$name" "$(file_size "$path")" "$(file_digest "$path")" >> "$canonical_file"
}

canonical_record 'AIAscensionSTS2GameMod.dll'
canonical_record 'AIAscensionSTS2GameMod.json'
canonical_record 'AIAscensionSTS2GameModNative.dll'
content_digest=$(sha256sum "$canonical_file" | awk '{print $1}')

manifest_path="$output_dir/$MANIFEST_NAME"
{
    printf '{\n'
    printf '  "schema_version": "%s",\n' "$SCHEMA_VERSION"
    printf '  "package_id": "%s",\n' "$PACKAGE_ID"
    printf '  "package_version": "%s",\n' "$package_version"
    printf '  "consumer_app_id": %s,\n' "$consumer_app_id"
    printf '  "published_file_id": %s,\n' "$published_file_id"
    printf '  "game_version": "%s",\n' "$game_version"
    printf '  "platform": "windows-x86_64",\n'
    printf '  "loader_contract": "%s",\n' "$LOADER_CONTRACT"
    printf '  "content_kind": "first_party_executable",\n'
    printf '  "entrypoint": "%s",\n' "$ENTRYPOINT"
    printf '  "files": [\n'
    printf '    {"path": "AIAscensionSTS2GameMod.dll", "role": "managed_assembly", "size_bytes": %s, "sha256": "%s"},\n' "$(file_size "$output_dir/AIAscensionSTS2GameMod.dll")" "$(file_digest "$output_dir/AIAscensionSTS2GameMod.dll")"
    printf '    {"path": "AIAscensionSTS2GameMod.json", "role": "loader_manifest", "size_bytes": %s, "sha256": "%s"},\n' "$(file_size "$output_dir/AIAscensionSTS2GameMod.json")" "$(file_digest "$output_dir/AIAscensionSTS2GameMod.json")"
    printf '    {"path": "AIAscensionSTS2GameModNative.dll", "role": "native_library", "size_bytes": %s, "sha256": "%s"}\n' "$(file_size "$output_dir/AIAscensionSTS2GameModNative.dll")" "$(file_digest "$output_dir/AIAscensionSTS2GameModNative.dll")"
    printf '  ],\n'
    printf '  "content_digest": "%s",\n' "$content_digest"
    printf '  "source_revision": "%s"\n' "$source_revision"
    printf '}\n'
} > "$manifest_path"

checksum_path="$output_dir/$CHECKSUM_NAME"
{
    for name in "${payload_files[@]}" "$MANIFEST_NAME"; do
        printf '%s  %s\n' "$(file_digest "$output_dir/$name")" "$name"
    done
} > "$checksum_path"

vdf_escape() {
    local value=$1
    value=${value//\\/\\\\}
    value=${value//\"/\\\"}
    printf '%s' "$value"
}

{
    printf '"workshopitem"\n{\n'
    printf '  "appid" "%s"\n' "$consumer_app_id"
    printf '  "publishedfileid" "%s"\n' "$published_file_id"
    printf '  "contentfolder" "%s"\n' "$(vdf_escape "$output_dir")"
    printf '  "previewfile" "%s"\n' "$(vdf_escape "$preview_file")"
    printf '  "visibility" "%s"\n' "$WORKSHOP_VISIBILITY"
    printf '  "title" "%s"\n' "$TITLE"
    printf '  "description" "%s"\n' "$DESCRIPTION"
    printf '  "changenote" "Package %s from %s"\n' "$package_version" "$source_revision"
    printf '}\n'
} > "$vdf_path"

printf 'Workshop package staged at %s\n' "$output_dir"
printf 'SteamCMD/ISteamUGC configuration staged at %s\n' "$vdf_path"
