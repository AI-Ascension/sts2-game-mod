#!/usr/bin/env bash

set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$script_dir/../.." && pwd)
temp_dir=$(mktemp -d)
cleanup() {
    rm -rf -- "$temp_dir"
}
trap cleanup EXIT

windows_receipt="$temp_dir/windows-receipt"
payload_dir="$windows_receipt/payload"
output_dir="$temp_dir/workshop-item"
preview_file="$temp_dir/preview.jpg"
mkdir -p "$payload_dir"
printf 'synthetic managed payload\n' > "$payload_dir/AIAscensionSTS2GameMod.dll"
git show "$(git -C "$repo_root" rev-parse HEAD):experiments/managed-rust-interop/game-loader/mod_manifest.json" \
    > "$payload_dir/AIAscensionSTS2GameMod.json"
printf 'synthetic native payload\n' > "$payload_dir/AIAscensionSTS2GameModNative.dll"
printf 'synthetic preview\n' > "$preview_file"

linux_receipt="$temp_dir/linux-receipt"
linux_payload="$linux_receipt/payload"
linux_output="$temp_dir/linux-workshop-item"
mkdir -p "$linux_payload"
printf 'synthetic managed payload\n' > "$linux_payload/AIAscensionSTS2GameMod.dll"
cp -- "$payload_dir/AIAscensionSTS2GameMod.json" "$linux_payload/AIAscensionSTS2GameMod.json"
printf 'synthetic linux native payload\n' > "$linux_payload/libAIAscensionSTS2GameModNative.so"
source_revision=$(git -C "$repo_root" rev-parse HEAD)
build_manifest="$temp_dir/build-artifact-manifest.json"
python3 - "$windows_receipt" "$linux_receipt" "$source_revision" "$repo_root" <<'PY'
import hashlib
import json
import pathlib
import subprocess
import sys

windows_root = pathlib.Path(sys.argv[1])
linux_root = pathlib.Path(sys.argv[2])
source_revision = sys.argv[3]
repo = pathlib.Path(sys.argv[4])
source_tree = subprocess.check_output(
    ["git", "-C", str(repo), "rev-parse", f"{source_revision}^{{tree}}"], text=True
).strip()
host_bytes = b"synthetic host reference\n"
toolchain = {
    "dotnet_executable": "dotnet",
    "dotnet_path_style": "posix",
    "dotnet_version": "9.0.317",
    "rustc_version": "rustc 1.97.1 (fixture-test 2026-01-01)",
    "cargo_version": "cargo 1.97.1 (fixture-test 2026-01-01)",
}

for platform, root, native_name in (
    ("windows-x86_64", windows_root, "AIAscensionSTS2GameModNative.dll"),
    ("linux-x86_64", linux_root, "libAIAscensionSTS2GameModNative.so"),
):
    payload = root / "payload"
    records = []
    for path in (
        "AIAscensionSTS2GameMod.dll",
        "AIAscensionSTS2GameMod.json",
        native_name,
    ):
        data = (payload / path).read_bytes()
        records.append({
            "path": path,
            "size_bytes": len(data),
            "sha256": hashlib.sha256(data).hexdigest(),
        })
    receipt = {
        "schema_version": "sts2-release-build-receipt-v1",
        "scope": "production",
        "platform": platform,
        "source_commit": source_revision,
        "source_tree": source_tree,
        "game_version": "0.107.1",
        "package_version": "0.4.0",
        "loader_manifest_version": "0.4.0",
        "toolchain": toolchain,
        "host_references": [
            {
                "name": name,
                "size_bytes": len(host_bytes),
                "sha256": hashlib.sha256(host_bytes).hexdigest(),
            }
            for name in ("sts2.dll", "GodotSharp.dll")
        ],
        "fixture_flags": [],
        "payload": records,
    }
    (root / "build-receipt.json").write_text(
        json.dumps(receipt, indent=2) + "\n", encoding="utf-8"
    )
