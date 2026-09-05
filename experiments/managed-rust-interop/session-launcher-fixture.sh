#!/usr/bin/env bash

set -euo pipefail

if [[ ${STS2_SESSION_TEST_INSPECTION_HANG:-0} == 1 ]]; then
    sleep 3
fi

if [[ "${STS2_SESSION_TEST_GAME_RUNNING:-0}" == 1 ]]; then
    printf '%s\n' 'SlayTheSpire2.exe  4242 Console 1 12,345 K'
fi
