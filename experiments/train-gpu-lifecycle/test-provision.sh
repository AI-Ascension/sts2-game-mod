#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Failure boundaries only; regular files do not model xe allocation semantics.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/provision.sh"
test_root=$(mktemp -d)
trap 'rm -rf -- "$test_root"' EXIT

fixture() {
    pci=$test_root/$1/$pci_id; debug=$test_root/$1/debug; record=$test_root/$1/before.tsv
    mkdir -p "$pci" "$test_root/$1/xe" "$test_root/$1/$vf_id"
    ln -s "$test_root/$1/xe" "$pci/driver"
    ln -s "$test_root/$1/$vf_id" "$pci/virtfn0"
    printf '0x8086\n' > "$pci/vendor"; printf '0xe211\n' > "$pci/device"
    printf '7\n' > "$pci/sriov_totalvfs"; printf '0\n' > "$pci/sriov_numvfs"
    profile
    local path
    for path in "${paths[@]}"; do mkdir -p "$(dirname "$path")"; printf '0\n' > "$path"; done
}
virsh() { printf 'shut off\n'; }
podman() { printf 'false\n'; }
rejected() { if "$@" >/dev/null 2>&1; then fail 'expected refusal'; exit 1; fi; }

(
    fixture active
    virsh() { printf 'running\n'; }
    rejected apply
    [[ ! -e $record && $(number "$pci/sriov_numvfs") == 0 ]]
    echo 'PASS active guest refuses before snapshot'
)
(
    fixture workload
    podman() { printf 'true\n'; }
    rejected apply
    [[ ! -e $record ]]
    echo 'PASS conflicting workload refuses before snapshot'
)
(
    fixture missing
    rm -- "${paths[0]}"
    rejected apply
    [[ ! -e $record ]]
    echo 'PASS missing field refuses before snapshot'
)
(
    fixture repeat
    apply
    check
    before=$(sha256sum "$record")
    idle() { fail 'matching profile must not query guest state'; }
    apply
    [[ $(sha256sum "$record") == "$before" ]]
    echo 'PASS matching profile is read-only'
)
(
    fixture partial
    write_checked() { fail 'injected kernel failure'; }
    rejected apply
    [[ -s $record && $(number "$pci/sriov_numvfs") == 0 ]]
    before=$(sha256sum "$record")
    rejected apply
    [[ $(sha256sum "$record") == "$before" ]]
    echo 'PASS partial failure retains rollback and refuses blind retry'
)
(
    fixture rollback
    apply
    restore
    [[ $(number "$pci/sriov_numvfs") == 0 ]]
    for path in "${paths[@]}"; do [[ $(number "$path") == 0 ]]; done
    echo 'PASS restore returns all original values'
)
(
    fixture foreign
    mkdir -p "$test_root/foreign/vfio-pci"
    ln -s "$test_root/foreign/vfio-pci" "$pci/virtfn0/driver"
    rejected apply
    [[ ! -e $record ]]
    echo 'PASS VFIO-bound VF refuses resource changes'
)
(
    fixture xe_foreign
    mkdir -p "$test_root/xe_foreign/xe-vfio-pci"
    ln -s "$test_root/xe_foreign/xe-vfio-pci" "$pci/virtfn0/driver"
    rejected apply
    [[ ! -e $record ]]
    echo 'PASS xe-vfio-pci-bound VF refuses resource changes'
)
(
    fixture ordering
    write_checked() {
        if [[ $1 == "$pci/sriov_numvfs" && $2 == 1 ]]; then
            local i
            for i in "${!paths[@]}"; do [[ $(number "${paths[i]}") == "${expected[i]}" ]] || return 1; done
        fi
        printf '%s\n' "$2" > "$1"
    }
    apply
    echo 'PASS quotas are complete before VF enable'
)
(
    fixture corrupt
    apply
    printf 'unexpected\t0\n' > "$record"
    rejected restore
    check
    echo 'PASS corrupt rollback record causes no resource writes'
)
(
    fixture check_workload
    apply
    podman() { printf 'true\n'; }
    rejected check
    rejected apply
    echo 'PASS read-only checks and matching apply reject a conflicting workload'
)
