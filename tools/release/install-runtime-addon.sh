#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -Eeuo pipefail

readonly SCRIPT_NAME=$(basename "$0")
readonly MANIFEST_NAME=sts2-workshop-manifest.json
readonly CHECKSUM_NAME=SHA256SUMS

die() {
    printf '%s: %s\n' "$SCRIPT_NAME" "$1" >&2
    exit 1
}

usage() {
    printf 'usage: %s install <platform> <package-directory> <install-directory> <backup-directory>\n' "$SCRIPT_NAME" >&2
    printf '       %s rollback <install-directory> <backup-directory>\n' "$SCRIPT_NAME" >&2
    exit 2
}

[[ $# -ge 1 ]] || usage
operation=$1
shift

case "$operation" in
    install)
        [[ $# -eq 4 ]] || usage
        platform=$1
        package_arg=$2
        install_arg=$3
        backup_arg=$4
        ;;
    rollback)
        [[ $# -eq 2 ]] || usage
        package_arg=''
        platform=''
        install_arg=$1
        backup_arg=$2
        native_name=''
        ;;
    *) usage ;;
esac

export LC_ALL=C
install_dir=$(realpath -m -- "$install_arg") || die "could not resolve install directory: $install_arg"
backup_dir=$(realpath -m -- "$backup_arg") || die "could not resolve backup directory: $backup_arg"
[[ "$install_dir" != "/" && "$backup_dir" != "/" ]] || die 'root directory is not a valid install or backup target'

readonly common_files=(
    AIAscensionSTS2GameMod.dll
    AIAscensionSTS2GameMod.json
)

set_platform_contract() {
    case "$1" in
        windows-x86_64) native_name=AIAscensionSTS2GameModNative.dll ;;
        linux-x86_64) native_name=libAIAscensionSTS2GameModNative.so ;;
        *) die "unsupported platform: $1" ;;
    esac
    package_files=("${common_files[@]}" "$native_name" "$MANIFEST_NAME" "$CHECKSUM_NAME")
}

is_same_or_below() {
    case "$1/" in
        "$2/"*) return 0 ;;
        *) return 1 ;;
    esac
}

require_plain_file() {
    local path=$1 label=$2
    [[ -f "$path" && ! -L "$path" ]] || die "$label must be a regular non-symlink file: $path"
}

require_plain_directory() {
    local path=$1 label=$2
    [[ -d "$path" && ! -L "$path" ]] || die "$label must be a regular non-symlink directory: $path"
}

