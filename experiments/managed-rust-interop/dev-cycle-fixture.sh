#!/usr/bin/env bash
set -euo pipefail
# Invoked only by dev-cycle.test.sh under copied tool names.
case ${0##*/} in
    wslpath) printf '%s\n' "${@: -1}" ;;
    date) printf '%s\n' '20260904T000000Z' ;;
    powershell.exe)
        mode=''
        while (( $# )); do
            if [[ $1 == -Mode ]]; then mode=$2; break; fi
            shift
        done
        printf '%s\n' "$mode" >> "$STS2_DEV_CYCLE_TEST_LOG"
        [[ ${STS2_DEV_CYCLE_TEST_INSPECTION_FAIL:-no} != yes ]] || exit 1
        stopped_file="$STS2_DEV_CYCLE_TEST_LOG.stopped"
        running=no
        if [[ ${STS2_DEV_CYCLE_TEST_RUNNING:-no} == yes || ${STS2_DEV_CYCLE_TEST_OTHER_RUNNING:-no} == yes ]]; then
            [[ -e $stopped_file ]] || running=yes
        fi
        if [[ $mode == StopAll ]]; then
            : > "$stopped_file"
            running=no
        fi
        if [[ $mode == AssertStopped && $running == yes ]]; then
            exit 1
        fi
        if [[ $mode == AssertNoGame && $running == yes ]]; then
            exit 1
        fi
        ;;
    *) exit 2 ;;
esac
