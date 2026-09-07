#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -Eeuo pipefail

script_dir=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
repo_root=$(CDPATH= cd -- "$script_dir/../.." && pwd -P)
source_commit=$(git -C "$repo_root" rev-parse HEAD)
temp_dir=$(mktemp -d -t sts2-runtime-receipt-XXXXXXXX)
clean_worktree=''
cross_device_output=''
cleanup() {
    if [[ -n "$clean_worktree" && -d "$clean_worktree" ]]; then
        git -C "$repo_root" worktree remove --force "$clean_worktree" >/dev/null 2>&1 || true
    fi
    if [[ -n "$cross_device_output" && -e "$cross_device_output" ]]; then
        rm -rf -- "$cross_device_output"
    fi
    rm -rf -- "$temp_dir"
}
trap cleanup EXIT

fixture="$temp_dir/fixture"
mkdir -p -- "$fixture"
printf 'managed fixture bytes\n' > "$fixture/AIAscensionSTS2GameMod.dll"
printf '{"id":"AIAscensionSTS2GameMod","version":"0.4.0"}\n' > "$fixture/AIAscensionSTS2GameMod.json"
printf 'native fixture bytes\n' > "$fixture/libAIAscensionSTS2GameModNative.so"

dotnet_real="$temp_dir/dotnet.exe"
printf '#!/bin/sh\nexit 0\n' > "$dotnet_real"
chmod 0755 "$dotnet_real"
ln -s -- "$dotnet_real" "$temp_dir/dotnet-link"
PYTHONPATH="$script_dir" python3 -B - "$dotnet_real" "$temp_dir/dotnet-link" <<'PY'
import sys

from runtime_receipt_build import resolve_dotnet

real = resolve_dotnet(sys.argv[1])
link = resolve_dotnet(sys.argv[2])
assert real.kind == "windows-exe" and real.path_style == "windows"
assert link.kind == "windows-exe" and link.path_style == "windows"
assert link.path == real.path
PY

if bash "$script_dir/build-runtime-receipt.sh" --scope fixture --fixture-payload-dir "$fixture" \
    --platform linux-x86_64 --source-commit "$source_commit" --game-version 0.107.1 \
    --package-version 0.4.0 --output-dir "$repo_root/tools/release/receipt-inside-source" >/dev/null 2>&1; then
    printf '%s\n' 'output inside the source checkout was unexpectedly accepted' >&2
    exit 1
fi
[[ ! -e "$repo_root/tools/release/receipt-inside-source" ]]

cross_device_output="/dev/shm/sts2-runtime-receipt-$$-${RANDOM}"
if [[ "$(stat -c '%d' "$temp_dir")" == "$(stat -c '%d' /dev/shm)" ]]; then
    printf '%s\n' 'cross-device receipt test requires /dev/shm on a different filesystem' >&2
    exit 1
fi
if bash "$script_dir/build-runtime-receipt.sh" --scope fixture --fixture-payload-dir "$fixture" \
    --platform linux-x86_64 --source-commit "$source_commit" --game-version 0.107.1 \
    --package-version 0.4.0 --output-dir "$cross_device_output" \
    --evidence-dir "$temp_dir/cross-device-evidence" >/dev/null 2>&1; then
    printf '%s\n' 'cross-device publication unexpectedly succeeded' >&2
    exit 1
fi
[[ ! -e "$cross_device_output" && ! -e "$cross_device_output.claim" ]]
[[ $(find "$temp_dir/cross-device-evidence" -mindepth 1 -maxdepth 1 -type d -name 'scratch-*' | wc -l | tr -d '[:space:]') == 1 ]]
cross_device_scratch=$(find "$temp_dir/cross-device-evidence" -mindepth 1 -maxdepth 1 -type d -name 'scratch-*')
[[ -d "$cross_device_scratch/receipt-root/payload" ]]
cross_device_output=''