if [[ "$operation" == install ]]; then
    set_platform_contract "$platform"
    package_dir=$(realpath -e -- "$package_arg") || die "package directory does not exist: $package_arg"
    require_plain_directory "$package_dir" package
    is_same_or_below "$package_dir" "$install_dir" && die 'package directory must not be inside install directory'
    is_same_or_below "$install_dir" "$package_dir" && die 'install directory must not be inside package directory'
    is_same_or_below "$backup_dir" "$package_dir" && die 'backup directory must not be inside package directory'
    is_same_or_below "$package_dir" "$backup_dir" && die 'package directory must not be inside backup directory'
    is_same_or_below "$install_dir" "$backup_dir" && die 'install directory must not be inside backup directory'
    is_same_or_below "$backup_dir" "$install_dir" && die 'backup directory must not be inside install directory'
    for file in "${package_files[@]}"; do
        require_plain_file "$package_dir/$file" package
    done
    while IFS= read -r -d '' entry; do
        name=${entry##*/}
        allowed=false
        for file in "${package_files[@]}"; do
            [[ "$name" == "$file" ]] && allowed=true
        done
        [[ "$allowed" == true ]] || die "package contains an unexpected entry: $name"
        require_plain_file "$entry" package
    done < <(find "$package_dir" -mindepth 1 -maxdepth 1 -print0)

    python3 - "$package_dir" "$platform" "$native_name" "$MANIFEST_NAME" "$CHECKSUM_NAME" <<'PY'
import json
import pathlib
import re
import sys
import hashlib

package_dir = pathlib.Path(sys.argv[1])
platform = sys.argv[2]
native_name = sys.argv[3]
manifest_path = package_dir / sys.argv[4]
checksum_path = package_dir / sys.argv[5]

def reject_duplicate_properties(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate manifest property")
        result[key] = value
    return result

manifest = json.loads(manifest_path.read_text(), object_pairs_hook=reject_duplicate_properties)
expected = ["AIAscensionSTS2GameMod.dll", "AIAscensionSTS2GameMod.json", native_name]
if manifest.get("platform") != platform:
    raise SystemExit(f"manifest platform does not match requested platform: {manifest.get('platform')!r}")
files = manifest.get("files")
paths = [item.get("path") if isinstance(item, dict) else None for item in files] if isinstance(files, list) else []
if paths != expected:
    raise SystemExit("manifest file allowlist does not match requested platform")
roles = ["managed_assembly", "loader_manifest", "native_library"]
for item, path, role in zip(files, expected, roles):
    if (not isinstance(item, dict) or item.get("role") != role
            or type(item.get("size_bytes")) is not int or item["size_bytes"] <= 0
            or not isinstance(item.get("sha256"), str)
            or re.fullmatch(r"[0-9a-f]{64}", item["sha256"]) is None):
        raise SystemExit("manifest payload inventory is malformed")
    payload = package_dir / path
    if payload.stat().st_size != item["size_bytes"] or item["size_bytes"] > 256 * 1024 * 1024:
        raise SystemExit(f"manifest size does not match payload: {path}")
    digest_builder = hashlib.sha256()
    with payload.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest_builder.update(chunk)
    digest = digest_builder.hexdigest()
    if digest != item["sha256"]:
        raise SystemExit(f"manifest digest does not match payload: {path}")

canonical = "".join(
    f"{item['path']}\t{item['size_bytes']}\t{item['sha256']}\n" for item in files
).encode()
if manifest.get("content_digest") != hashlib.sha256(canonical).hexdigest():
    raise SystemExit("manifest content digest does not match payload inventory")

expected_checksum_names = expected + [sys.argv[4]]
lines = checksum_path.read_text().splitlines()
if len(lines) != len(expected_checksum_names):
    raise SystemExit("checksum inventory is not complete")
seen = []
for line in lines:
    match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9._-]+)", line)
    if match is None or match.group(2) not in expected_checksum_names:
        raise SystemExit("checksum inventory contains an unsafe path")
    seen.append(match.group(2))
if seen != expected_checksum_names or len(set(seen)) != len(seen):
    raise SystemExit("checksum inventory does not exactly cover the package")
