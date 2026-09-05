#!/usr/bin/env bash
set -Eeuo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
source "$script_dir/session-launcher.sh"
fixture_root=$(mktemp -d)
trap 'rm -rf -- "$fixture_root"' EXIT
mkdir -p "$fixture_root/bin" "$fixture_root/provider/target/debug" "$fixture_root/alternate/debug"
cp "$script_dir/provider-build-fixture.sh" "$fixture_root/bin/cargo"
chmod u+x "$fixture_root/bin/cargo"
cp /bin/false "$fixture_root/provider/target/debug/sts2-gateway-runtime"
cp /bin/true "$fixture_root/alternate/debug/sts2-gateway-runtime"
export PATH="$fixture_root/bin:$PATH"
export CARGO_TARGET_DIR="$fixture_root/alternate"
export STS2_PROVIDER_FIXTURE_EXECUTABLE="$fixture_root/alternate/debug/sts2-gateway-runtime"
live_authorization_deadline=$((EPOCHSECONDS + 30))
selected=$(build_provider_binary "$fixture_root/provider" sts2-gateway-runtime)
[[ "$selected" == "$STS2_PROVIDER_FIXTURE_EXECUTABLE" ]] || exit 1
"$selected" # The stale default executable would fail.
printf 'PASS: provider executable comes from Cargo output, not stale default target\n'
