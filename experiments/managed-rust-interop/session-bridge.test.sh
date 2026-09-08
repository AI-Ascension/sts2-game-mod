#!/usr/bin/env bash
set -Eeuo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
source "$script_dir/session-launcher.sh"
fail() { printf 'bridge handoff test failed: %s\n' "$1" >&2; exit 1; }
for scenario in valid valid-forward valid-backward stall partial oversized; do
    scenario_start=$(launcher_elapsed_seconds) || fail 'elapsed-time clock unavailable'
    (
        runtime_token=$(new_credential)
        live_authorization_deadline=$((EPOCHSECONDS + 30))
        startup_timeout_seconds=1
        [[ "$scenario" != valid* ]] || startup_timeout_seconds=3
        export STS2_BRIDGE_FIXTURE_MODE=$scenario
        if [[ "$scenario" == valid* ]]; then STS2_BRIDGE_FIXTURE_MODE=valid; fi
        case "$scenario" in
            valid-forward) read() { SECONDS=1000000; builtin read "$@"; } ;;
            valid-backward) read() { SECONDS=-1000000; builtin read "$@"; } ;;
        esac
        start=$(launcher_elapsed_seconds) || fail 'elapsed-time clock unavailable'
        trap 'stop_bridge_guardian' EXIT
        launch_owned_bridge bash "$script_dir/session-bridge-fixture.sh"
        [[ "$scenario" == valid* && "$game_pid" == 123 && "$game_start_ticks" == 456 ]] \
            || fail 'invalid guardian admitted'
        elapsed=$(launcher_elapsed_seconds) || fail 'elapsed-time clock unavailable'
        (( elapsed - start < 3 )) || fail 'handoff not bounded'
    ) >/dev/null 2>&1 && status=0 || status=$?
    elapsed=$(launcher_elapsed_seconds) || fail 'elapsed-time clock unavailable'
    (( elapsed - scenario_start < 10 )) || fail 'handoff plus owned cleanup exceeded its bound'
    if [[ "$scenario" == valid* ]]; then
        [[ $status == 0 ]] || fail 'valid guardian rejected'
    else
        [[ $status != 0 ]] || fail 'stalled or malformed guardian accepted'
    fi
done
clock_fixture=$(mktemp -d)
trap 'rm -rf -- "$clock_fixture"' EXIT
if (
    live_authorization_deadline=$((EPOCHSECONDS + 30))
    launcher_elapsed_seconds() { return 1; }
    trap 'stop_bridge_guardian' EXIT
    launch_owned_bridge bash -c 'printf started >"$1"' -- "$clock_fixture/started"
) >/dev/null 2>&1; then
    fail 'unavailable elapsed-time clock admitted a guardian'
fi
[[ ! -e "$clock_fixture/started" ]] || fail 'clock failure started a subprocess'
printf 'PASS: bounded guardian receipt, clock-step tolerance, clock failure, partial/oversized rejection, owned cancellation\n'