PY
    (cd "$package_dir" && sha256sum --check --strict "$CHECKSUM_NAME") >/dev/null \
        || die 'package checksum inventory failed'

    [[ ! -e "$backup_dir" && ! -L "$backup_dir" ]] || die "backup directory already exists: $backup_dir"
    if [[ -e "$install_dir" || -L "$install_dir" ]]; then
        require_plain_directory "$install_dir" install
    else
        mkdir -p -- "$install_dir"
    fi
    other_native_name=AIAscensionSTS2GameModNative.dll
    [[ "$native_name" == "$other_native_name" ]] && other_native_name=libAIAscensionSTS2GameModNative.so
    [[ ! -e "$install_dir/$other_native_name" && ! -L "$install_dir/$other_native_name" ]] \
        || die "install contains a native library for another platform: $other_native_name"
    mkdir -p -- "$backup_dir"
    for file in "${package_files[@]}"; do
        target="$install_dir/$file"
        [[ ! -L "$target" ]] || die "install target is a symlink: $target"
        if [[ -e "$target" ]]; then
            [[ -f "$target" ]] || die "install target is not a regular file: $target"
            cp -- "$target" "$backup_dir/$file"
            printf 'present\n' > "$backup_dir/$file.state"
        else
            printf 'missing\n' > "$backup_dir/$file.state"
        fi
    done
    printf '%s\n' "$platform" > "$backup_dir/platform"

    stage_dir=$(mktemp -d "$install_dir/.ai-ascension-runtime.XXXXXX")
    rollback_install() {
        trap - ERR
        for file in "${common_files[@]}" "$native_name" "$MANIFEST_NAME" "$CHECKSUM_NAME"; do
            state="$backup_dir/$file.state"
            if [[ -f "$state" ]] && [[ $(< "$state") == present ]]; then
                cp -- "$backup_dir/$file" "$install_dir/.$file.rollback.$$" || true
                mv -f -- "$install_dir/.$file.rollback.$$" "$install_dir/$file" || true
            else
                rm -f -- "$install_dir/$file" || true
            fi
        done
        rm -rf -- "$stage_dir" || true
    }
    trap rollback_install ERR
    for file in "${package_files[@]}"; do
        cp -- "$package_dir/$file" "$stage_dir/$file"
    done
    for file in "${package_files[@]}"; do
        mv -f -- "$stage_dir/$file" "$install_dir/$file"
    done
    rmdir -- "$stage_dir"
    trap - ERR
    printf 'runtime addon %s installed at %s\n' "$platform" "$install_dir"
    printf 'rollback backup: %s\n' "$backup_dir"
else
    require_plain_directory "$install_dir" install
    require_plain_directory "$backup_dir" backup
    is_same_or_below "$backup_dir" "$install_dir" && die 'backup directory must not be inside install directory'
    is_same_or_below "$install_dir" "$backup_dir" && die 'install directory must not be inside backup directory'
    require_plain_file "$backup_dir/platform" backup
    if cmp -s "$backup_dir/platform" <(printf 'windows-x86_64\n'); then
        backup_platform=windows-x86_64
    elif cmp -s "$backup_dir/platform" <(printf 'linux-x86_64\n'); then
        backup_platform=linux-x86_64
    else
        die 'backup platform marker is malformed'
    fi
    set_platform_contract "$backup_platform"

    expected_backup_entries=(platform)
    for file in "${package_files[@]}"; do
        expected_backup_entries+=("$file" "$file.state")
    done
    while IFS= read -r -d '' entry; do
        name=${entry##*/}
        allowed=false
        for file in "${expected_backup_entries[@]}"; do
            [[ "$name" == "$file" ]] && allowed=true
        done
        [[ "$allowed" == true ]] || die "backup contains an unexpected entry: $name"
        require_plain_file "$entry" backup
    done < <(find "$backup_dir" -mindepth 1 -maxdepth 1 -print0)
    for file in "${package_files[@]}"; do
        state="$backup_dir/$file.state"
        require_plain_file "$state" backup
        if cmp -s "$state" <(printf 'present\n'); then
            require_plain_file "$backup_dir/$file" backup
        elif cmp -s "$state" <(printf 'missing\n'); then
            [[ ! -e "$backup_dir/$file" && ! -L "$backup_dir/$file" ]] \
                || die "missing-state backup contains payload bytes: $file"
        else
            die "backup state is malformed: $file.state"
        fi
        target="$install_dir/$file"
        [[ ! -L "$target" ]] || die "install target is a symlink: $target"
        temporary="$install_dir/.$file.rollback.$$"
        [[ ! -e "$temporary" && ! -L "$temporary" ]] || die "rollback temporary path already exists: $temporary"
    done

    for file in "${package_files[@]}"; do
        state="$backup_dir/$file.state"
        if cmp -s "$state" <(printf 'present\n'); then
            cp -- "$backup_dir/$file" "$install_dir/.$file.rollback.$$"
            mv -f -- "$install_dir/.$file.rollback.$$" "$install_dir/$file"
        else
            rm -f -- "$install_dir/$file"
        fi
    done
    printf 'runtime addon rollback restored from %s\n' "$backup_dir"
fi
