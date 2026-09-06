#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

config=/etc/sts2-gpu-lifecycle/debug-directory
helper=/usr/local/libexec/sts2-gpu-lifecycle/provision.sh
boot_id_file=/proc/sys/kernel/random/boot_id
state_directory=/var/lib/sts2-gpu-lifecycle

boot_prepare() {
    local debug boot_id
    debug=$(cat -- "$config") || return
    [[ $debug == /* && $debug != *[$'\t\n ']* ]] || return 1
    boot_id=$(cat -- "$boot_id_file") || return
    [[ $boot_id =~ ^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$ ]] || return 1
    # Reuse the boot's exclusive record: a partial failure must refuse blind retry.
    "$helper" apply "$debug" "$state_directory/$boot_id.tsv" || return
    "$helper" check "$debug" /dev/null
}

if [[ ${BASH_SOURCE[0]} == "$0" ]]; then
    [[ $# == 0 && $EUID == 0 ]] || exit 1
    boot_prepare
fi
