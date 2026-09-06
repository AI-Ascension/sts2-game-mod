#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

config=/etc/sts2-gpu-lifecycle/debug-directory
helper=/usr/local/libexec/sts2-gpu-lifecycle/provision.sh

gate_main() {
    [[ $# == 4 ]] || return 1
    [[ $1 == sts.home.complete.tech-windows ]] || return 0
    [[ $3 == begin ]] || return 0
    case $2 in
        prepare|start|restore|migrate|attach) ;;
        *) return 0 ;;
    esac
    local debug
    debug=$(cat -- "$config") || return
    [[ $debug == /* && $debug != *[$'\t\n ']* ]] || return 1
    # No libvirt calls, resource writes, or XML output in this hook. A failed read-only
    # check blocks startup. Reconnect is deliberately excluded: it concerns a running VM.
    "$helper" check "$debug" /dev/null >&2
}

if [[ ${BASH_SOURCE[0]} == "$0" ]]; then gate_main "$@"; fi