clean_worktree="$temp_dir/clean-worktree"
git -C "$repo_root" worktree add --detach "$clean_worktree" "$source_commit" >/dev/null
clean_host="$temp_dir/clean-host"
mkdir -p -- "$clean_host"
printf 'host assembly fixture\n' > "$clean_host/sts2.dll"
printf 'host support fixture\n' > "$clean_host/GodotSharp.dll"
if env -u PYTHONDONTWRITEBYTECODE bash "$clean_worktree/tools/release/build-runtime-receipt.sh" \
    --scope production --host-data-dir "$clean_host" --dotnet "$temp_dir/missing-dotnet" \
    --platform linux-x86_64 --source-commit "$source_commit" --game-version 0.107.1 \
    --package-version 0.4.0 --output-dir "$temp_dir/clean-production" \
    --evidence-dir "$temp_dir/clean-production-evidence" >/dev/null 2>&1; then
    printf '%s\n' 'missing production dotnet unexpectedly succeeded' >&2
    exit 1
fi
[[ -z "$(git -C "$clean_worktree" status --porcelain=v1 --untracked-files=all)" ]]
[[ ! -d "$clean_worktree/tools/release/__pycache__" ]]
git -C "$repo_root" worktree remove --force "$clean_worktree" >/dev/null
clean_worktree=''

output="$temp_dir/receipt"
STS2_RELEASE_TEST_FIXTURE=1 bash "$script_dir/build-runtime-receipt.sh" \
    --scope fixture --fixture-payload-dir "$fixture" --platform linux-x86_64 \
    --source-commit "$source_commit" --game-version 0.107.1 --package-version 0.4.0 \
    --output-dir "$output" >/dev/null

[[ -d "$output" && -f "$output/build-receipt.json" && -d "$output/payload" ]]
[[ $(find "$output" -mindepth 1 -maxdepth 1 -printf '%f\n' | sort | tr '\n' ' ') == 'build-receipt.json payload ' ]]
[[ $(find "$output/payload" -mindepth 1 -maxdepth 1 -type f | wc -l | tr -d '[:space:]') == 3 ]]
[[ ! -e "$output.claim" ]]
python3 - "$output/build-receipt.json" "$output/payload" <<'PY'
import hashlib
import json
import pathlib
import sys

receipt = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
payload = pathlib.Path(sys.argv[2])
assert receipt["schema_version"] == "sts2-release-build-receipt-v1"
assert receipt["scope"] == "fixture"
assert receipt["platform"] == "linux-x86_64"
assert receipt["fixture_flags"] == ["fixture_payload"]
assert receipt["host_references"] == []
assert receipt["toolchain"] == {
    "dotnet_executable": "fixture",
    "dotnet_path_style": "fixture",
    "dotnet_version": "fixture",
    "rustc_version": "fixture",
    "cargo_version": "fixture",
}
assert receipt["loader_manifest_version"] == receipt["package_version"] == "0.4.0"
assert [record["path"] for record in receipt["payload"]] == [
    "AIAscensionSTS2GameMod.dll", "AIAscensionSTS2GameMod.json", "libAIAscensionSTS2GameModNative.so"
]
for record in receipt["payload"]:
    value = (payload / record["path"]).read_bytes()
    assert len(value) == record["size_bytes"]
    assert hashlib.sha256(value).hexdigest() == record["sha256"]
assert "fixture" not in json.dumps(receipt["payload"])
PY

if bash "$script_dir/build-runtime-receipt.sh" --platform linux-x86_64 \
    --source-commit "$source_commit" --game-version 0.107.1 --package-version 0.4.0 \
    --output-dir "$temp_dir/no-host" >/dev/null 2>&1; then
    printf '%s\n' 'production receipt without host references unexpectedly succeeded' >&2
    exit 1
fi

