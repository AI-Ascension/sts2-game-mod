#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

# Symlinks to this script act as fake tools. No SDK, cargo build, or game is run.
case ${0##*/} in
    wslpath)
        printf '%s\n' "$1" >> "$PACKAGE_TEST_ROOT/conversions"
        if [[ $1 == -u ]]; then
            printf '%s\n' "$PACKAGE_TEST_DATA"
        else
            printf 'WINDOWS::%s\n' "${@: -1}"
        fi
        exit 0 ;;
    cargo)
        target=${CARGO_TARGET_DIR:-${PACKAGE_TEST_CONFIG_TARGET:-"$PACKAGE_TEST_ROOT/target"}}
        [[ $target == /* ]] || target="$PWD/$target"
        if [[ $1 == metadata ]]; then
            if [[ ${PACKAGE_TEST_RECORD_CARGO_HOME:-false} == true ]]; then
                printf '%s\n' "${CARGO_HOME-}" >> "$PACKAGE_TEST_ROOT/cargo-home-values"
            fi
            jq -n --arg target "$target" '{target_directory: $target}'
            exit 0
        fi
        [[ $1 == build ]]
        if [[ ${PACKAGE_TEST_RECORD_CARGO_HOME:-false} == true ]]; then
            printf '%s\n' "${CARGO_HOME-}" >> "$PACKAGE_TEST_ROOT/cargo-home-values"
        fi
        native="$target/x86_64-pc-windows-gnu/release"
        mkdir -p "$native"
        printf 'synthetic native' > "$native/ai_ascension_sts2_game_mod_native.dll"
        exit 0 ;;
    dotnet|dotnet.exe)
        printf '%s\0' "$@" > "$PACKAGE_TEST_ROOT/$1.args"
        if [[ $1 == build ]]; then
            managed="$PACKAGE_TEST_ROOT/experiments/managed-rust-interop/game-loader/bin/Release/net9.0"
            mkdir -p "$managed"
            printf 'synthetic managed' > "$managed/AIAscensionSTS2GameMod.dll"
        fi
        exit 0 ;;
esac

test_script=$(realpath -- "${BASH_SOURCE[0]}")
source_dir=${test_script%/*}
test_root=$(mktemp -d -t sts2-package-paths-XXXXXXXX)
trap 'rm -rf -- "$test_root"' EXIT
original_path=$PATH

setup_case() {
    export PACKAGE_TEST_ROOT="$test_root/$1/fake repository"
    export PACKAGE_TEST_DATA="$PACKAGE_TEST_ROOT/fake host data"
    loader="$PACKAGE_TEST_ROOT/experiments/managed-rust-interop/game-loader"
    mkdir -p "$loader" "$PACKAGE_TEST_DATA" "$PACKAGE_TEST_ROOT/tools"
    cp "$source_dir/package-runtime-addon.sh" "$loader/../package-runtime-addon.sh"
    cp "$source_dir/build-native-release.sh" "$loader/../build-native-release.sh"
    chmod +x "$loader/../build-native-release.sh"
    printf '{}' > "$loader/mod_manifest.json"
    printf 'synthetic reference' > "$PACKAGE_TEST_DATA/sts2.dll"
    printf 'synthetic reference' > "$PACKAGE_TEST_DATA/GodotSharp.dll"
    # Copy rather than link dotnet.exe: production resolves links to classify the SDK.
    cp "$test_script" "$PACKAGE_TEST_ROOT/tools/dotnet.exe"
    chmod +x "$PACKAGE_TEST_ROOT/tools/dotnet.exe"
    cp "$test_script" "$PACKAGE_TEST_ROOT/tools/dotnet"
    chmod +x "$PACKAGE_TEST_ROOT/tools/dotnet"
    for tool in cargo wslpath; do
        ln -s "$test_script" "$PACKAGE_TEST_ROOT/tools/$tool"
    done
    export PATH="$PACKAGE_TEST_ROOT/tools:$original_path"
    unset DOTNET_COMMAND CARGO_TARGET_DIR PACKAGE_TEST_CONFIG_TARGET CARGO_HOME PACKAGE_TEST_RECORD_CARGO_HOME
}

check_case() {
    local input=$1 prefix=$2 expected_conversions=$3 phase found
    (cd "$PACKAGE_TEST_ROOT" && bash "$loader/../package-runtime-addon.sh" \
        "$input" "$PACKAGE_TEST_ROOT/output") > "$PACKAGE_TEST_ROOT/result"
    for phase in restore build; do
        mapfile -d '' -t args < "$PACKAGE_TEST_ROOT/$phase.args"
        [[ ${args[0]} == "$phase" ]]
        [[ ${args[1]} == "$prefix$loader/GameLoaderProbe.csproj" ]]
        found=false
        for arg in "${args[@]}"; do
            if [[ $arg == "-p:STS2GameDataDir=$prefix$PACKAGE_TEST_DATA" ]]; then found=true; fi
        done
        [[ $found == true ]]
    done
    if [[ $expected_conversions == 0 ]]; then
        [[ ! -e "$PACKAGE_TEST_ROOT/conversions" ]]
    else
        [[ $(wc -l < "$PACKAGE_TEST_ROOT/conversions") == "$expected_conversions" ]]
    fi
    [[ $(find "$PACKAGE_TEST_ROOT/output" -type f | wc -l) == 3 ]]
    for artifact in AIAscensionSTS2GameMod.dll AIAscensionSTS2GameMod.json AIAscensionSTS2GameModNative.dll; do
        [[ -s "$PACKAGE_TEST_ROOT/output/$artifact" ]]
    done
    [[ $(< "$PACKAGE_TEST_ROOT/output/AIAscensionSTS2GameModNative.dll") == 'synthetic native' ]]
}

check_relative_caller_case() {
    local phase found
    mkdir -p "$PACKAGE_TEST_ROOT/caller"
    export DOTNET_COMMAND='../tools/dotnet'
    (cd "$PACKAGE_TEST_ROOT/caller" && bash "$loader/../package-runtime-addon.sh" \
        '../fake host data' '../output') > "$PACKAGE_TEST_ROOT/result"
    for phase in restore build; do
        mapfile -d '' -t args < "$PACKAGE_TEST_ROOT/$phase.args"
        [[ ${args[0]} == "$phase" ]]
        [[ ${args[1]} == "$loader/GameLoaderProbe.csproj" ]]
        found=false
        for arg in "${args[@]}"; do
            if [[ $arg == "-p:STS2GameDataDir=$PACKAGE_TEST_DATA" ]]; then found=true; fi
        done
        [[ $found == true ]]
    done
    [[ $(find "$PACKAGE_TEST_ROOT/output" -type f | wc -l) == 3 ]]
    [[ $(< "$PACKAGE_TEST_ROOT/output/AIAscensionSTS2GameModNative.dll") == 'synthetic native' ]]
}

check_relative_helper_cache_case() {
    local expected
    mkdir -p "$PACKAGE_TEST_ROOT/helper-caller/relative cargo home"
    export CARGO_HOME='relative cargo home'
    export PACKAGE_TEST_RECORD_CARGO_HOME=true
    (cd "$PACKAGE_TEST_ROOT/helper-caller" && bash "$loader/../build-native-release.sh" \
        windows-x86_64) > "$PACKAGE_TEST_ROOT/helper-result"
    expected="$PACKAGE_TEST_ROOT/helper-caller/relative cargo home"
    mapfile -t seen < "$PACKAGE_TEST_ROOT/cargo-home-values"
    [[ ${#seen[@]} == 2 ]]
    [[ ${seen[0]} == "$expected" && ${seen[1]} == "$expected" ]]
    [[ $(< "$PACKAGE_TEST_ROOT/helper-result") == \
        "$PACKAGE_TEST_ROOT/target/x86_64-pc-windows-gnu/release/ai_ascension_sts2_game_mod_native.dll" ]]
}

stale_default_artifact() {
    local stale="$PACKAGE_TEST_ROOT/target/x86_64-pc-windows-gnu/release"
    mkdir -p "$stale"
    printf 'stale default binary' > "$stale/ai_ascension_sts2_game_mod_native.dll"
}

setup_case native-absolute
rm -- "$PACKAGE_TEST_ROOT/tools/wslpath"
check_case "$PACKAGE_TEST_DATA" '' 0
setup_case native-relative
check_case 'fake host data' '' 0
setup_case native-windows-input
check_case 'C:\Fake Host' '' 1
setup_case native-unc-input
check_case '\\server\Fake Host' '' 1
setup_case windows-sdk
export DOTNET_COMMAND="$PACKAGE_TEST_ROOT/tools/dotnet.exe"
check_case "$PACKAGE_TEST_DATA" 'WINDOWS::' 2
setup_case windows-symlink
rm -- "$PACKAGE_TEST_ROOT/tools/dotnet"
ln -s dotnet.exe "$PACKAGE_TEST_ROOT/tools/dotnet"
check_case "$PACKAGE_TEST_DATA" 'WINDOWS::' 2
setup_case redirected-absolute-target
export CARGO_TARGET_DIR="$test_root/alternate cargo output"
stale_default_artifact
check_case "$PACKAGE_TEST_DATA" '' 0
setup_case redirected-relative-target
export CARGO_TARGET_DIR='relative cargo output'
stale_default_artifact
check_case "$PACKAGE_TEST_DATA" '' 0
setup_case cargo-config-target
export PACKAGE_TEST_CONFIG_TARGET="$test_root/configured cargo output"
stale_default_artifact
check_case "$PACKAGE_TEST_DATA" '' 0
setup_case relative-caller
check_relative_caller_case
setup_case relative-helper-cache
check_relative_helper_cache_case
setup_case missing-sdk
export DOTNET_COMMAND="$PACKAGE_TEST_ROOT/tools/missing-dotnet"
if bash "$loader/../package-runtime-addon.sh" "$PACKAGE_TEST_DATA" "$PACKAGE_TEST_ROOT/output" \
    > "$PACKAGE_TEST_ROOT/error" 2>&1; then
    printf 'missing SDK unexpectedly accepted\n' >&2
    exit 1
fi
[[ $(< "$PACKAGE_TEST_ROOT/error") == *'dotnet command is unavailable'* ]]
[[ ! -e "$PACKAGE_TEST_ROOT/restore.args" && ! -e "$PACKAGE_TEST_ROOT/output" ]]
printf 'package-runtime-addon: 12 synthetic path-selection and regression tests passed\n'
