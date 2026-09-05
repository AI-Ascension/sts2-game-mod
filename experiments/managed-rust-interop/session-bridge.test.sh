#!/usr/bin/env bash
set -Eeuo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
source "$script_dir/session-launcher.sh"
fail() { printf 'bridge handoff test failed: %s\n' "$1" >&2; exit 1; }
for scenario in valid stall partial oversized; do
    scenario_start=$SECONDS
    (
        runtime_token=$(new_credential)
        live_authorization_deadline=$((EPOCHSECONDS + 30))
        startup_timeout_seconds=1
        [[ "$scenario" != valid ]] || startup_timeout_seconds=3
        export STS2_BRIDGE_FIXTURE_MODE=$scenario
        start=$SECONDS
        trap 'stop_bridge_guardian' EXIT
        launch_owned_bridge bash "$script_dir/session-bridge-fixture.sh"
        [[ "$scenario" == valid && "$game_pid" == 123 && "$game_start_ticks" == 456 ]] \
            || fail 'invalid guardian admitted'
        (( SECONDS - start < 3 )) || fail 'handoff not bounded'
    ) >/dev/null 2>&1 && status=0 || status=$?
    (( SECONDS - scenario_start < 10 )) || fail 'handoff plus owned cleanup exceeded its bound'
    if [[ "$scenario" == valid ]]; then
        [[ $status == 0 ]] || fail 'valid guardian rejected'
    else
        [[ $status != 0 ]] || fail 'stalled or malformed guardian accepted'
    fi
done
printf 'PASS: bounded guardian receipt, partial/oversized rejection, owned cancellation\n'
