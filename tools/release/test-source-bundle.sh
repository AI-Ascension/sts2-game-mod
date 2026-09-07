#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$script_dir/../.." && pwd)
test_dir=$(mktemp -d -t sts2-source-bundle-test-XXXXXXXX)
cleanup() {
    rm -rf -- "$test_dir"
}
trap cleanup EXIT

export SOURCE_DATE_EPOCH=0
version=0.4.0
platform=linux-x86_64
(umask 022; cd "$repo_root" && bash "$script_dir/build-source-bundle.sh" "$version" "$platform" "$test_dir/first" HEAD)
(umask 077; cd "$repo_root" && bash "$script_dir/build-source-bundle.sh" "$version" "$platform" "$test_dir/second" HEAD)

first_archive=$(find "$test_dir/first" -maxdepth 1 -type f -name '*.tar.gz' -print -quit)
second_archive=$(find "$test_dir/second" -maxdepth 1 -type f -name '*.tar.gz' -print -quit)
[[ -n "$first_archive" && -n "$second_archive" ]]
cmp "$first_archive" "$second_archive"
cmp "$first_archive.sha256" "$second_archive.sha256"
(cd "$(dirname "$first_archive")" && sha256sum --check --strict "$(basename "$first_archive.sha256")")

extract_dir="$test_dir/extracted"
mkdir -p "$extract_dir"
tar -xzf "$first_archive" -C "$extract_dir"
bundle_root=$(find "$extract_dir" -mindepth 1 -maxdepth 1 -type d -print -quit)
[[ -n "$bundle_root" ]]
(cd "$bundle_root" && sha256sum --check --strict SHA256SUMS)

python3 - "$bundle_root/RELEASE-MANIFEST.json" <<'PY'
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert manifest["artifact_kind"] == "source_bundle"
assert manifest["artifact_scope"] == "source_only"
assert manifest["platform"] == "linux-x86_64"
assert manifest["proprietary_files_included"] is False
assert len(manifest["source_commit"]) == 40
assert len(manifest["source_tree"]) == 40
PY

if (cd "$repo_root" && bash "$script_dir/build-source-bundle.sh" "$version" 'linux/unsafe' "$test_dir/rejected" HEAD); then
    printf '%s\n' 'unsafe platform was unexpectedly accepted' >&2
    exit 1
fi

printf 'source bundle reproducibility and allowlist checks passed\n'
