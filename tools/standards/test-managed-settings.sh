#!/usr/bin/env bash
# Actual MSBuild evaluation in an isolated repository; never build a host target.
set -euo pipefail
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d)"
trap 'rm -rf -- "$test_root"' EXIT
mkdir -p "$test_root/tools/standards"
cp "$repo_root/tools/standards/check-managed.sh" "$test_root/tools/standards/"
cp "$repo_root/Directory.Build.props" "$test_root/"
cp "$repo_root/global.json" "$test_root/"
printf 'Fixture.csproj\n' > "$test_root/tools/standards/managed-projects.txt"
git -C "$test_root" init -q
cat > "$test_root/Fixture.csproj" <<'PROJECT'
<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net9.0</TargetFramework></PropertyGroup></Project>
PROJECT
git -C "$test_root" add -- Fixture.csproj Directory.Build.props
bash "$test_root/tools/standards/check-managed.sh"
cp "$test_root/Fixture.csproj" "$test_root/Second.csproj"
printf 'Fixture.csproj\nSecond.csproj\n' > "$test_root/tools/standards/managed-projects.txt"
bash "$test_root/tools/standards/check-managed.sh"
rm -- "$test_root/Second.csproj"
if bash "$test_root/tools/standards/check-managed.sh" > "$test_root/output" 2>&1; then
    echo 'Accepted partially missing managed inventory' >&2; exit 1
fi
grep -Fq 'Managed project inventory differs' "$test_root/output"
printf 'Fixture.csproj\n' > "$test_root/tools/standards/managed-projects.txt"
for property in Nullable AnalysisLevel EnableNETAnalyzers TreatWarningsAsErrors Deterministic LangVersion EnforceCodeStyleInBuild; do
    case "$property" in
        Nullable) value=disable ;;
        AnalysisLevel) value=none ;;
        LangVersion) value=11.0 ;;
        *) value=false ;;
    esac
    cat > "$test_root/Fixture.csproj" <<PROJECT
<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net9.0</TargetFramework></PropertyGroup><PropertyGroup Condition="'\$(Configuration)' == 'Release'"><$property>$value</$property></PropertyGroup></Project>
PROJECT
    if bash "$test_root/tools/standards/check-managed.sh" > "$test_root/output" 2>&1; then
        printf 'Accepted weakened %s\n' "$property" >&2
        exit 1
    fi
    grep -Fq 'Managed settings rejected: Fixture.csproj (Release)' "$test_root/output"
done
cat > "$test_root/Fixture.csproj" <<'PROJECT'
<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net9.0</TargetFramework></PropertyGroup></Project>
PROJECT
printf 'class Fixture{void Work(){int value=1;}}\n' > "$test_root/Fixture.cs"
if bash "$test_root/tools/standards/check-managed.sh" format > "$test_root/output" 2>&1; then
    echo 'Accepted invalid C# formatting' >&2; exit 1
fi
grep -Fq 'WHITESPACE' "$test_root/output"
grep -Fq 'class Fixture{void Work(){int value=1;}}' "$test_root/Fixture.cs"
mv -- "$test_root/Fixture.cs" "$test_root/outside-source"
ln -s -- "$test_root/outside-source" "$test_root/Fixture.cs"
if bash "$test_root/tools/standards/check-managed.sh" format > "$test_root/output" 2>&1; then
    echo 'Accepted linked formatting input' >&2; exit 1
fi
grep -Fq 'Missing or linked managed formatting input' "$test_root/output"
rm -- "$test_root/Fixture.cs"
rm -- "$test_root/Fixture.csproj"
if bash "$test_root/tools/standards/check-managed.sh" > "$test_root/output" 2>&1; then
    echo 'Accepted missing project' >&2; exit 1
fi
grep -Fq 'Missing or linked managed project' "$test_root/output"
git -C "$test_root" rm --cached -q -- Fixture.csproj
if bash "$test_root/tools/standards/check-managed.sh" > "$test_root/output" 2>&1; then
    echo 'Accepted zero projects' >&2; exit 1
fi
grep -Fq 'No managed projects found' "$test_root/output"
echo 'Managed settings: positive inheritance and 12 negative cases passed; format check preserved source'
