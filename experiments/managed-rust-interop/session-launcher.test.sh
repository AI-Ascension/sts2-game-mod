#!/usr/bin/env bash

set -Eeuo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
launcher="$script_dir/session-launcher.sh"
fixture="$script_dir/session-launcher-fixture.sh"
source "$launcher"

fail() {
    printf 'session launcher test failed: %s\n' "$1" >&2
    exit 1
}

runtime_token=$(new_credential) || fail 'runtime CSPRNG'
gateway_token=$(new_credential) || fail 'gateway CSPRNG'
credential_is_safe "$runtime_token" || fail 'runtime encoding or length'
credential_is_safe "$gateway_token" || fail 'gateway encoding or length'
[[ "$runtime_token" != "$gateway_token" ]] || fail 'credential reuse'

gateway_addr=127.0.0.1:15525
mod_addr=127.0.0.1:15526
mcp_binary=/bin/true
export STS2_EXPECTED_RUNTIME="$runtime_token" STS2_EXPECTED_GATEWAY="$gateway_token"
run_gateway_with_credentials bash -c '
    [[ -z "${STS2_RUNTIME_TOKEN:-}" ]] \
        && [[ "$STS2_MOD_TOKEN" == "$STS2_EXPECTED_RUNTIME" ]] \
        && [[ "$STS2_GATEWAY_TOKEN" == "$STS2_EXPECTED_GATEWAY" ]]
' || fail 'gateway role mapping'
run_harness_with_credentials bash -c '
    [[ "$STS2_GATEWAY_TOKEN" == "$STS2_EXPECTED_GATEWAY" ]] \
        && [[ -z "${STS2_RUNTIME_TOKEN:-}" ]] \
        && [[ -z "${STS2_MOD_TOKEN:-}" ]]
' || fail 'harness role mapping'
unset STS2_EXPECTED_RUNTIME STS2_EXPECTED_GATEWAY

run_gateway_with_credentials sleep 2 &
argument_probe_pid=$!
for ((attempt = 0; attempt < 20; attempt++)); do
    if ps -p "$argument_probe_pid" >/dev/null 2>&1; then
        break
    fi
    sleep 0.05
done
if argument_probe=$(ps -o args= -p "$argument_probe_pid" 2>/dev/null); then
    :
else
    argument_probe=''
fi
[[ "$argument_probe" != *"$runtime_token"* ]] || fail 'runtime token appeared in process arguments'
[[ "$argument_probe" != *"$gateway_token"* ]] || fail 'gateway token appeared in process arguments'
wait "$argument_probe_pid" || fail 'argument probe process failed'

STS2_PROBE_STATUS_LINE='HTTP/1.1 401 Unauthorized' \
    authorization_status_matches 401 || fail 'missing or wrong auth rejection'
STS2_PROBE_STATUS_LINE='HTTP/1.1 200 OK' \
    authorization_status_matches 200 || fail 'correct auth acceptance'
STS2_PROBE_STATUS_LINE='HTTP/1.1 200 OK' \
    authorization_status_matches 401 && fail 'wrong auth accepted'
export STS2_EXPECTED_AUTHORIZATION="$runtime_token"
unset STS2_PROBE_AUTHORIZATION
authorization_header_matches && fail 'missing auth accepted'
STS2_PROBE_AUTHORIZATION='Bearer wrong'
authorization_header_matches && fail 'wrong auth accepted'
STS2_PROBE_AUTHORIZATION="Bearer $runtime_token"
authorization_header_matches || fail 'correct auth rejected'
unset STS2_EXPECTED_AUTHORIZATION STS2_PROBE_AUTHORIZATION

tasklist_cmd="$fixture"
export STS2_SESSION_TEST_GAME_RUNNING=0
game_is_running && fail 'reported a stopped game as running'
export STS2_SESSION_TEST_GAME_RUNNING=1
game_is_running || fail 'did not detect an already-running game'
if running_error=$(refuse_if_game_running 2>&1); then
    fail 'already-running game was not refused'
fi
[[ "$running_error" == *'restart required'* ]] || fail 'restart-required message was missing'
unset STS2_SESSION_TEST_GAME_RUNNING

setsid bash -c 'trap "exit 0" TERM INT; while :; do sleep 1; done' \
    </dev/null >/dev/null 2>&1 &
fixture_pid=$!
fixture_identity=$(record_process_identity "$fixture_pid") || fail 'process identity'
fixture_group=$(printf '%s\n' "$fixture_identity" | sed -n '1p')
fixture_session=$(printf '%s\n' "$fixture_identity" | sed -n '2p')
group_has_process "$fixture_group" "$fixture_session" || fail 'process ownership'
stop_posix_group "$fixture_group" "$fixture_session" "$fixture_pid"
group_has_process "$fixture_group" "$fixture_session" && fail 'owned process cleanup'

startup_timeout_seconds=1
if (wait_for_probe 'synthetic timeout' 127.0.0.1 1 /health/ready 200 '' posix "$BASHPID") \
    >/dev/null 2>&1; then
    fail 'startup timeout was not bounded'
fi

bridge_source=$(<"$script_dir/session-launcher/windows-bridge/Program.cs")
[[ "$bridge_source" == *'Console.ReadLine()'* ]] || fail 'bridge does not read the token from stdin'
[[ "$bridge_source" == *'STS2_RUNTIME_TOKEN'* ]] || fail 'bridge does not set the runtime token environment'
[[ "$bridge_source" == *'FileName = options.GameExecutable'* ]] || fail 'bridge does not launch the requested game directly'
[[ "$bridge_source" == *'WorkingDirectory = options.WorkingDirectory'* ]] || fail 'bridge does not preserve the game working directory'
[[ "$bridge_source" == *'UseShellExecute = false'* ]] || fail 'bridge shell boundary is not explicit'
[[ "$bridge_source" == *'ArgumentList.Add("--headless")'* ]] || fail 'bridge does not force headless launch'
[[ "$bridge_source" == *'ArgumentList.Add("--audio-driver")'* ]] || fail 'bridge does not select dummy audio'
[[ "$bridge_source" == *'ArgumentList.Add("Dummy")'* ]] || fail 'bridge does not select the dummy audio driver'
[[ "$bridge_source" != *'Start-Process'* ]] || fail 'bridge delegates game launch to PowerShell'
[[ "$bridge_source" != *'ArgumentList.Add(credential)'* ]] || fail 'bridge passes a token as an argument'
[[ "$bridge_source" != *'QuotePowerShell(credential)'* ]] || fail 'bridge interpolates a token into PowerShell'
input_automation_is_disabled || fail 'launcher or bridge contains a system-input control seam'
dev_cycle_source=$(<"$script_dir/dev-cycle.sh")
[[ "$dev_cycle_source" == *'UseShellExecute'* ]] || fail 'development cycle does not use a non-shell launch boundary'
[[ "$dev_cycle_source" == *'--headless --audio-driver Dummy'* ]] || fail 'development cycle does not force headless launch'
[[ "$dev_cycle_source" != *'Start-Process'* ]] || fail 'development cycle delegates game launch to PowerShell'

printf '%s\n' \
    'CSPRNG/encoding/difference=TRUE' \
    'Role separation=TRUE' \
    'Argument leakage=FALSE' \
    'Auth missing/wrong/correct=TRUE' \
    'Already-running refusal predicate=TRUE' \
    'Startup timeout=TRUE' \
    'Owned cleanup=TRUE' \
    'WSL-to-Windows stdin boundary=TRUE' \
    'System input automation=FALSE'
