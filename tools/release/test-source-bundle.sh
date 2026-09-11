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

python3 - "$bundle_root/RELEASE-MANIFEST.json" \
    "$bundle_root/tools/release/source-distribution-policy-v1.json" "$bundle_root" <<'PY'
import hashlib
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
policy_path = pathlib.Path(sys.argv[2])
bundle_root = pathlib.Path(sys.argv[3])
policy = json.loads(policy_path.read_text(encoding="utf-8"))
assert manifest["artifact_kind"] == "source_bundle"
assert manifest["artifact_scope"] == "production_source_only"
assert manifest["platform"] == "linux-x86_64"
assert manifest["proprietary_files_included"] is False
assert len(manifest["source_commit"]) == 40
assert len(manifest["source_tree"]) == 40
assert manifest["source_tree_scope"] == "original_full_git_tree"
assert manifest["source_policy_id"] == policy["policy_id"]
assert manifest["source_policy_sha256"] == hashlib.sha256(policy_path.read_bytes()).hexdigest()
assert manifest["excluded_paths"] == policy["excluded_paths"]

excluded = set(policy["excluded_paths"])
for relative in excluded:
    assert not (bundle_root / relative).exists(), relative

for relative in policy["required_paths"]:
    path = bundle_root / relative
    assert path.is_file() and not path.is_symlink(), relative

included_paths = sorted(
    str(path.relative_to(bundle_root))
    for path in bundle_root.rglob("*")
    if path.is_file() and not path.is_symlink()
    and str(path.relative_to(bundle_root)) not in {"RELEASE-MANIFEST.json", "SHA256SUMS"}
)
assert included_paths == policy["allowed_paths"]
inventory = "".join(
    f"{hashlib.sha256((bundle_root / relative).read_bytes()).hexdigest()}  {relative}\n"
    for relative in included_paths
).encode("utf-8")
assert manifest["included_path_count"] == len(included_paths)
assert manifest["included_content_digest"] == hashlib.sha256(inventory).hexdigest()
assert "experiments/managed-rust-interop/game-loader/RuntimeV3GameplayFixtures.cs" in excluded
assert (bundle_root / "experiments/managed-rust-interop/game-loader/LiveCombatDemo.cs").is_file()
assert (bundle_root / "experiments/managed-rust-interop/game-loader/LiveCombatSource.cs").is_file()
PY

if (cd "$repo_root" && bash "$script_dir/build-source-bundle.sh" "$version" 'linux/unsafe' "$test_dir/rejected" HEAD); then
    printf '%s\n' 'unsafe platform was unexpectedly accepted' >&2
    exit 1
fi

printf 'source bundle reproducibility and allowlist checks passed\n'
