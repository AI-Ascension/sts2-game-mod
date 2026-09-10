#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$script_dir/../.." && pwd)
dotnet_command=${DOTNET:-dotnet}
project="$repo_root/experiments/managed-rust-interop/rest-action-tests/RuntimeV4ExpertRestActionProbe.csproj"
expected_dir="$repo_root/protocol-artifact/runtime-v4-expert-rest-action/producer"

if ! dotnet=$(command -v "$dotnet_command"); then
    printf 'required .NET SDK command was not found: %s\n' "$dotnet_command" >&2
    exit 1
fi
if [[ $# -gt 1 ]]; then
    printf 'usage: %s [capture-output-directory]\n' "${0##*/}" >&2
    exit 2
fi

build_root=$(mktemp -d -t sts2-rest-producer-capture-XXXXXXXX)
cleanup() {
    rm -rf -- "$build_root"
}
trap cleanup EXIT

if [[ $# == 1 ]]; then
    output_dir=$(realpath -m -- "$1")
    mkdir -p "$output_dir"
    for fixture in smith-selection-lifecycle.json mend-selection-lifecycle.json; do
        [[ ! -e "$output_dir/$fixture" && ! -L "$output_dir/$fixture" ]] || {
            printf 'refusing to overwrite existing capture file: %s\n' "$output_dir/$fixture" >&2
            exit 1
        }
    done
    compare=false
else
    output_dir="$build_root/fixtures"
    mkdir -p "$output_dir"
    compare=true
fi

mkdir -p "$build_root/dotnet-home" "$build_root/nuget" "$build_root/http-cache"
export DOTNET_CLI_HOME="$build_root/dotnet-home"
export NUGET_PACKAGES="$build_root/nuget"
export NUGET_HTTP_CACHE_PATH="$build_root/http-cache"
export DOTNET_CLI_TELEMETRY_OPTOUT=1

base_intermediate="$build_root/obj/"
msbuild_extensions="$build_root/obj/"
output_path="$build_root/out/"
"$dotnet" restore "$project" --packages "$NUGET_PACKAGES" \
    -p:BaseIntermediateOutputPath="$base_intermediate" \
    -p:MSBuildProjectExtensionsPath="$msbuild_extensions"
"$dotnet" build "$project" --configuration Release --no-restore \
    -p:BaseIntermediateOutputPath="$base_intermediate" \
    -p:MSBuildProjectExtensionsPath="$msbuild_extensions" \
    -p:OutputPath="$output_path"
STS2_REST_PRODUCER_CAPTURE_DIR="$output_dir" "$dotnet" run --project "$project" \
    --configuration Release --no-build --no-restore \
    -p:BaseIntermediateOutputPath="$base_intermediate" \
    -p:MSBuildProjectExtensionsPath="$msbuild_extensions" \
    -p:OutputPath="$output_path"

for fixture in smith-selection-lifecycle.json mend-selection-lifecycle.json; do
    [[ -s "$output_dir/$fixture" ]]
done

if [[ $compare == true ]]; then
    cmp "$output_dir/smith-selection-lifecycle.json" \
        "$expected_dir/smith-selection-lifecycle.json"
    cmp "$output_dir/mend-selection-lifecycle.json" \
        "$expected_dir/mend-selection-lifecycle.json"
    sha256sum "$output_dir"/*.json
    printf 'managed rest producer export and frozen-fixture regression passed\n'
else
    printf 'managed rest producer fixtures exported to %s\n' "$output_dir"
fi
