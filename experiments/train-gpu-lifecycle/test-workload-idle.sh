#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

script_dir=$(dirname "${BASH_SOURCE[0]}")
provision=$script_dir/provision.sh

fail() {
    echo "FAIL $*" >&2
    exit 1
}

run_case() {
    local name=$1 exists=$2 state=$3 inspect=$4 expected=$5 output rc
    set +e
    output=$(
        EXISTS=$exists STATE=$state INSPECT=$inspect \
        bash -c '
            set -euo pipefail
            source "$1"
            podman() {
                if [[ $1 == container && $2 == exists ]]; then
                    return "$EXISTS"
                fi
                if [[ $1 == inspect ]]; then
                    printf "%s\n" "$STATE"
                    return "$INSPECT"
                fi
                return 99
            }
            workload_idle
        ' test "$provision" 2>&1
    )
    rc=$?
    set -e
    if [[ $expected == 0 && $rc == 0 || $expected == nonzero && $rc != 0 ]]; then
        printf 'PASS %s\n' "$name"
    else
        printf '%s\n' "$output" >&2
        fail "$name (expected exit $expected, got $rc)"
    fi
}

run_case 'retired exact workload absent' 1 false 0 0
run_case 'present stopped workload' 0 false 0 0
run_case 'present running workload' 0 true 0 nonzero
run_case 'unknown lookup fails closed' 125 false 0 nonzero
run_case 'inspection error fails closed' 0 false 125 nonzero
run_case 'invalid state fails closed' 0 unknown 0 nonzero