PY
STS2_RELEASE_ALLOW_DIRTY=1 bash "$repo_root/tools/release/build-artifact-manifest.sh" \
    "$source_revision" 0.107.1 0.4.0 "$windows_receipt" "$linux_receipt" "$build_manifest" >/dev/null
python3 - "$build_manifest" <<'PY'
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert manifest["schema_version"] == "sts2-build-artifact-manifest-v2"
assert manifest["loader_manifest_version"] == "0.4.0"
assert set(manifest["receipts"]) == {"windows-x86_64", "linux-x86_64"}
for platform, receipt in manifest["receipts"].items():
    assert receipt["scope"] == "production"
    assert receipt["fixture_flags"] == []
    assert receipt["platform"] == platform
    assert receipt["payload"] == manifest["artifacts"][platform]
    assert set(receipt["toolchain"]) == {
        "dotnet_executable", "dotnet_path_style", "dotnet_version",
        "rustc_version", "cargo_version",
    }
    assert [item["name"] for item in receipt["host_references"]] == [
        "sts2.dll", "GodotSharp.dll"
    ]
assert manifest["loader_manifest"]["path"].endswith("/mod_manifest.json")
PY

fixture_receipt="$temp_dir/fixture-receipt"
cp -a -- "$windows_receipt" "$fixture_receipt"
python3 - "$fixture_receipt/build-receipt.json" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
value = json.loads(path.read_text(encoding="utf-8"))
value["scope"] = "fixture"
value["fixture_flags"] = ["fixture_payload"]
path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
PY
if STS2_RELEASE_ALLOW_DIRTY=1 bash "$repo_root/tools/release/build-artifact-manifest.sh" \
    "$source_revision" 0.107.1 0.4.0 "$fixture_receipt" "$linux_receipt" \
    "$temp_dir/fixture-build-manifest.json" >/dev/null 2>&1; then
    printf '%s\n' 'fixture receipt was unexpectedly accepted by the paired manifest producer' >&2
    exit 1
fi

bad_payload_receipt="$temp_dir/bad-payload-receipt"
cp -a -- "$linux_receipt" "$bad_payload_receipt"
printf 'receipt mismatch\n' >> "$bad_payload_receipt/payload/libAIAscensionSTS2GameModNative.so"
if STS2_RELEASE_ALLOW_DIRTY=1 bash "$repo_root/tools/release/build-artifact-manifest.sh" \
    "$source_revision" 0.107.1 0.4.0 "$windows_receipt" "$bad_payload_receipt" \
    "$temp_dir/bad-payload-manifest.json" >/dev/null 2>&1; then
    printf '%s\n' 'payload bytes changed after receipt were unexpectedly accepted' >&2
    exit 1
fi

for platform in windows-x86_64 linux-x86_64; do
    source_root="$windows_receipt"
    [[ "$platform" == windows-x86_64 ]] || source_root="$linux_receipt"
    for bad_kind in string integer null; do
        bad_root="$temp_dir/bad-platform-$platform-$bad_kind"
        cp -a -- "$source_root" "$bad_root"
        python3 - "$bad_root/build-receipt.json" "$bad_kind" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
value = json.loads(path.read_text(encoding="utf-8"))
bad_kind = sys.argv[2]
value["platform"] = {
    "string": "wrong-platform",
    "integer": 7,
    "null": None,
}[bad_kind]
path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
PY
        windows_arg="$windows_receipt"
        linux_arg="$linux_receipt"
        [[ "$platform" == windows-x86_64 ]] && windows_arg="$bad_root" || linux_arg="$bad_root"
        if STS2_RELEASE_ALLOW_DIRTY=1 bash "$repo_root/tools/release/build-artifact-manifest.sh" \
            "$source_revision" 0.107.1 0.4.0 "$windows_arg" "$linux_arg" \
            "$temp_dir/bad-platform-$platform-$bad_kind-manifest.json" >/dev/null 2>&1; then
            printf '%s\n' "receipt platform $platform/$bad_kind was unexpectedly accepted" >&2
            exit 1
        fi
    done
