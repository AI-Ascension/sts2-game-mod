#!/usr/bin/env bash

set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
temp_dir=$(mktemp -d)
cleanup() {
    rm -rf -- "$temp_dir"
}
trap cleanup EXIT

payload_dir="$temp_dir/payload"
output_dir="$temp_dir/workshop-item"
preview_file="$temp_dir/preview.jpg"
mkdir -p "$payload_dir"
printf 'synthetic managed payload\n' > "$payload_dir/AIAscensionSTS2GameMod.dll"
printf '{"id":"synthetic-loader"}\n' > "$payload_dir/AIAscensionSTS2GameMod.json"
printf 'synthetic native payload\n' > "$payload_dir/AIAscensionSTS2GameModNative.dll"
printf 'synthetic preview\n' > "$preview_file"

bash "$script_dir/package-item.sh" \
    "$payload_dir" "$output_dir" 480 123456789 0.107.1 0.1.0 commit-123 "$preview_file"

[[ -f "$output_dir/sts2-workshop-manifest.json" ]]
[[ -f "$output_dir/SHA256SUMS" ]]
[[ -f "$output_dir/AIAscensionSTS2GameMod.dll" ]]
[[ -f "$output_dir/AIAscensionSTS2GameMod.json" ]]
[[ -f "$output_dir/AIAscensionSTS2GameModNative.dll" ]]
[[ -f "$output_dir.vdf" ]]

file_count=$(find "$output_dir" -mindepth 1 -maxdepth 1 -type f | wc -l | tr -d '[:space:]')
[[ "$file_count" == 5 ]]
sha256sum --check --strict <(sed "s#  #  $output_dir/#" "$output_dir/SHA256SUMS") >/dev/null

python3 - "$output_dir/sts2-workshop-manifest.json" "$output_dir.vdf" <<'PY'
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert manifest["schema_version"] == "sts2-workshop-manifest-v1"
assert manifest["consumer_app_id"] == 480
assert manifest["published_file_id"] == 123456789
assert manifest["content_kind"] == "first_party_executable"
assert [item["path"] for item in manifest["files"]] == [
    "AIAscensionSTS2GameMod.dll",
    "AIAscensionSTS2GameMod.json",
    "AIAscensionSTS2GameModNative.dll",
]
assert all(len(item["sha256"]) == 64 for item in manifest["files"])
assert len(manifest["content_digest"]) == 64

vdf = pathlib.Path(sys.argv[2]).read_text()
assert '"appid" "480"' in vdf
assert '"publishedfileid" "123456789"' in vdf
assert '"previewfile"' in vdf
PY

if bash "$script_dir/package-item.sh" \
    "$payload_dir" "$temp_dir/rejected" 480 123456789 0.107.1 0.1.0 bad..revision "$preview_file"; then
    printf '%s\n' 'expected unsafe source revision to be rejected' >&2
    exit 1
fi

printf '%s\n' 'Workshop package tool test passed.'

expect_rejected() {
    local output=$1
    shift
    if bash "$script_dir/package-item.sh" "$payload_dir" "$output" "$@" "$preview_file"; then
        printf '%s\n' 'expected invalid package input to be rejected' >&2
        exit 1
    fi
    [[ ! -e "$output" && ! -e "$output.vdf" ]]
}
expect_rejected "$temp_dir/leading-zero" 480 00123 0.107.1 0.1.0 commit-123
expect_rejected "$temp_dir/app-overflow" 4294967296 123 0.107.1 0.1.0 commit-123
expect_rejected "$temp_dir/item-overflow" 480 18446744073709551616 0.107.1 0.1.0 commit-123
expect_rejected "$temp_dir/unsafe-version" 480 123 bad..version 0.1.0 commit-123
expect_rejected "$payload_dir/nested" 480 123 0.107.1 0.1.0 commit-123
truncate -s 268435457 "$payload_dir/AIAscensionSTS2GameMod.dll"
expect_rejected "$temp_dir/large-payload" 480 123 0.107.1 0.1.0 commit-123
printf '%s\n' 'Workshop package negative tests passed.'
