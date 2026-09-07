#!/usr/bin/env bash
# Evaluate every real project without building or loading proprietary host code.
set -euo pipefail
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"
mode="${1:-settings}"
case "$mode" in settings|format) ;; *) echo 'usage: check-managed.sh [settings|format]' >&2; exit 2 ;; esac
command -v dotnet >/dev/null
command -v jq >/dev/null
[[ $(dotnet --version) == 9.0.317 ]] || { echo 'Required SDK: 9.0.317' >&2; exit 1; }
mapfile -d '' -t projects < <(git ls-files -z --cached --others --exclude-standard -- '*.csproj')
(( ${#projects[@]} > 0 )) || { echo 'No managed projects found' >&2; exit 1; }
inventory=tools/standards/managed-projects.txt
[[ -f "$inventory" && ! -L "$inventory" ]] || { echo 'Missing managed project inventory' >&2; exit 1; }
if ! diff -u "$inventory" <(printf '%s\n' "${projects[@]}" | LC_ALL=C sort -u); then
    echo 'Managed project inventory differs; review entrypoint additions and removals' >&2
    exit 1
fi
for project in "${projects[@]}"; do
    [[ -f "$project" && ! -L "$project" && $(realpath -- "$project") == "$repo_root/$project" ]] || {
        echo 'Missing or linked managed project' >&2; exit 1;
    }
    for configuration in Debug Release; do
        settings="$(dotnet msbuild "$project" -nologo -property:Configuration="$configuration" \
            -getProperty:Nullable,AnalysisLevel,EnableNETAnalyzers,TreatWarningsAsErrors,Deterministic,LangVersion,EnforceCodeStyleInBuild)"
        if ! jq -e '.Properties | .Nullable == "enable" and .AnalysisLevel == "9.0-recommended"
            and .EnableNETAnalyzers == "true" and .TreatWarningsAsErrors == "true"
            and .Deterministic == "true" and .LangVersion == "12.0"
            and .EnforceCodeStyleInBuild == "true"' <<< "$settings" >/dev/null; then
            printf 'Managed settings rejected: %s (%s)\n' "$project" "$configuration" >&2
            exit 1
        fi
    done
done
printf 'Evaluated settings: %s projects, Debug and Release\n' "${#projects[@]}"
if [[ "$mode" == format ]]; then
    mapfile -d '' -t sources < <(git ls-files -z --cached --others --exclude-standard -- '*.cs')
    (( ${#sources[@]} > 0 )) || { echo 'No managed source files found' >&2; exit 1; }
    mapfile -d '' -t configs < <(git ls-files -z --cached --others --exclude-standard -- \
        '.editorconfig' '*/.editorconfig' 'global.json')
    format_root="$(mktemp -d)"
    trap 'rm -rf -- "$format_root"' EXIT
    for source in "${sources[@]}" "${configs[@]}"; do
        [[ -f "$source" && ! -L "$source" && $(realpath -- "$source") == "$repo_root/$source" ]] || {
            echo 'Missing or linked managed formatting input' >&2; exit 1;
        }
        mkdir -p -- "$format_root/$(dirname -- "$source")"
        cp -- "$source" "$format_root/$source"
    done
    # The folder workspace sees only admitted source/config copies, never ignored
    # output trees or unrelated symlink directories in the developer checkout.
    (cd "$format_root" && dotnet format whitespace --folder --include "${sources[@]}" --verify-no-changes)
    printf 'Checked whitespace: %s admitted C# files\n' "${#sources[@]}"
fi
