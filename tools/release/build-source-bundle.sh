#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

usage() {
    printf 'usage: %s <version> <platform> <output-directory> [source-revision]\n' "$0" >&2
    printf 'platform is a compatibility label such as windows-x86_64 or linux-x86_64.\n' >&2
    exit 2
}

fail() {
    printf 'source bundle: %s\n' "$1" >&2
    exit 1
}

[[ $# -ge 3 && $# -le 4 ]] || usage
version=$1
platform=$2
output_dir=$3
source_revision=${4:-HEAD}

[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9.-]+)?$ ]] || {
    printf 'version must be a safe semantic version: %s\n' "$version" >&2
    exit 1
}
[[ "$platform" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]] || {
    printf 'platform must be a safe compatibility label: %s\n' "$platform" >&2
    exit 1
}
[[ "${SOURCE_DATE_EPOCH:-0}" =~ ^[0-9]+$ ]] || {
    printf 'SOURCE_DATE_EPOCH must be a non-negative decimal integer\n' >&2
    exit 1
}

repo_root=$(git rev-parse --show-toplevel)
remote_url=$(git config --get remote.origin.url || true)
component=${remote_url##*/}
component=${component%.git}
[[ -n "$component" ]] || component=$(basename "$repo_root")
[[ "$component" =~ ^[A-Za-z0-9._-]+$ ]] || {
    printf 'repository name is not a safe artifact component: %s\n' "$component" >&2
    exit 1
}
source_commit=$(git rev-parse --verify --end-of-options "$source_revision^{commit}")
source_tree=$(git rev-parse --verify "$source_commit^{tree}")
artifact_name="ai-ascension-${component}-${version}-${platform}-source"
output_dir=$(realpath -m -- "$output_dir")
mkdir -p -- "$output_dir"

work_dir=$(mktemp -d -t sts2-source-bundle-XXXXXXXX)
cleanup() {
    rm -rf -- "$work_dir"
}
trap cleanup EXIT
archive_root="$work_dir/$artifact_name"
mkdir -p -- "$archive_root"
policy_path="tools/release/source-distribution-policy-v1.json"
source_paths_file="$work_dir/source-paths"
allowed_paths_file="$work_dir/allowed-paths"
excluded_paths_file="$work_dir/excluded-paths"
required_paths_file="$work_dir/required-paths"

# Preserve the executable bits from Git independently of the caller's umask. Directory modes are
# normalized below because Git does not store them as tree metadata.
git archive --format=tar "$source_commit" | tar --extract --same-permissions --no-same-owner -f - -C "$archive_root"
find "$archive_root" -type d -exec chmod 0755 {} +

# The policy is an exact path inventory for the source commit. This makes a newly tracked file
# fail closed until the policy is reviewed and updated in the same source change.
git ls-tree -r --name-only "$source_commit" > "$source_paths_file"
policy_file="$archive_root/$policy_path"
[[ -f "$policy_file" && ! -L "$policy_file" ]] || fail "source policy is missing from the resolved commit: $policy_path"
excluded_json=$(
    cd -- "$repo_root"
    cargo +1.97.1 run --quiet --locked --offline --manifest-path "$repo_root/Cargo.toml" \
        --package sts2-release-tool -- validate-source-policy \
        "$policy_file" "$source_paths_file" "$allowed_paths_file" "$excluded_paths_file" \
        "$required_paths_file"
)
mapfile -t allowed_paths < "$allowed_paths_file"
mapfile -t excluded_paths < "$excluded_paths_file"
mapfile -t required_paths < "$required_paths_file"

for relative in "${excluded_paths[@]}"; do
    excluded_file="$archive_root/$relative"
    [[ -f "$excluded_file" && ! -L "$excluded_file" ]] \
        || fail "policy exclusion is not a regular source file: $relative"
    rm -- "$excluded_file"
done

for relative in "${allowed_paths[@]}"; do
    allowed_file="$archive_root/$relative"
    [[ -f "$allowed_file" && ! -L "$allowed_file" ]] \
        || fail "policy allowlist path is not a regular source file: $relative"
done
for relative in "${required_paths[@]}"; do
    required_file="$archive_root/$relative"
    [[ -f "$required_file" && ! -L "$required_file" ]] \
        || fail "required production source path is missing: $relative"
done

# A source bundle must never silently acquire a host binary, profile, save, or build tree.
while IFS= read -r -d '' path; do
    relative=${path#"$archive_root/"}
    printf 'unsupported non-regular release input in source tree: %s\n' "$relative" >&2
    exit 1
done < <(find "$archive_root" \( -type l -o -type b -o -type c -o -type p \) -print0)
while IFS= read -r -d '' path; do
    relative=${path#"$archive_root/"}
    case "$relative" in
        .git/*|target/*|*/target/*|bin/*|*/bin/*|obj/*|*/obj/*|*.dll|*.dylib|*.so|*.pdb|*.save|*.sav|*.sts2profile|.env|.env.*|*/.env|*/.env.*)
            printf 'unsupported release input in source tree: %s\n' "$relative" >&2
            exit 1
            ;;
    esac
done < <(find "$archive_root" -type f -print0)

policy_sha256=$(sha256sum -- "$policy_file" | cut -d ' ' -f1)
included_inventory_file="$work_dir/included-content.sha256"
(
    cd -- "$archive_root"
    while IFS= read -r relative; do
        sha256sum -- "$relative"
    done < <(find . -type f ! -path './RELEASE-MANIFEST.json' ! -path './SHA256SUMS' \
        -printf '%P\n' | LC_ALL=C sort)
) > "$included_inventory_file"
included_path_count=$(wc -l < "$included_inventory_file" | tr -d '[:space:]')
included_content_digest=$(sha256sum -- "$included_inventory_file" | cut -d ' ' -f1)

cat > "$archive_root/RELEASE-MANIFEST.json" <<EOF
{
  "schema_version": "ai-ascension-release-manifest-v1",
  "artifact_kind": "source_bundle",
  "artifact_scope": "production_source_only",
  "component": "${component}",
  "version": "${version}",
  "platform": "${platform}",
  "source_commit": "${source_commit}",
  "source_tree": "${source_tree}",
  "source_tree_scope": "original_full_git_tree",
  "source_policy_id": "source-distribution-v1",
  "source_policy_sha256": "${policy_sha256}",
  "included_path_count": ${included_path_count},
  "included_content_digest": "${included_content_digest}",
  "excluded_paths": ${excluded_json},
  "source_date_epoch": ${SOURCE_DATE_EPOCH:-0},
  "checksum_inventory": "SHA256SUMS",
  "proprietary_files_included": false,
  "host_installation_evidence": "unverified",
  "workshop_publication_evidence": "unverified"
}
EOF
chmod 0644 "$archive_root/RELEASE-MANIFEST.json"

(
    cd -- "$archive_root"
    : > SHA256SUMS
    while IFS= read -r relative; do
        sha256sum -- "$relative"
    done < <(find . -type f ! -path './SHA256SUMS' -printf '%P\n' | LC_ALL=C sort)
) > "$archive_root/SHA256SUMS"
chmod 0644 "$archive_root/SHA256SUMS"

archive_path="$output_dir/$artifact_name.tar.gz"
tar --create --file=- --sort=name --mtime="@${SOURCE_DATE_EPOCH:-0}" \
    --owner=0 --group=0 --numeric-owner --format=ustar \
    -C "$work_dir" "$artifact_name" | gzip --no-name --best > "$archive_path"
(
    cd -- "$output_dir"
    sha256sum -- "$(basename "$archive_path")"
) > "$archive_path.sha256"
printf 'source bundle: %s\n' "$archive_path"
printf 'source commit: %s\n' "$source_commit"
