#!/usr/bin/env bash
# Mutate owned source copies to establish that the source-linked ABI probe fails.
set -euo pipefail
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d)"
trap 'rm -rf -- "$test_root"' EXIT
mkdir -p "$test_root/queue-tests" "$test_root/game-loader"
cp "$repo_root/global.json" "$repo_root/Directory.Build.props" "$test_root/"
source_root="$repo_root/experiments/managed-rust-interop"
cp "$source_root/queue-tests/"*.cs "$source_root/queue-tests/RuntimeQueueProbe.csproj" "$test_root/queue-tests/"
for source in RuntimeDispatchQueue.cs RuntimeInterop.cs RuntimeInterop.Network.cs RuntimeInterop.Queue.cs RuntimeInterop.Types.cs; do
    cp "$source_root/game-loader/$source" "$test_root/game-loader/"
done
cd "$test_root"
dotnet run --project queue-tests/RuntimeQueueProbe.csproj --configuration Release > positive.log 2>&1
grep -Fq 'Managed ABI layout and 8 invalid/bounded buffer checks passed' positive.log
# Change the table's actual field offset while preserving compilability.
sed 's/public nint Request;/public nint Extra; public nint Request;/' \
    "$source_root/game-loader/RuntimeInterop.Types.cs" > game-loader/RuntimeInterop.Types.cs
if dotnet run --project queue-tests/RuntimeQueueProbe.csproj --configuration Release > negative.log 2>&1; then
    echo 'Source-linked probe accepted changed callback ABI' >&2; exit 1
fi
grep -Fq 'callback table width' negative.log
cp "$source_root/game-loader/RuntimeInterop.Types.cs" game-loader/RuntimeInterop.Types.cs
sed 's/CallingConvention.Cdecl/CallingConvention.StdCall/g' \
    "$source_root/game-loader/RuntimeInterop.cs" > game-loader/RuntimeInterop.cs
if dotnet run --project queue-tests/RuntimeQueueProbe.csproj --configuration Release > negative.log 2>&1; then
    echo 'Source-linked probe accepted changed calling convention' >&2; exit 1
fi
grep -Fq 'callback calling convention' negative.log
echo 'Managed ABI mutation gate: positive and 2 production-source negative cases passed'
