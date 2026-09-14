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
        selected_stopped_file="$STS2_DEV_CYCLE_TEST_LOG.selected-stopped"
        # Model the selected installation and other-path instances separately so
        # AssertStopped stays selected-only, matching the real inspector. Markers
        # persist across fixture invocations because each mode is a new process.
        selected_running=no
        other_running=no
        [[ ${STS2_DEV_CYCLE_TEST_RUNNING:-no} == yes && ! -e $stopped_file && ! -e $selected_stopped_file ]] \
            && selected_running=yes
        [[ ${STS2_DEV_CYCLE_TEST_OTHER_RUNNING:-no} == yes && ! -e $stopped_file ]] && other_running=yes
        if [[ $mode == Stop ]]; then
            # Stop terminates only the selected installation.
            : > "$selected_stopped_file"
            selected_running=no
        fi
        if [[ $mode == StopAll ]]; then
            : > "$stopped_file"
            selected_running=no
            other_running=no
        fi
        if [[ $mode == AssertStopped && $selected_running == yes ]]; then
            exit 1
        fi
        if [[ $mode == AssertNoGame && ($selected_running == yes || $other_running == yes) ]]; then
            exit 1
        fi
        ;;
    *) exit 2 ;;
esac