if bash "$script_dir/build-runtime-receipt.sh" --scope production --fixture-payload-dir "$fixture" \
    --platform linux-x86_64 --source-commit "$source_commit" --game-version 0.107.1 \
    --package-version 0.4.0 --host-data-dir "$temp_dir/no-host-data" --output-dir "$temp_dir/fixture-as-production" >/dev/null 2>&1; then
    printf '%s\n' 'fixture payload was unexpectedly accepted in production scope' >&2
    exit 1
fi

if bash "$script_dir/build-runtime-receipt.sh" --scope fixture --fixture-payload-dir "$fixture" \
    --platform linux-x86_64 --source-commit "0000000000000000000000000000000000000000" \
    --game-version 0.107.1 --package-version 0.4.0 --output-dir "$temp_dir/wrong-source" >/dev/null 2>&1; then
    printf '%s\n' 'wrong source commit was unexpectedly accepted' >&2
    exit 1
fi

sed 's/"version":"0.4.0"/"version":"0.4.1"/' "$fixture/AIAscensionSTS2GameMod.json" > "$fixture/loader-wrong-version.json"
mv -- "$fixture/loader-wrong-version.json" "$fixture/AIAscensionSTS2GameMod.json"
if bash "$script_dir/build-runtime-receipt.sh" --scope fixture --fixture-payload-dir "$fixture" \
    --platform linux-x86_64 --source-commit "$source_commit" --game-version 0.107.1 \
    --package-version 0.4.0 --output-dir "$temp_dir/wrong-version" >/dev/null 2>&1; then
    printf '%s\n' 'loader version mismatch was unexpectedly accepted' >&2
    exit 1
fi
printf '{"id":"AIAscensionSTS2GameMod","version":"0.4.0"}\n' > "$fixture/AIAscensionSTS2GameMod.json"

if STS2_RELEASE_TEST_INTERRUPT_BEFORE_PUBLISH=1 bash "$script_dir/build-runtime-receipt.sh" \
    --scope fixture --fixture-payload-dir "$fixture" --platform linux-x86_64 \
    --source-commit "$source_commit" --game-version 0.107.1 --package-version 0.4.0 \
    --output-dir "$temp_dir/interrupted" >/dev/null 2>&1; then
    printf '%s\n' 'interrupted receipt unexpectedly succeeded' >&2
    exit 1
fi
[[ ! -e "$temp_dir/interrupted" && ! -e "$temp_dir/interrupted.claim" ]]
evidence_count=$(find "$temp_dir" -mindepth 1 -maxdepth 1 -type d -name '.interrupted.evidence-*' | wc -l | tr -d '[:space:]')
[[ "$evidence_count" == 1 ]]
[[ $(find "$temp_dir" -mindepth 2 -maxdepth 2 -type d -path "$temp_dir/.interrupted.evidence-*/scratch-*" | wc -l | tr -d '[:space:]') == 1 ]]

set +e
bash "$script_dir/build-runtime-receipt.sh" --scope fixture --fixture-payload-dir "$fixture" \
    --platform linux-x86_64 --source-commit "$source_commit" --game-version 0.107.1 \
    --package-version 0.4.0 --output-dir "$temp_dir/race" >/dev/null 2>&1 &
first=$!
bash "$script_dir/build-runtime-receipt.sh" --scope fixture --fixture-payload-dir "$fixture" \
    --platform linux-x86_64 --source-commit "$source_commit" --game-version 0.107.1 \
    --package-version 0.4.0 --output-dir "$temp_dir/race" >/dev/null 2>&1 &
second=$!
wait "$first"; first_status=$?
wait "$second"; second_status=$?
set -e
if ! { [[ "$first_status" == 0 && "$second_status" != 0 ]] || \
       [[ "$first_status" != 0 && "$second_status" == 0 ]]; }; then
    printf '%s\n' "expected exactly one racing receipt to win: $first_status/$second_status" >&2
    exit 1
fi
[[ -f "$temp_dir/race/build-receipt.json" ]]

printf '%s\n' 'runtime receipt fixture, provenance, version, interruption, and race checks passed.'
