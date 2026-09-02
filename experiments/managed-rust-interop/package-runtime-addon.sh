#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 2 ]]; then
    printf 'usage: %s <sts2-data-directory> <output-directory>\n' "$0" >&2
    exit 2
fi

game_data_input=$1
output_dir=$2
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
managed_project="$repo_root/experiments/managed-rust-interop/game-loader/GameLoaderProbe.csproj"
managed_project_msbuild=$(wslpath -w "$managed_project")
native_manifest="$repo_root/experiments/managed-rust-interop/native/Cargo.toml"
managed_build_artifact="$repo_root/experiments/managed-rust-interop/game-loader/bin/Release/net9.0/AIAscensionSTS2GameMod.dll"
native_build_artifact="$repo_root/target/x86_64-pc-windows-gnu/release/ai_ascension_sts2_game_mod_native.dll"
manifest="$repo_root/experiments/managed-rust-interop/game-loader/mod_manifest.json"

if [[ "$game_data_input" == /* ]]; then
    game_data_wsl=$game_data_input
    game_data_msbuild=$(wslpath -w "$game_data_wsl")
else
    game_data_msbuild=$game_data_input
    game_data_wsl=$(wslpath -u "$game_data_msbuild")
fi

if [[ ! -f "$game_data_wsl/sts2.dll" || ! -f "$game_data_wsl/GodotSharp.dll" ]]; then
    printf 'STS2GameDataDir must contain sts2.dll and GodotSharp.dll: %s\n' "$game_data_input" >&2
    exit 1
fi

dotnet_command=${DOTNET_COMMAND:-dotnet}
if ! command -v "$dotnet_command" >/dev/null 2>&1 && [[ -x "/mnt/c/Program Files/dotnet/dotnet.exe" ]]; then
    dotnet_command="/mnt/c/Program Files/dotnet/dotnet.exe"
fi
if ! command -v "$dotnet_command" >/dev/null 2>&1 && [[ ! -x "$dotnet_command" ]]; then
    printf 'dotnet command is unavailable: %s\n' "$dotnet_command" >&2
    exit 1
fi

cargo build --locked --release --target x86_64-pc-windows-gnu --manifest-path "$native_manifest"
"$dotnet_command" restore "$managed_project_msbuild" -p:STS2GameDataDir="$game_data_msbuild"
"$dotnet_command" build "$managed_project_msbuild" --configuration Release \
    -p:STS2GameDataDir="$game_data_msbuild" --no-restore

if [[ ! -f "$managed_build_artifact" || ! -f "$native_build_artifact" ]]; then
    printf 'build did not produce the expected Windows addon artifacts\n' >&2
    exit 1
fi

mkdir -p "$output_dir"
cp "$managed_build_artifact" "$output_dir/AIAscensionSTS2GameMod.dll"
cp "$native_build_artifact" "$output_dir/AIAscensionSTS2GameModNative.dll"
cp "$manifest" "$output_dir/AIAscensionSTS2GameMod.json"

sha256sum \
    "$output_dir/AIAscensionSTS2GameMod.dll" \
    "$output_dir/AIAscensionSTS2GameMod.json" \
    "$output_dir/AIAscensionSTS2GameModNative.dll"
