#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Operator fixture candidate. Not installed, not host-verified.
set -euo pipefail

pci_id=0000:07:00.0
vf_id=0000:07:00.1
pci=/sys/bus/pci/devices/$pci_id
domain=sts.home.complete.tech-windows
llama=cafc3e3f45acf6f329c50c56039f50da9b7c8d4c479c22169e47ebbaf36d2b8f
declare -a paths=() expected=() original=()

fail() { echo "GPU lifecycle stopped: $*" >&2; return 1; }
number() {
    local value
    value=$(cat -- "$1") || return
    [[ $value =~ ^(0x[0-9a-fA-F]+|[0-9]+)$ ]] || { fail "invalid numeric field $1"; return 1; }
    if [[ $value == 0x* ]]; then printf '%d\n' "$((value))";
    else printf '%d\n' "$((10#$value))"; fi
}
write_checked() {
    printf '%s\n' "$2" > "$1" || return
    [[ $(number "$1") == "$2" ]] || fail "kernel readback mismatch: $1"
}
identity() {
    [[ $(number "$pci/vendor") == 32902 && $(number "$pci/device") == 57873 ]] || {
        fail 'PF does not match the verified Intel device'; return 1;
    }
    [[ $(basename "$(readlink -f "$pci/driver")") == xe ]] || {
        fail 'PF must remain bound to xe'; return 1;
    }
    [[ $(number "$pci/sriov_totalvfs") == 7 ]] || { fail 'unexpected VF capacity'; return 1; }
    count=$(number "$pci/sriov_numvfs") || return
    [[ $count == 0 || $count == 1 ]] || { fail 'unrelated VF configuration'; return 1; }
    if [[ $count == 1 ]]; then
        [[ $(basename "$(readlink -f "$pci/virtfn0")") == "$vf_id" ]] || {
            fail 'unexpected VF identity'; return 1;
        }
    fi
}
add() { paths+=("$debug/$1"); expected+=("$2"); }
profile() {
    local gt role
    paths=(); expected=()
    # PF resources first, then VF quotas, enable last.
    add gt0/pf/lmem_spare 4294967296
    for gt in gt0 gt1; do
        add "$gt/pf/exec_quantum_ms" 25
        add "$gt/pf/preempt_timeout_us" 500000
    done
    add gt0/vf1/lmem_quota 6442450944
    for gt in gt0 gt1; do
        add "$gt/vf1/ggtt_quota" 671088640
        add "$gt/vf1/contexts_quota" 8192
        add "$gt/vf1/doorbells_quota" 60
        add "$gt/vf1/exec_quantum_ms" 25
        add "$gt/vf1/preempt_timeout_us" 500000
    done
}
idle() {
    local state driver
    state=$(LC_ALL=C virsh -c qemu:///system domstate "$domain") || return
    [[ $state == 'shut off' ]] || { fail 'Windows fixture must be shut off'; return 1; }
    if [[ -e $pci/virtfn0/driver ]]; then
        driver=$(basename "$(readlink -f "$pci/virtfn0/driver")")
        [[ $driver != vfio-pci ]] || { fail 'VF still bound to VFIO; reconcile ownership'; return 1; }
    fi
    state=$(podman inspect --format '{{.State.Running}}' "$llama") || return
    [[ $state == false ]] || { fail 'conflicting GPU workload is running or unknown'; return 1; }
}
check() {
    identity || return
    [[ $count == 1 ]] || { fail 'VF is not enabled'; return 1; }
    local i value
    for i in "${!paths[@]}"; do
        value=$(number "${paths[i]}") || return
        [[ $value == "${expected[i]}" ]] || { fail "profile mismatch: ${paths[i]}"; return 1; }
    done
}
snapshot() {
    # Noclobber preserves the first record, including after partial failure.
    (set -o noclobber; umask 077
        { printf '%s\t%s\n' "$pci_id" "$count"
          local i
          for i in "${!paths[@]}"; do printf '%s\t%s\n' "${paths[i]}" "${original[i]}"; done
        } > "$record"
    ) || return
    sync -f "$record"
}
apply() {
    identity || return
    local i same=1
    original=()
    for i in "${!paths[@]}"; do
        original[i]=$(number "${paths[i]}") || return
        [[ ${original[i]} == "${expected[i]}" ]] || same=0
    done
    if [[ $count == 1 && $same == 1 ]]; then return 0; fi
    idle || return
    snapshot || return
    for i in "${!paths[@]}"; do
        if [[ ${original[i]} != "${expected[i]}" ]]; then
            write_checked "${paths[i]}" "${expected[i]}" || return
        fi
    done
    if [[ $count == 0 ]]; then write_checked "$pci/sriov_numvfs" 1 || return; fi
    check
}
restore() {
    identity || return
    local header prior line path value i=0
    original=()
    {
        IFS=$'\t' read -r header prior || return
        [[ $header == "$pci_id" && ( $prior == 0 || $prior == 1 ) ]] || {
            fail 'invalid rollback identity'; return 1;
        }
        while IFS=$'\t' read -r path value; do
            [[ $i -lt ${#paths[@]} && $path == "${paths[i]}" && $value =~ ^[0-9]+$ ]] || {
                fail 'rollback paths or values do not match'; return 1;
            }
            original+=("$value"); i=$((i + 1))
        done
    } < "$record"
    [[ $i == "${#paths[@]}" ]] || { fail 'incomplete rollback record'; return 1; }
    idle || return
    if [[ $count != 0 ]]; then write_checked "$pci/sriov_numvfs" 0 || return; fi
    # Reverse apply order: release VF resources before reducing PF spare.
    for ((i=${#paths[@]}-1; i>=0; i--)); do
        write_checked "${paths[i]}" "${original[i]}" || return
    done
    if [[ $prior == 1 ]]; then write_checked "$pci/sriov_numvfs" 1 || return; fi
}
main() {
    [[ $# == 3 ]] || { fail 'usage: provision.sh check|apply|restore DEBUG_DIRECTORY RECORD'; return 1; }
    [[ $EUID == 0 ]] || { fail 'root access required for xe debugfs'; return 1; }
    local operation=$1
    debug=$(realpath -e -- "$2"); record=$3
    [[ $debug != *[$'\t\n ']* ]] || { fail 'invalid debugfs directory'; return 1; }
    grep -qw -- "$pci_id" "$debug/name" || { fail 'debugfs PF identity mismatch'; return 1; }
    profile
    exec 9>/run/lock/sts2-gpu-lifecycle.lock
    flock -n 9 || { fail 'another lifecycle operation is active'; return 1; }
    case $operation in
        check) check ;;
        apply) apply ;;
        restore) restore ;;
        *) fail 'unknown operation'; return 1 ;;
    esac
    echo "GPU profile $operation verified"
}
if [[ ${BASH_SOURCE[0]} == "$0" ]]; then main "$@"; fi
