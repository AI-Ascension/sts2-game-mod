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
        if [[ $mode == AssertStopped && ${STS2_DEV_CYCLE_TEST_RUNNING:-no} == yes ]]; then
            exit 1
        fi
        ;;
    *) exit 2 ;;
esac
