#!/usr/bin/env bash
set -euo pipefail
case ${STS2_BRIDGE_FIXTURE_MODE:-valid} in
    stall) sleep 30 ;;
    oversized) printf '%0256d\n' 0; sleep 30 ;;
    partial) printf 'STARTED=TRUE\n'; sleep 30 ;;
    valid)
        IFS= read -r credential
        [[ ${#credential} == 96 ]] || exit 2
        printf 'STARTED=TRUE\nPID=123\nSTART_TICKS=456\n'
        # Closing the owned channel must be enough to cancel the guardian.
        IFS= read -r cancellation || :
        ;;
esac