done

bash "$script_dir/package-item.sh" \
    "$payload_dir" "$output_dir" 480 123456789 0.107.1 0.4.0 "$source_revision" "$preview_file" \
    --build-manifest "$build_manifest"

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

bad_receipt_manifest="$temp_dir/bad-receipt-manifest.json"
cp -- "$build_manifest" "$bad_receipt_manifest"
python3 - "$bad_receipt_manifest" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
value = json.loads(path.read_text(encoding="utf-8"))
value["receipts"]["linux-x86_64"]["fixture_flags"] = ["fixture_payload"]
path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
PY
if bash "$script_dir/package-item.sh" \
    "$payload_dir" "$temp_dir/bad-receipt-output" 480 123456789 0.107.1 0.4.0 \
    "$source_revision" "$preview_file" --build-manifest "$bad_receipt_manifest" \
    >/dev/null 2>&1; then
    printf '%s\n' 'fixture receipt metadata was unexpectedly accepted by the packager' >&2
    exit 1
fi
[[ ! -e "$temp_dir/bad-receipt-output" && ! -e "$temp_dir/bad-receipt-output.vdf" ]]

corrupt_payload="$temp_dir/corrupt-payload"
cp -a -- "$payload_dir" "$corrupt_payload"
printf 'corruption\n' >> "$corrupt_payload/AIAscensionSTS2GameMod.dll"
if bash "$script_dir/package-item.sh" \
    "$corrupt_payload" "$temp_dir/corrupt-output" 480 123456789 0.107.1 0.4.0 \
    "$source_revision" "$preview_file" --build-manifest "$build_manifest" >/dev/null 2>&1; then
    printf '%s\n' 'payload corruption was unexpectedly accepted' >&2
    exit 1
fi
[[ ! -e "$temp_dir/corrupt-output" && ! -e "$temp_dir/corrupt-output.vdf" ]]

corrupt_manifest="$temp_dir/corrupt-build-manifest.json"
cp -- "$build_manifest" "$corrupt_manifest"
python3 - "$corrupt_manifest" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
value = json.loads(path.read_text(encoding="utf-8"))
value["package_version"] = "0.4.1"
path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
PY
if bash "$script_dir/package-item.sh" \
    "$payload_dir" "$temp_dir/corrupt-manifest-output" 480 123456789 0.107.1 0.4.0 \
    "$source_revision" "$preview_file" --build-manifest "$corrupt_manifest" >/dev/null 2>&1; then
    printf '%s\n' 'build manifest corruption was unexpectedly accepted' >&2
    exit 1
fi
[[ ! -e "$temp_dir/corrupt-manifest-output" && ! -e "$temp_dir/corrupt-manifest-output.vdf" ]]

if STS2_RELEASE_TEST_INTERRUPT_AFTER_COPY=1 bash "$script_dir/package-item.sh" \
    "$payload_dir" "$temp_dir/interrupted-output" 480 123456788 0.107.1 0.4.0 \
    "$source_revision" "$preview_file" --build-manifest "$build_manifest" >/dev/null 2>&1; then
    printf '%s\n' 'interrupted package unexpectedly succeeded' >&2
    exit 1
fi
[[ ! -e "$temp_dir/interrupted-output" && ! -e "$temp_dir/interrupted-output.vdf" ]]
[[ ! -e "$temp_dir/interrupted-output.claim" ]]

set +e
bash "$script_dir/package-item.sh" \
    "$payload_dir" "$temp_dir/raced-output" 480 123456787 0.107.1 0.4.0 \
    "$source_revision" "$preview_file" --build-manifest "$build_manifest" >/dev/null 2>&1 &
race_one=$!
bash "$script_dir/package-item.sh" \
    "$payload_dir" "$temp_dir/raced-output" 480 123456786 0.107.1 0.4.0 \
    "$source_revision" "$preview_file" --build-manifest "$build_manifest" >/dev/null 2>&1 &
