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
        case "$platform" in
            windows-x86_64) native_name=AIAscensionSTS2GameModNative.dll ;;
            linux-x86_64) native_name=libAIAscensionSTS2GameModNative.so ;;
            *) die "unsupported platform: $platform" ;;
        esac
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
case "$backup_dir/" in
    "$install_dir/"*) die 'backup directory must not be inside install directory' ;;
esac

readonly common_files=(
    AIAscensionSTS2GameMod.dll
    AIAscensionSTS2GameMod.json
)

require_plain_file() {
    local path=$1 label=$2
    [[ -f "$path" && ! -L "$path" ]] || die "$label must be a regular non-symlink file: $path"
}

require_plain_directory() {
    local path=$1 label=$2
    [[ -d "$path" && ! -L "$path" ]] || die "$label must be a regular non-symlink directory: $path"
}

if [[ "$operation" == install ]]; then
    package_dir=$(realpath -e -- "$package_arg") || die "package directory does not exist: $package_arg"
    require_plain_directory "$package_dir" package
    case "$package_dir/" in
        "$install_dir/"*) die 'package directory must not be inside install directory' ;;
    esac
    for file in "${common_files[@]}" "$native_name" "$MANIFEST_NAME" "$CHECKSUM_NAME"; do
        require_plain_file "$package_dir/$file" package
    done
    for entry in "$package_dir"/*; do
        name=${entry##*/}
        case "$name" in
            AIAscensionSTS2GameMod.dll|AIAscensionSTS2GameMod.json|"$native_name"|"$MANIFEST_NAME"|"$CHECKSUM_NAME") ;;
            *) die "package contains an unexpected entry: $name" ;;
        esac
        require_plain_file "$entry" package
    done

    python3 - "$package_dir/$MANIFEST_NAME" "$platform" "$native_name" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
platform = sys.argv[2]
native_name = sys.argv[3]
manifest = json.loads(manifest_path.read_text())
expected = ["AIAscensionSTS2GameMod.dll", "AIAscensionSTS2GameMod.json", native_name]
if manifest.get("platform") != platform:
    raise SystemExit(f"manifest platform does not match requested platform: {manifest.get('platform')!r}")
files = manifest.get("files")
if [item.get("path") for item in files or []] != expected:
    raise SystemExit("manifest file allowlist does not match requested platform")
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
    for file in "${common_files[@]}" "$native_name" "$MANIFEST_NAME" "$CHECKSUM_NAME"; do
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
    for file in "${common_files[@]}" "$native_name" "$MANIFEST_NAME" "$CHECKSUM_NAME"; do
        cp -- "$package_dir/$file" "$stage_dir/$file"
    done
    for file in "${common_files[@]}" "$native_name" "$MANIFEST_NAME" "$CHECKSUM_NAME"; do
        mv -f -- "$stage_dir/$file" "$install_dir/$file"
    done
    rmdir -- "$stage_dir"
    trap - ERR
    printf 'runtime addon %s installed at %s\n' "$platform" "$install_dir"
    printf 'rollback backup: %s\n' "$backup_dir"
else
    require_plain_directory "$install_dir" install
    require_plain_directory "$backup_dir" backup
    require_plain_file "$backup_dir/platform" backup platform
    for file in "${common_files[@]}" AIAscensionSTS2GameModNative.dll libAIAscensionSTS2GameModNative.so "$MANIFEST_NAME" "$CHECKSUM_NAME"; do
        state="$backup_dir/$file.state"
        [[ -f "$state" ]] || continue
        if [[ $(< "$state") == present ]]; then
            require_plain_file "$backup_dir/$file" backup
            cp -- "$backup_dir/$file" "$install_dir/.$file.rollback.$$"
            mv -f -- "$install_dir/.$file.rollback.$$" "$install_dir/$file"
        else
            rm -f -- "$install_dir/$file"
        fi
    done
    printf 'runtime addon rollback restored from %s\n' "$backup_dir"
fi
