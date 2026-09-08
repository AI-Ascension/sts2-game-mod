#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -Eeuo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
launcher="$script_dir/live-combat-session.sh"
fixture_root=$(mktemp -d)
cleanup() { find "$fixture_root" -depth -delete; }
trap cleanup EXIT
mkdir -p "$fixture_root/host" "$fixture_root/bin" "$fixture_root/artifacts"
touch "$fixture_root/host/override.cfg"

cat > "$fixture_root/bin/provider" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == --describe ]]; then
    printf '%s\n' '{"kind":"ollama","provider":"ollama","model":"gemma4:31b-cloud"}'
fi
EOF
cat > "$fixture_root/bin/gateway" <<'EOF'
#!/usr/bin/env bash
trap 'exit 0' INT TERM
while :; do sleep 1; done
EOF
cat > "$fixture_root/bin/mcp" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
cat > "$fixture_root/bin/harness" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "STS2_COMBAT_DEMO=${STS2_COMBAT_DEMO:-}" "STS2_MAX_STEPS=${STS2_MAX_STEPS:-}" > "$FAKE_HARNESS_ENV_FILE"
printf '%s\n' '{"stage":"defeat"}'
EOF
cat > "$fixture_root/bin/powershell" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@" > "$FAKE_GUARDIAN_ARGS_FILE"
sleep 2
EOF
cat > "$fixture_root/bin/curl" <<'EOF'
#!/usr/bin/env bash
printf '200'
EOF
chmod +x "$fixture_root/bin"/*

base_args=(
    --host-dir "$fixture_root/host" --user-dir 'C:\\STS2\\test-user'
    --artifacts-dir "$fixture_root/artifacts"
    --gateway-binary "$fixture_root/bin/gateway" --mcp-binary "$fixture_root/bin/mcp"
    --harness-binary "$fixture_root/bin/harness" --provider-binary "$fixture_root/bin/provider"
    --powershell-binary "$fixture_root/bin/powershell" --hold-seconds 0
)

run_case() {
    local label=$1
    shift
    FAKE_GUARDIAN_ARGS_FILE="$fixture_root/$label.guardian" \
        FAKE_HARNESS_ENV_FILE="$fixture_root/$label.harness" \
        PATH="$fixture_root/bin:$PATH" \
        bash "$launcher" "${base_args[@]}" "$@" >"$fixture_root/$label.out" 2>"$fixture_root/$label.err"
}

has_arg() { grep -Fx -- "$1" "$2" >/dev/null; }
missing_arg() { ! has_arg "$1" "$2"; }

run_case campaign --run-kind campaign --campaign-mode standard --max-runtime-seconds 180
campaign_args="$fixture_root/campaign.guardian"
has_arg '-RunKind' "$campaign_args"
has_arg campaign "$campaign_args"
has_arg '-CampaignMode' "$campaign_args"
has_arg standard "$campaign_args"
has_arg '-MaxRuntimeSeconds' "$campaign_args"
has_arg 180 "$campaign_args"
missing_arg '-Seed' "$campaign_args"
campaign_run=$(awk '/^Artifacts: / { print $2; exit }' "$fixture_root/campaign.out")
jq -e '.run_kind == "campaign" and .campaign_mode == "standard" and .max_runtime_seconds == 180 and .seed == null' \
    "$campaign_run/manifest.json" >/dev/null
grep -Fx 'STS2_COMBAT_DEMO=false' "$fixture_root/campaign.harness" >/dev/null

run_case campaign_default --run-kind campaign --campaign-mode standard
campaign_default_args="$fixture_root/campaign_default.guardian"
has_arg '-MaxRuntimeSeconds' "$campaign_default_args"
has_arg 3600 "$campaign_default_args"
campaign_default_run=$(awk '/^Artifacts: / { print $2; exit }' "$fixture_root/campaign_default.out")
jq -e '.run_kind == "campaign" and .max_runtime_seconds == 3600' \
    "$campaign_default_run/manifest.json" >/dev/null

run_case demo --run-kind demo --seed DEMO1 --max-runtime-seconds 90
demo_args="$fixture_root/demo.guardian"
has_arg '-RunKind' "$demo_args"
has_arg demo "$demo_args"
has_arg '-Seed' "$demo_args"
has_arg DEMO1 "$demo_args"
has_arg '-MaxRuntimeSeconds' "$demo_args"
has_arg 90 "$demo_args"
missing_arg '-CampaignMode' "$demo_args"
demo_run=$(awk '/^Artifacts: / { print $2; exit }' "$fixture_root/demo.out")
jq -e '.run_kind == "demo" and .campaign_mode == null and .max_runtime_seconds == 90 and .seed == "DEMO1"' \
    "$demo_run/manifest.json" >/dev/null
grep -Fx 'STS2_COMBAT_DEMO=true' "$fixture_root/demo.harness" >/dev/null
grep -Fx 'STS2_MAX_STEPS=100' "$fixture_root/demo.harness" >/dev/null

run_case demo_default --run-kind demo
demo_default_args="$fixture_root/demo_default.guardian"
has_arg '-MaxRuntimeSeconds' "$demo_default_args"
has_arg 900 "$demo_default_args"
has_arg '-Seed' "$demo_default_args"
has_arg AIASCENSIONREPLAY1 "$demo_default_args"
demo_default_run=$(awk '/^Artifacts: / { print $2; exit }' "$fixture_root/demo_default.out")
jq -e '.run_kind == "demo" and .max_runtime_seconds == 900 and .seed == "AIASCENSIONREPLAY1"' \
    "$demo_default_run/manifest.json" >/dev/null

expect_rejected() {
    local label=$1
    shift
    set +e
    PATH="$fixture_root/bin:$PATH" bash "$launcher" "${base_args[@]}" "$@" \
        >"$fixture_root/$label.out" 2>"$fixture_root/$label.err"
    local status=$?
    set -e
    [[ $status -eq 2 ]] || { printf 'expected argument rejection for %s, got %s\n' "$label" "$status" >&2; exit 1; }
}

expect_rejected campaign_too_short --run-kind campaign --max-runtime-seconds 59
expect_rejected demo_too_long --run-kind demo --max-runtime-seconds 901
expect_rejected duration_overflow --run-kind campaign --max-runtime-seconds 999999999999999999999
expect_rejected demo_campaign_mode --run-kind demo --campaign-mode practice --seed DEMO1
printf 'PASS: campaign/demo guardian wiring, manifest identity, and bounded argument rejection\n'
