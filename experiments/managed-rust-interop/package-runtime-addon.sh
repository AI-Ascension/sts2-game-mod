#!/usr/bin/env bash

set -euo pipefail

# Keep native linker metadata reproducible by default. A release may override this
# with the approved source timestamp, but an unset environment must not embed wall-clock time.
: "${SOURCE_DATE_EPOCH:=0}"
export SOURCE_DATE_EPOCH

platform=windows-x86_64
if [[ ${1:-} == '--platform' ]]; then
    [[ $# -eq 4 ]] || {
        printf 'usage: %s [--platform windows-x86_64|linux-x86_64] <sts2-data-directory> <output-directory>\n' "$0" >&2
        exit 2
    }
    platform=$2
    shift 2
fi

if [[ $# -ne 2 ]]; then
    printf 'usage: %s [--platform windows-x86_64|linux-x86_64] <sts2-data-directory> <output-directory>\n' "$0" >&2
    exit 2
fi

case "$platform" in
    windows-x86_64)
        native_target=x86_64-pc-windows-gnu
        native_file=ai_ascension_sts2_game_mod_native.dll
        native_output=AIAscensionSTS2GameModNative.dll
        ;;
    linux-x86_64)
        native_target=x86_64-unknown-linux-gnu
        native_file=libai_ascension_sts2_game_mod_native.so
        native_output=libAIAscensionSTS2GameModNative.so
        ;;
    *)
        printf 'unsupported platform: %s\n' "$platform" >&2
        exit 2
        ;;
esac

game_data_input=$1
output_dir=$2
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
managed_project="$repo_root/experiments/managed-rust-interop/game-loader/GameLoaderProbe.csproj"
native_manifest="$repo_root/experiments/managed-rust-interop/native/Cargo.toml"
manifest="$repo_root/experiments/managed-rust-interop/game-loader/mod_manifest.json"

case "$game_data_input" in
    [[:alpha:]]:[\\/]*|\\\\*) game_data_wsl=$(wslpath -u "$game_data_input") ;;
    *) game_data_wsl=$game_data_input ;;
esac

if [[ ! -f "$game_data_wsl/sts2.dll" || ! -f "$game_data_wsl/GodotSharp.dll" ]]; then
    printf 'STS2GameDataDir must contain sts2.dll and GodotSharp.dll: %s\n' "$game_data_input" >&2
    exit 1
fi
game_data_wsl=$(cd -- "$game_data_wsl" && pwd -P)

dotnet_command=${DOTNET_COMMAND:-dotnet}
if [[ -z ${DOTNET_COMMAND:-} ]] && ! command -v "$dotnet_command" >/dev/null 2>&1 \
    && [[ -x "/mnt/c/Program Files/dotnet/dotnet.exe" ]]; then
    dotnet_command="/mnt/c/Program Files/dotnet/dotnet.exe"
fi

if ! command -v "$dotnet_command" >/dev/null 2>&1 && [[ ! -x "$dotnet_command" ]]; then
    printf 'dotnet command is unavailable: %s\n' "$dotnet_command" >&2
    exit 1
fi

# MSBuild paths belong to the selected SDK, not to the game's target platform.
# Resolve aliases through symlinks so a `dotnet` link to `dotnet.exe` works too.
dotnet_resolved=$(command -v "$dotnet_command")
dotnet_resolved=$(readlink -f -- "$dotnet_resolved")
managed_project_msbuild=$managed_project
game_data_msbuild=$game_data_wsl
case "$dotnet_resolved" in
    *.[eE][xX][eE])
        managed_project_msbuild=$(wslpath -w "$managed_project")
        game_data_msbuild=$(wslpath -w "$game_data_wsl")
        ;;
esac

# Ask Cargo for its effective output root: environment and Cargo configuration
# may redirect it away from this checkout's target directory.
native_target_dir=$(cargo metadata --locked --offline --no-deps --format-version 1 \
    --manifest-path "$native_manifest" | jq -er '.target_directory | select(type == "string" and startswith("/"))')
managed_output_root="$native_target_dir/managed/$platform"
managed_build_artifact="$managed_output_root/Release/net9.0/AIAscensionSTS2GameMod.dll"
managed_output_msbuild=$managed_output_root
case "$dotnet_resolved" in
    *.[eE][xX][eE])
        managed_output_msbuild=$(wslpath -w "$managed_output_root")
        ;;
esac
native_build_artifact="$native_target_dir/$native_target/release/$native_file"
cargo build --locked --release --target "$native_target" --manifest-path "$native_manifest"
"$dotnet_command" restore "$managed_project_msbuild" -p:STS2GameDataDir="$game_data_msbuild" \
    -p:BaseOutputPath="$managed_output_msbuild/"
"$dotnet_command" build "$managed_project_msbuild" --configuration Release \
    -p:STS2GameDataDir="$game_data_msbuild" -p:BaseOutputPath="$managed_output_msbuild/" --no-restore

if [[ ! -f "$managed_build_artifact" || ! -f "$native_build_artifact" ]]; then
    printf 'build did not produce the expected %s addon artifacts\n' "$platform" >&2
    exit 1
fi

mkdir -p "$output_dir"
cp --remove-destination -- "$managed_build_artifact" "$output_dir/AIAscensionSTS2GameMod.dll"
cp --remove-destination -- "$native_build_artifact" "$output_dir/$native_output"
cp --remove-destination -- "$manifest" "$output_dir/AIAscensionSTS2GameMod.json"

sha256sum \
    "$output_dir/AIAscensionSTS2GameMod.dll" \
    "$output_dir/AIAscensionSTS2GameMod.json" \
    "$output_dir/$native_output"
