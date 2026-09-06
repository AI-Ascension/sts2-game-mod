#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/boot.sh"
source "$(dirname "${BASH_SOURCE[0]}")/qemu-start-gate.sh"
test_root=$(mktemp -d)
trap 'rm -rf -- "$test_root"' EXIT
config=$test_root/debug-directory
boot_id_file=$test_root/boot-id
state_directory=$test_root/records
helper=fixture_helper
mkdir "$state_directory"
printf '/fixture/debug\n' > "$config"
printf '11111111-2222-3333-4444-555555555555\n' > "$boot_id_file"
fixture_helper() { printf '%s\t%s\t%s\n' "$@" >> "$test_root/calls"; }
rejected() { if "$@" >/dev/null 2>&1; then echo 'Expected refusal' >&2; exit 1; fi; }

boot_prepare
[[ $(wc -l < "$test_root/calls") == 2 ]]
[[ $(head -n 1 "$test_root/calls") == $'apply\t/fixture/debug\t'"$state_directory/11111111-2222-3333-4444-555555555555.tsv" ]]
[[ $(tail -n 1 "$test_root/calls") == $'check\t/fixture/debug\t/dev/null' ]]
echo 'PASS boot uses a stable per-boot rollback identity and verifies apply'

(
    fixture_helper() { printf '%s\n' "$1" >> "$test_root/failure-calls"; return 1; }
    rejected boot_prepare
    [[ $(cat "$test_root/failure-calls") == apply ]]
    echo 'PASS failed apply does not proceed to check'
)
(
    printf 'invalid\n' > "$boot_id_file"
    rejected boot_prepare
    [[ $(wc -l < "$test_root/calls") == 2 ]]
    echo 'PASS malformed boot identity prevents helper invocation'
)

for operation in prepare start restore migrate attach; do
    gate_main sts.home.complete.tech-windows "$operation" begin - > "$test_root/stdout"
    [[ ! -s $test_root/stdout ]]
done
[[ $(wc -l < "$test_root/calls") == 7 ]]
[[ $(tail -n 1 "$test_root/calls") == $'check\t/fixture/debug\t/dev/null' ]]
echo 'PASS admission hooks check without altering XML'
gate_main other-domain prepare begin -
gate_main sts.home.complete.tech-windows reconnect begin -
gate_main sts.home.complete.tech-windows release end -
[[ $(wc -l < "$test_root/calls") == 7 ]]
echo 'PASS unrelated domains and running-guest reconnects do not invoke the guard'
(
    fixture_helper() { return 1; }
    rejected gate_main sts.home.complete.tech-windows prepare begin -
    echo 'PASS failed check blocks VM admission'
)
printf 'relative/path\n' > "$config"
rejected gate_main sts.home.complete.tech-windows prepare begin -
[[ $(wc -l < "$test_root/calls") == 7 ]]
echo 'PASS invalid configuration prevents helper invocation'
