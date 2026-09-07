#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -Eeuo pipefail

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
source_root=${1:-${VALVE_SOURCE_SDK_ROOT:-}}
expected_commit=${VALVE_SOURCE_SDK_COMMIT:-b8cfb12c0e083a2ef5b2f9f9b50f3902fa034474}
[[ -n "$source_root" ]] || {
    printf 'usage: %s /path/to/ValveSoftware/source-sdk-2013\n' "$0" >&2
    printf 'Set VALVE_SOURCE_SDK_ROOT and optionally VALVE_SOURCE_SDK_COMMIT.\n' >&2
    exit 2
}
[[ -d "$source_root" && ! -L "$source_root" ]] || {
    printf 'SDK root must be a regular non-symlink directory: %s\n' "$source_root" >&2
    exit 2
}
header_root=$(realpath -e -- "$source_root/src/public/steam") || {
    printf 'SDK root does not contain src/public/steam: %s\n' "$source_root" >&2
    exit 2
}
for header in steamclientpublic.h isteamugc.h steam_api_common.h steamtypes.h; do
    [[ -f "$header_root/$header" && ! -L "$header_root/$header" ]] || {
        printf 'missing SDK header: %s\n' "$header" >&2
        exit 2
    }
done
actual_commit=$(git -C "$source_root" rev-parse HEAD 2>/dev/null) || {
    printf 'SDK root must be a Git checkout so its source identity is reviewable\n' >&2
    exit 2
}
[[ -z "$(git -C "$source_root" status --porcelain=v1 --untracked-files=all)" ]] || {
    printf 'SDK root must be a clean checkout; review or remove local changes first\n' >&2
    exit 2
}
[[ "$actual_commit" == "$expected_commit" ]] || {
    printf 'SDK commit mismatch: expected %s, got %s\n' "$expected_commit" "$actual_commit" >&2
    exit 2
}

compiler=${CXX:-c++}
command -v "$compiler" >/dev/null || {
    printf 'C++ compiler not found: %s\n' "$compiler" >&2
    exit 2
}
output_dir=$(mktemp -d -t sts2-steam-abi-XXXXXXXX)
trap 'rm -rf -- "$output_dir"' EXIT
"$compiler" -std=c++17 -Wall -Wextra -Werror -pedantic \
    -I "$header_root" "$script_dir/create-item-result-linux.cpp" \
    -o "$output_dir/create-item-result-linux"
"$output_dir/create-item-result-linux"
printf 'Valve SDK Linux CreateItemResult_t ABI proof passed: commit=%s size=16 offsets=(0,4,12) callback=%s\n' \
    "$actual_commit" "$((3400 + 3))"