race_two=$!
wait "$race_one"; race_status_one=$?
wait "$race_two"; race_status_two=$?
set -e
if ! { [[ "$race_status_one" == 0 && "$race_status_two" != 0 ]] || \
       [[ "$race_status_one" != 0 && "$race_status_two" == 0 ]]; }; then
    printf '%s\n' "expected exactly one racing package to win: $race_status_one/$race_status_two" >&2
    exit 1
fi
[[ -f "$temp_dir/raced-output/sts2-workshop-manifest.json" ]]

if bash "$script_dir/package-item.sh" \
    "$payload_dir" "$temp_dir/rejected" 480 123456789 0.107.1 0.4.0 bad..revision "$preview_file"; then
    printf '%s\n' 'expected unsafe source revision to be rejected' >&2
    exit 1
fi

printf '%s\n' 'Workshop package tool test passed.'

bash "$script_dir/package-platform-item.sh" \
    linux-x86_64 "$linux_payload" "$linux_output" 480 123456790 0.107.1 0.4.0 "$source_revision" "$preview_file" \
    --build-manifest "$build_manifest"

[[ -f "$linux_output/libAIAscensionSTS2GameModNative.so" ]]
[[ ! -e "$linux_output/AIAscensionSTS2GameModNative.dll" ]]
linux_file_count=$(find "$linux_output" -mindepth 1 -maxdepth 1 -type f | wc -l | tr -d '[:space:]')
[[ "$linux_file_count" == 5 ]]
sha256sum --check --strict <(sed "s#  #  $linux_output/#" "$linux_output/SHA256SUMS") >/dev/null
python3 - "$linux_output/sts2-workshop-manifest.json" <<'PY'
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert manifest["platform"] == "linux-x86_64"
assert [item["path"] for item in manifest["files"]] == [
    "AIAscensionSTS2GameMod.dll",
    "AIAscensionSTS2GameMod.json",
    "libAIAscensionSTS2GameModNative.so",
]
PY

if bash "$script_dir/package-platform-item.sh" \
    macos-x86_64 "$linux_payload" "$temp_dir/unsupported" 480 123456790 0.107.1 0.1.0 commit-123 "$preview_file" --legacy-unbound; then
    printf '%s\n' 'expected unsupported platform to be rejected' >&2
    exit 1
fi
printf '%s\n' 'Windows and Linux Workshop package tests passed.'

expect_rejected() {
    local output=$1
    shift
    if bash "$script_dir/package-item.sh" "$payload_dir" "$output" "$@" "$preview_file" --legacy-unbound; then
        printf '%s\n' 'expected invalid package input to be rejected' >&2
        exit 1
    fi
    [[ ! -e "$output" && ! -e "$output.vdf" ]]
}

if bash "$script_dir/package-item.sh" \
    "$payload_dir" "$temp_dir/missing-manifest" 480 123 0.107.1 0.1.0 commit-123 "$preview_file"; then
    printf '%s\n' 'unbound package was unexpectedly accepted without explicit legacy mode' >&2
    exit 1
fi
[[ ! -e "$temp_dir/missing-manifest" && ! -e "$temp_dir/missing-manifest.vdf" ]]
printf '%s\n' 'manifest provenance gate passed.'
expect_rejected "$temp_dir/leading-zero" 480 00123 0.107.1 0.1.0 commit-123
expect_rejected "$temp_dir/app-overflow" 4294967296 123 0.107.1 0.1.0 commit-123
expect_rejected "$temp_dir/item-overflow" 480 18446744073709551616 0.107.1 0.1.0 commit-123
expect_rejected "$temp_dir/unsafe-version" 480 123 bad..version 0.1.0 commit-123
expect_rejected "$payload_dir/nested" 480 123 0.107.1 0.1.0 commit-123
truncate -s 268435457 "$payload_dir/AIAscensionSTS2GameMod.dll"
expect_rejected "$temp_dir/large-payload" 480 123 0.107.1 0.1.0 commit-123
printf '%s\n' 'Workshop package negative tests passed.'
