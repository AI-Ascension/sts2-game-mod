#!/usr/bin/env bash

set -Eeuo pipefail

session_launcher_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
session_launcher_repo_root=$(cd -- "$session_launcher_dir/../.." && pwd -P)
session_launcher_package_script="$session_launcher_dir/package-runtime-addon.sh"
session_launcher_bridge_project="$session_launcher_dir/session-launcher/windows-bridge/SessionWindowsBridge.csproj"
session_launcher_bridge_dll="$session_launcher_dir/session-launcher/windows-bridge/bin/Release/net8.0/SessionWindowsBridge.dll"

probe_timeout_seconds=2
startup_timeout_seconds=30
harness_timeout_seconds=30
cleanup_done=0
cleanup_failed=0
runtime_token=''
gateway_token=''
gateway_pid=''
gateway_group=''
gateway_session=''
harness_pid=''
harness_group=''
harness_session=''
game_pid=''
network_port=''
gateway_host=''
gateway_port=''
game_probe_host=''
tasklist_cmd=''
taskkill_cmd=''
session_launcher_caller_group=$(ps -o pgid= -p $$ 2>/dev/null | tr -d ' ')
game_started=0
gateway_started=0
harness_started=0

die() {
    printf 'error: %s\n' "$1" >&2
    exit 1
}

take_value() {
    local option=$1
    if [[ $# -lt 2 || -z ${2:-} ]]; then
        die "$option requires a value"
    fi
    printf '%s' "$2"
}

credential_is_safe() {
    local value=${1:-}
    [[ ${#value} -ge 43 && ${#value} -le 256 ]] \
        && [[ "$value" =~ ^[A-Za-z0-9_-]+$ ]]
}

new_credential() {
    local value
    value=$(openssl rand -hex 48)
    [[ "$value" =~ ^[0-9a-f]{96}$ ]] || return 1
    printf '%s' "$value"
}

authorization_status_matches() {
    local expected=$1
    local status_line=${STS2_PROBE_STATUS_LINE:-}
    [[ "$status_line" == "HTTP/1.1 $expected "* \
        || "$status_line" == "HTTP/1.0 $expected "* ]]
}

authorization_header_matches() {
    local expected=${STS2_EXPECTED_AUTHORIZATION:-}
    local actual=${STS2_PROBE_AUTHORIZATION:-}
    [[ "$actual" == "Bearer $expected" && -n "$expected" ]]
}

parse_endpoint() {
    local endpoint=$1
    [[ "$endpoint" =~ ^([A-Za-z0-9.-]+):([0-9]{1,5})$ ]] || return 1
    local port=${BASH_REMATCH[2]}
    (( port >= 1 && port <= 65535 )) || return 1
    printf '%s\n%s' "${BASH_REMATCH[1]}" "$port"
}

find_windows_tool() {
    local tool=$1
    local candidate

    if command -v "$tool" >/dev/null 2>&1; then
        command -v "$tool"
        return 0
    fi

    case "$tool" in
        tasklist.exe|taskkill.exe)
            for candidate in \
                "/mnt/c/Windows/System32/$tool" \
                "/mnt/c/Windows/Sysnative/$tool"; do
                if [[ -x "$candidate" ]]; then
                    printf '%s' "$candidate"
                    return 0
                fi
            done
            ;;
    esac

    return 1
}

to_wsl_path() {
    local input=$1
    if [[ "$input" == /* ]]; then
        printf '%s' "$input"
        return 0
    fi
    command -v wslpath >/dev/null 2>&1 \
        || die "wslpath is required to convert Windows path '$input'"
    wslpath -u -- "$input"
}

to_windows_path() {
    command -v wslpath >/dev/null 2>&1 \
        || die 'wslpath is required to cross the WSL-to-Windows boundary'
    wslpath -w -- "$1"
}

resolve_executable() {
    local input=$1
    if [[ "$input" == /* ]]; then
        printf '%s' "$input"
    else
        command -v "$input" || return 1
    fi
}

posix_process_is_alive() {
    local pid=$1
    [[ "$pid" =~ ^[0-9]+$ ]] && kill -0 "$pid" 2>/dev/null
}

group_has_process() {
    local group=$1
    local session=$2
    [[ "$group" =~ ^[0-9]+$ && "$session" =~ ^[0-9]+$ ]] || return 1
    ps -eo pgid=,sid= 2>/dev/null \
        | awk -v expected_group="$group" -v expected_session="$session" \
            '$1 == expected_group && $2 == expected_session { found = 1 } END { exit(found ? 0 : 1) }'
}

record_process_identity() {
    local pid=$1
    local identity
    local process_group
    local process_session
    local identity_attempt

    # A freshly detached process may not be visible to ps on the first read.
    # Keep the retry bounded while preserving the caller-group safety check.
    for ((identity_attempt = 0; identity_attempt < 50; identity_attempt++)); do
        identity=$(ps -o pgid=,sid= -p "$pid" 2>/dev/null | awk 'NF == 2 { print; exit }')
        if [[ "$identity" =~ ^[[:space:]]*([0-9]+)[[:space:]]+([0-9]+)[[:space:]]*$ ]]; then
            process_group=${BASH_REMATCH[1]}
            process_session=${BASH_REMATCH[2]}
            if [[ -n "$session_launcher_caller_group" \
                && "$process_group" != "$session_launcher_caller_group" ]]; then
                printf '%s\n%s' "$process_group" "$process_session"
                return 0
            fi
        fi
        sleep 0.05
    done
    return 1
}

stop_posix_group() {
    local group=$1
    local session=$2
    local pid=$3
    local count

    if [[ -z "$group" || -z "$session" ]]; then
        return 0
    fi
    if [[ -n "$session_launcher_caller_group" && "$group" == "$session_launcher_caller_group" ]]; then
        cleanup_failed=1
        return 1
    fi
    if ! group_has_process "$group" "$session"; then
        return 0
    fi
    if ! kill -TERM -- "-$group" 2>/dev/null; then
        cleanup_failed=1
    fi
    for ((count = 0; count < 50; count++)); do
        group_has_process "$group" "$session" || break
        sleep 0.1
    done
    if group_has_process "$group" "$session"; then
        if ! kill -KILL -- "-$group" 2>/dev/null; then
            cleanup_failed=1
        fi
    fi
    if [[ "$pid" =~ ^[0-9]+$ ]]; then
        wait "$pid" 2>/dev/null || :
    fi
}

probe_status() {
    local host=$1
    local port=$2
    local path=$3
    local expected=$4
    local token=${5:-}

    STS2_PROBE_TOKEN="$token" timeout --foreground "$probe_timeout_seconds" bash -c '
        set -u
        host=$1
        port=$2
        path=$3
        expected=$4
        if ! exec {socket_fd}<>"/dev/tcp/$host/$port"; then
            exit 1
        fi
        printf "GET %s HTTP/1.1\r\n" "$path" >&"$socket_fd"
        printf "Host: %s\r\n" "$host" >&"$socket_fd"
        if [[ -n ${STS2_PROBE_TOKEN:-} ]]; then
            printf "Authorization: Bearer %s\r\n" "$STS2_PROBE_TOKEN" >&"$socket_fd"
        fi
        printf "Connection: close\r\n\r\n" >&"$socket_fd"
        if ! IFS= read -r -t 1 status_line <&"$socket_fd"; then
            exec {socket_fd}>&-
            exit 1
        fi
        exec {socket_fd}>&-
        if [[ "$expected" == ANY ]]; then
            [[ "$status_line" == HTTP/1.1\ * || "$status_line" == HTTP/1.0\ * ]]
            exit
        fi
        [[ "$status_line" == "HTTP/1.1 $expected "* || "$status_line" == "HTTP/1.0 $expected "* ]]
    ' session-http-probe "$host" "$port" "$path" "$expected" 2>/dev/null
}

run_gateway_with_credentials() {
    STS2_GATEWAY_ADDR="$gateway_addr" \
        STS2_MOD_ADDR="$mod_addr" \
        STS2_GATEWAY_TOKEN="$gateway_token" \
        STS2_MOD_TOKEN="$runtime_token" \
        STS2_INSTANCE_ID=instance-1 \
        STS2_CALLER_ID=harness \
        STS2_SESSION_ID=session-1 \
        STS2_LEASE_ID=lease-1 \
        STS2_LEASE_EPOCH=1 \
        "$@"
}

run_harness_with_credentials() {
    STS2_GATEWAY_ADDR="$gateway_addr" \
        STS2_GATEWAY_TOKEN="$gateway_token" \
        STS2_MCP_BINARY="$mcp_binary" \
        STS2_INSTANCE_ID=instance-1 \
        STS2_CALLER_ID=harness \
        STS2_SESSION_ID=session-1 \
        STS2_LEASE_ID=lease-1 \
        STS2_LEASE_EPOCH=1 \
        STS2_MCP_SESSION_ID=mcp-session-1 \
        "$@"
}

game_is_running() {
    "$tasklist_cmd" /FI 'IMAGENAME eq SlayTheSpire2.exe' /NH 2>/dev/null \
        | tr -d '\r' \
        | awk '$1 == "SlayTheSpire2.exe" { found = 1 } END { exit(found ? 0 : 1) }'
}

game_pid_is_running() {
    [[ "$game_pid" =~ ^[0-9]+$ ]] || return 1
    "$tasklist_cmd" /FI "PID eq $game_pid" /NH 2>/dev/null \
        | tr -d '\r' \
        | awk -v expected_pid="$game_pid" \
            '$1 == "SlayTheSpire2.exe" && $2 == expected_pid { found = 1 } END { exit(found ? 0 : 1) }'
}

refuse_if_game_running() {
    if game_is_running; then
        die 'SlayTheSpire2.exe is already running; restart required before an ephemeral session can start'
    fi
}

wait_for_probe() {
    local label=$1
    local host=$2
    local port=$3
    local path=$4
    local expected=$5
    local token=$6
    local process_kind=$7
    local process_id=$8
    local attempts=$((startup_timeout_seconds * 5))
    local attempt

    for ((attempt = 0; attempt < attempts; attempt++)); do
        if [[ "$process_kind" == posix ]] && ! posix_process_is_alive "$process_id"; then
            die "$label exited before readiness"
        fi
        if [[ "$process_kind" == windows ]] && ! game_pid_is_running; then
            die "$label exited before readiness"
        fi
        if probe_status "$host" "$port" "$path" "$expected" "$token"; then
            return 0
        fi
        sleep 0.2
    done
    die "$label did not become ready within ${startup_timeout_seconds}s"
}

wait_for_harness() {
    local attempts=$((harness_timeout_seconds * 10))
    local attempt
    local status

    for ((attempt = 0; attempt < attempts; attempt++)); do
        if ! posix_process_is_alive "$harness_pid"; then
            if wait "$harness_pid"; then
                return 0
            else
                status=$?
            fi
            die "harness exited with status $status"
        fi
        sleep 0.1
    done
    die "harness did not finish within ${harness_timeout_seconds}s"
}

install_addon() {
    local game_data_dir=$1
    local game_dir=$2
    local mods_dir=$3
    local stage_dir="$session_launcher_repo_root/.sts2-dev/runtime-session-addon"
    local backup_root="$session_launcher_repo_root/.sts2-dev/session-backups"
    local backup_dir
    local artifact
    local -a artifacts=(AIAscensionSTS2Poc.dll ai_ascension_sts2_poc.dll AIAscensionSTS2Poc.json)
    local -a existing_files=()

    game_is_running && die 'SlayTheSpire2.exe started while the addon was being prepared; restart required'
    bash "$session_launcher_package_script" "$game_data_dir" "$stage_dir" >/dev/null
    for artifact in "${artifacts[@]}"; do
        [[ -s "$stage_dir/$artifact" ]] || die "staged addon artifact is missing: $artifact"
    done

    mkdir -p "$mods_dir" "$backup_root"
    backup_dir=$(mktemp -d "$backup_root/session.XXXXXX")
    for artifact in "${artifacts[@]}"; do
        if [[ -f "$mods_dir/$artifact" ]]; then
            existing_files+=("$artifact")
            cp -p -- "$mods_dir/$artifact" "$backup_dir/$artifact"
        fi
    done

    for artifact in "${artifacts[@]}"; do
        if ! cp -f -- "$stage_dir/$artifact" "$mods_dir/$artifact"; then
            restore_addon "$mods_dir" "$backup_dir" "${artifacts[@]}"
            die "failed to install addon artifact: $artifact"
        fi
    done
    for artifact in "${artifacts[@]}"; do
        if ! cmp -s -- "$stage_dir/$artifact" "$mods_dir/$artifact"; then
            restore_addon "$mods_dir" "$backup_dir" "${artifacts[@]}"
            die "installed addon artifact did not match the staged artifact: $artifact"
        fi
    done
}

restore_addon() {
    local mods_dir=$1
    local backup_dir=$2
    shift 2
    local artifact
    for artifact in "$@"; do
        if [[ -f "$backup_dir/$artifact" ]]; then
            cp -f -- "$backup_dir/$artifact" "$mods_dir/$artifact"
        else
            rm -f -- "$mods_dir/$artifact"
        fi
    done
}

cleanup_owned_processes() {
    set +e
    if (( harness_started )); then
        stop_posix_group "$harness_group" "$harness_session" "$harness_pid"
    fi
    if (( gateway_started )); then
        stop_posix_group "$gateway_group" "$gateway_session" "$gateway_pid"
    fi
    if (( game_started )) && game_pid_is_running; then
        if ! "$taskkill_cmd" /PID "$game_pid" /T /F >/dev/null 2>&1; then
            cleanup_failed=1
        fi
        for ((cleanup_wait = 0; cleanup_wait < 100; cleanup_wait++)); do
            game_pid_is_running || break
            sleep 0.1
        done
        game_pid_is_running && cleanup_failed=1
    fi
    if (( gateway_started )) && probe_status "$gateway_host" "$gateway_port" /health/ready ANY ''; then
        cleanup_failed=1
    fi
    if (( game_started )) && probe_status "$game_probe_host" "$network_port" /health/ready ANY ''; then
        cleanup_failed=1
    fi
    unset runtime_token gateway_token STS2_PROBE_TOKEN
    if (( cleanup_failed )); then
        printf '%s\n' 'Owned session cleanup=FALSE' >&2
    else
        printf '%s\n' 'Owned session cleanup=TRUE'
    fi
}

on_exit() {
    local status=$1
    if (( ! cleanup_done )); then
        cleanup_done=1
        cleanup_owned_processes
    fi
    if (( status == 0 && cleanup_failed )); then
        status=1
    fi
    exit "$status"
}

self_test() {
    local first_runtime
    local second_runtime
    local first_gateway
    local second_gateway
    local fixture_pid
    local fixture_identity
    local fixture_group
    local fixture_session

    command -v openssl >/dev/null 2>&1 || die 'openssl is required for the CSPRNG self-test'
    command -v timeout >/dev/null 2>&1 || die 'timeout is required for the readiness self-test'
    first_runtime=$(new_credential) || die 'CSPRNG did not return a bounded credential'
    second_runtime=$(new_credential) || die 'CSPRNG did not return a second credential'
    first_gateway=$(new_credential) || die 'CSPRNG did not return a gateway credential'
    second_gateway=$(new_credential) || die 'CSPRNG did not return a second gateway credential'
    credential_is_safe "$first_runtime" || die 'runtime credential encoding/length check failed'
    credential_is_safe "$second_runtime" || die 'second runtime credential check failed'
    credential_is_safe "$first_gateway" || die 'gateway credential encoding/length check failed'
    [[ "$first_runtime" != "$second_runtime" ]] || die 'runtime credentials were reused'
    [[ "$first_gateway" != "$second_gateway" ]] || die 'gateway credentials were reused'
    [[ "$first_runtime" != "$first_gateway" ]] || die 'runtime and gateway credentials were reused'

    gateway_addr=127.0.0.1:15525
    mod_addr=127.0.0.1:15526
    runtime_token="$first_runtime"
    gateway_token="$first_gateway"
    mcp_binary=/bin/true
    export STS2_EXPECTED_RUNTIME="$runtime_token" STS2_EXPECTED_GATEWAY="$gateway_token"
    run_gateway_with_credentials bash -c '
            [[ -z "${STS2_RUNTIME_TOKEN:-}" ]] \
                && [[ "$STS2_MOD_TOKEN" == "$STS2_EXPECTED_RUNTIME" ]] \
                && [[ "$STS2_GATEWAY_TOKEN" == "$STS2_EXPECTED_GATEWAY" ]]
        ' || die 'gateway role separation failed'
    run_harness_with_credentials bash -c '
            [[ "$STS2_GATEWAY_TOKEN" == "$STS2_EXPECTED_GATEWAY" ]] \
                && [[ -z "${STS2_RUNTIME_TOKEN:-}" ]] \
                && [[ -z "${STS2_MOD_TOKEN:-}" ]]
        ' || die 'harness received an unexpected credential role'
    unset STS2_EXPECTED_RUNTIME STS2_EXPECTED_GATEWAY

    STS2_PROBE_STATUS_LINE='HTTP/1.1 401 Unauthorized' \
        authorization_status_matches 401 || die 'missing/wrong authorization rejection check failed'
    STS2_PROBE_STATUS_LINE='HTTP/1.1 200 OK' \
        authorization_status_matches 200 || die 'correct authorization acceptance check failed'
    STS2_PROBE_STATUS_LINE='HTTP/1.1 200 OK' \
        authorization_status_matches 401 && die 'wrong authorization was accepted'
    export STS2_EXPECTED_AUTHORIZATION="$runtime_token"
    unset STS2_PROBE_AUTHORIZATION
    authorization_header_matches && die 'missing authorization was accepted'
    STS2_PROBE_AUTHORIZATION='Bearer wrong'
    authorization_header_matches && die 'wrong authorization was accepted'
    STS2_PROBE_AUTHORIZATION="Bearer $runtime_token"
    authorization_header_matches || die 'correct authorization was rejected'
    unset STS2_EXPECTED_AUTHORIZATION STS2_PROBE_AUTHORIZATION

    setsid bash -c 'trap "exit 0" TERM INT; while :; do sleep 1; done' \
        </dev/null >/dev/null 2>&1 &
    fixture_pid=$!
    fixture_identity=$(record_process_identity "$fixture_pid") || die 'fixture process identity was unavailable'
    fixture_group=$(printf '%s\n' "$fixture_identity" | sed -n '1p')
    fixture_session=$(printf '%s\n' "$fixture_identity" | sed -n '2p')
    group_has_process "$fixture_group" "$fixture_session" \
        || die 'fixture process group was not observed'
    stop_posix_group "$fixture_group" "$fixture_session" "$fixture_pid"
    group_has_process "$fixture_group" "$fixture_session" \
        && die 'owned process group cleanup failed'

    printf '%s\n' \
        'CSPRNG=TRUE' \
        'Credential encoding=TRUE' \
        'Credential role separation=TRUE' \
        'Authorization fail-closed=TRUE' \
        'Owned process cleanup=TRUE' \
        'Token leakage=FALSE'
}

usage() {
    printf '%s\n' \
        'Usage: session-launcher.sh [options]' \
        '' \
        'Build/install the addon, create an ephemeral authenticated session, and' \
        'start the existing gateway, game, harness, and MCP binaries.' \
        '' \
        'Required provider inputs (one form per provider):' \
        '  --gateway-binary PATH  existing sts2-gateway-runtime executable' \
        '  --harness-binary PATH  existing sts2-harness-runtime executable' \
        '  --mcp-binary PATH      existing sts2-mcp-server executable' \
        '  --gateway-dir PATH     provider source directory; build its runtime binary' \
        '  --harness-dir PATH     provider source directory; build its runtime binary' \
        '  --mcp-dir PATH         provider source directory; build its runtime binary' \
        '' \
        'Options:' \
        '  --game-dir PATH        STS2 install directory' \
        '  --gateway-address EP   gateway listen endpoint (default 127.0.0.1:15525)' \
        '  --mod-address EP       game endpoint for gateway (default 127.0.0.1:<port>)' \
        '  --bind-address HOST    game listener bind address (default 127.0.0.1)' \
        '  --port N               game listener port, 1024-65535 (default 15526)' \
        '  --startup-timeout N    bounded readiness timeout in seconds (default 30)' \
        '  --harness-timeout N    bounded harness timeout in seconds (default 30)' \
        '  --windows-dotnet PATH  Windows dotnet.exe used by the bridge' \
        '  --bridge-dll PATH      prebuilt Windows bridge DLL' \
        '  --tasklist-command P   explicit tasklist.exe path' \
        '  --taskkill-command P   explicit taskkill.exe path' \
        '  --keep-alive           leave the owned session running until interrupted' \
        '  --self-test            run synthetic credential/auth/process tests' \
        '  -h, --help             show this help'
}

main() {
    local game_dir_input=${STS2_GAME_DIR:-}
    local gateway_dir_input=${STS2_RUNTIME_GATEWAY_DIR:-}
    local harness_dir_input=${STS2_RUNTIME_HARNESS_DIR:-}
    local mcp_dir_input=${STS2_RUNTIME_MCP_DIR:-}
    local gateway_binary_input=${STS2_RUNTIME_GATEWAY_BINARY:-}
    local harness_binary_input=${STS2_RUNTIME_HARNESS_BINARY:-}
    local mcp_binary_input=${STS2_RUNTIME_MCP_BINARY:-}
    local gateway_addr=${STS2_RUNTIME_GATEWAY_ADDR:-127.0.0.1:15525}
    local mod_addr_input=${STS2_RUNTIME_MOD_ADDR:-}
    local bind_address=${STS2_RUNTIME_BIND_ADDRESS:-127.0.0.1}
    network_port=${STS2_RUNTIME_PORT:-15526}
    local windows_dotnet_input=${STS2_WINDOWS_DOTNET_COMMAND:-${DOTNET_COMMAND:-}}
    local bridge_dll_input=''
    local tasklist_command_input=''
    local taskkill_command_input=''
    local keep_alive=false
    local self_test_requested=false
    local game_dir
    local game_data_dir
    local game_exe
    local mods_dir
    local gateway_identity
    local harness_identity
    local bridge_dll
    local bridge_dll_windows
    local bridge_project_windows
    local windows_dotnet
    local game_exe_windows
    local game_dir_windows
    local bridge_output
    local provider_dir
    local provider_binary
    local endpoint_parts

    while [[ $# -gt 0 ]]; do
        case $1 in
            --game-dir) game_dir_input=$(take_value "$1" "${2:-}"); shift 2 ;;
            --game-dir=*) game_dir_input=${1#*=}; shift ;;
            --gateway-dir) gateway_dir_input=$(take_value "$1" "${2:-}"); shift 2 ;;
            --gateway-dir=*) gateway_dir_input=${1#*=}; shift ;;
            --harness-dir) harness_dir_input=$(take_value "$1" "${2:-}"); shift 2 ;;
            --harness-dir=*) harness_dir_input=${1#*=}; shift ;;
            --mcp-dir) mcp_dir_input=$(take_value "$1" "${2:-}"); shift 2 ;;
            --mcp-dir=*) mcp_dir_input=${1#*=}; shift ;;
            --gateway-binary) gateway_binary_input=$(take_value "$1" "${2:-}"); shift 2 ;;
            --gateway-binary=*) gateway_binary_input=${1#*=}; shift ;;
            --harness-binary) harness_binary_input=$(take_value "$1" "${2:-}"); shift 2 ;;
            --harness-binary=*) harness_binary_input=${1#*=}; shift ;;
            --mcp-binary) mcp_binary_input=$(take_value "$1" "${2:-}"); shift 2 ;;
            --mcp-binary=*) mcp_binary_input=${1#*=}; shift ;;
            --gateway-address) gateway_addr=$(take_value "$1" "${2:-}"); shift 2 ;;
            --gateway-address=*) gateway_addr=${1#*=}; shift ;;
            --mod-address) mod_addr_input=$(take_value "$1" "${2:-}"); shift 2 ;;
            --mod-address=*) mod_addr_input=${1#*=}; shift ;;
            --bind-address) bind_address=$(take_value "$1" "${2:-}"); shift 2 ;;
            --bind-address=*) bind_address=${1#*=}; shift ;;
            --port) network_port=$(take_value "$1" "${2:-}"); shift 2 ;;
            --port=*) network_port=${1#*=}; shift ;;
            --startup-timeout) startup_timeout_seconds=$(take_value "$1" "${2:-}"); shift 2 ;;
            --startup-timeout=*) startup_timeout_seconds=${1#*=}; shift ;;
            --harness-timeout) harness_timeout_seconds=$(take_value "$1" "${2:-}"); shift 2 ;;
            --harness-timeout=*) harness_timeout_seconds=${1#*=}; shift ;;
            --windows-dotnet) windows_dotnet_input=$(take_value "$1" "${2:-}"); shift 2 ;;
            --windows-dotnet=*) windows_dotnet_input=${1#*=}; shift ;;
            --bridge-dll) bridge_dll_input=$(take_value "$1" "${2:-}"); shift 2 ;;
            --bridge-dll=*) bridge_dll_input=${1#*=}; shift ;;
            --tasklist-command) tasklist_command_input=$(take_value "$1" "${2:-}"); shift 2 ;;
            --tasklist-command=*) tasklist_command_input=${1#*=}; shift ;;
            --taskkill-command) taskkill_command_input=$(take_value "$1" "${2:-}"); shift 2 ;;
            --taskkill-command=*) taskkill_command_input=${1#*=}; shift ;;
            --keep-alive) keep_alive=true; shift ;;
            --self-test) self_test_requested=true; shift ;;
            -h|--help) usage; return 0 ;;
            *) die "unknown option: $1" ;;
        esac
    done

    if [[ "$self_test_requested" == true ]]; then
        self_test
        return 0
    fi

    unset STS2_RUNTIME_TOKEN STS2_GATEWAY_TOKEN STS2_MOD_TOKEN STS2_PROBE_TOKEN
    command -v openssl >/dev/null 2>&1 || die 'openssl is required for ephemeral credentials'
    command -v timeout >/dev/null 2>&1 || die 'timeout is required for bounded readiness'
    command -v setsid >/dev/null 2>&1 || die 'setsid is required for owned process groups'
    [[ -f "$session_launcher_package_script" ]] || die 'addon packaging script is missing'
    [[ -f "$session_launcher_bridge_project" ]] || die 'Windows session bridge project is missing'

    [[ "$startup_timeout_seconds" =~ ^[1-9][0-9]*$ ]] \
        || die '--startup-timeout must be a positive integer'
    [[ "$harness_timeout_seconds" =~ ^[1-9][0-9]*$ ]] \
        || die '--harness-timeout must be a positive integer'
    (( startup_timeout_seconds <= 300 )) || die '--startup-timeout must not exceed 300 seconds'
    (( harness_timeout_seconds <= 300 )) || die '--harness-timeout must not exceed 300 seconds'
    [[ "$network_port" =~ ^[0-9]+$ ]] \
        || die '--port must be an integer from 1024 through 65535'
    (( network_port >= 1024 && network_port <= 65535 )) \
        || die '--port must be an integer from 1024 through 65535'
    [[ "$bind_address" =~ ^[A-Za-z0-9.-]+$ ]] \
        || die '--bind-address must be an IPv4 address or hostname without whitespace'

    endpoint_parts=$(parse_endpoint "$gateway_addr") \
        || die '--gateway-address must be HOST:PORT with a port from 1 through 65535'
    gateway_host=$(printf '%s\n' "$endpoint_parts" | sed -n '1p')
    gateway_port=$(printf '%s\n' "$endpoint_parts" | sed -n '2p')
    mod_addr=${mod_addr_input:-127.0.0.1:$network_port}
    endpoint_parts=$(parse_endpoint "$mod_addr") \
        || die '--mod-address must be HOST:PORT with a port from 1 through 65535'
    mod_host=$(printf '%s\n' "$endpoint_parts" | sed -n '1p')
    mod_port=$(printf '%s\n' "$endpoint_parts" | sed -n '2p')
    case "$bind_address" in
        0.0.0.0) game_probe_host=127.0.0.1 ;;
        *) game_probe_host=$bind_address ;;
    esac

    if [[ -z "$game_dir_input" ]]; then
        for candidate in \
            '/mnt/c/Program Files (x86)/Steam/steamapps/common/Slay the Spire 2' \
            '/mnt/c/Program Files/Steam/steamapps/common/Slay the Spire 2'; do
            if [[ -d "$candidate" ]]; then
                game_dir_input=$candidate
                break
            fi
        done
    fi
    [[ -n "$game_dir_input" ]] || die 'game directory is missing; pass --game-dir or set STS2_GAME_DIR'
    game_dir=$(to_wsl_path "$game_dir_input")
    [[ -d "$game_dir" ]] || die "game directory does not exist: $game_dir_input"
    game_dir=$(cd -- "$game_dir" && pwd -P)
    game_data_dir="$game_dir/data_sts2_windows_x86_64"
    game_exe="$game_dir/SlayTheSpire2.exe"
    mods_dir="$game_dir/mods"
    [[ -f "$game_exe" ]] || die "SlayTheSpire2.exe was not found under: $game_dir"
    [[ -f "$game_data_dir/sts2.dll" ]] || die "sts2.dll was not found under: $game_data_dir"
    [[ -f "$game_data_dir/GodotSharp.dll" ]] || die "GodotSharp.dll was not found under: $game_data_dir"

    if [[ -n "$tasklist_command_input" ]]; then
        tasklist_cmd=$(resolve_executable "$(to_wsl_path "$tasklist_command_input")") \
            || die "tasklist command is unavailable: $tasklist_command_input"
    else
        tasklist_cmd=$(find_windows_tool tasklist.exe) \
            || die 'tasklist.exe is unavailable; run the launcher from WSL on Windows'
    fi
    if [[ -n "$taskkill_command_input" ]]; then
        taskkill_cmd=$(resolve_executable "$(to_wsl_path "$taskkill_command_input")") \
            || die "taskkill command is unavailable: $taskkill_command_input"
    else
        taskkill_cmd=$(find_windows_tool taskkill.exe) \
            || die 'taskkill.exe is unavailable; run the launcher from WSL on Windows'
    fi
    refuse_if_game_running

    if [[ -n "$gateway_binary_input" ]]; then
        gateway_binary=$(resolve_executable "$(to_wsl_path "$gateway_binary_input")") \
            || die "gateway binary is unavailable: $gateway_binary_input"
    else
        [[ -n "$gateway_dir_input" ]] || die 'pass --gateway-binary or --gateway-dir'
        provider_dir=$(to_wsl_path "$gateway_dir_input")
        [[ -f "$provider_dir/Cargo.toml" ]] || die "gateway Cargo.toml is missing: $gateway_dir_input"
        cargo build --locked --manifest-path "$provider_dir/Cargo.toml" --bin sts2-gateway-runtime \
            >/dev/null
        gateway_binary="$provider_dir/target/debug/sts2-gateway-runtime"
    fi
    [[ -x "$gateway_binary" ]] || die "gateway runtime binary is not executable: $gateway_binary"

    if [[ -n "$harness_binary_input" ]]; then
        harness_binary=$(resolve_executable "$(to_wsl_path "$harness_binary_input")") \
            || die "harness binary is unavailable: $harness_binary_input"
    else
        [[ -n "$harness_dir_input" ]] || die 'pass --harness-binary or --harness-dir'
        provider_dir=$(to_wsl_path "$harness_dir_input")
        [[ -f "$provider_dir/Cargo.toml" ]] || die "harness Cargo.toml is missing: $harness_dir_input"
        cargo build --locked --manifest-path "$provider_dir/Cargo.toml" --bin sts2-harness-runtime \
            >/dev/null
        harness_binary="$provider_dir/target/debug/sts2-harness-runtime"
    fi
    [[ -x "$harness_binary" ]] || die "harness runtime binary is not executable: $harness_binary"

    if [[ -n "$mcp_binary_input" ]]; then
        mcp_binary=$(resolve_executable "$(to_wsl_path "$mcp_binary_input")") \
            || die "MCP binary is unavailable: $mcp_binary_input"
    else
        [[ -n "$mcp_dir_input" ]] || die 'pass --mcp-binary or --mcp-dir'
        provider_dir=$(to_wsl_path "$mcp_dir_input")
        [[ -f "$provider_dir/Cargo.toml" ]] || die "MCP Cargo.toml is missing: $mcp_dir_input"
        cargo build --locked --manifest-path "$provider_dir/Cargo.toml" --bin sts2-mcp-server \
            >/dev/null
        mcp_binary="$provider_dir/target/debug/sts2-mcp-server"
    fi
    [[ -x "$mcp_binary" ]] || die "MCP runtime binary is not executable: $mcp_binary"

    if [[ -n "$windows_dotnet_input" ]]; then
        if [[ "$windows_dotnet_input" == /* ]]; then
            windows_dotnet=$(to_wsl_path "$windows_dotnet_input")
        else
            windows_dotnet=$(resolve_executable "$windows_dotnet_input") \
                || die "Windows dotnet command is unavailable: $windows_dotnet_input"
        fi
    else
        windows_dotnet='/mnt/c/Program Files/dotnet/dotnet.exe'
    fi
    [[ -x "$windows_dotnet" && "$windows_dotnet" == *.exe ]] \
        || die "a Windows dotnet.exe is required for the WSL-to-Windows environment bridge: $windows_dotnet"

    if [[ -n "$bridge_dll_input" ]]; then
        bridge_dll=$(to_wsl_path "$bridge_dll_input")
    else
        bridge_project_windows=$(to_windows_path "$session_launcher_bridge_project")
        "$windows_dotnet" build "$bridge_project_windows" --configuration Release --nologo >/dev/null
        bridge_dll="$session_launcher_bridge_dll"
    fi
    [[ -s "$bridge_dll" ]] || die "session bridge DLL is missing: $bridge_dll"
    bridge_dll_windows=$(to_windows_path "$bridge_dll")
    game_exe_windows=$(to_windows_path "$game_exe")
    game_dir_windows=$(to_windows_path "$game_dir")

    install_addon "$game_data_dir" "$game_dir" "$mods_dir"

    runtime_token=$(new_credential) || die 'could not generate the ephemeral runtime credential'
    gateway_token=$(new_credential) || die 'could not generate the ephemeral gateway credential'
    credential_is_safe "$runtime_token" || die 'generated runtime credential failed validation'
    credential_is_safe "$gateway_token" || die 'generated gateway credential failed validation'
    [[ "$runtime_token" != "$gateway_token" ]] || die 'generated credential roles were not distinct'

    trap 'on_exit "$?"' EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM

    STS2_GATEWAY_ADDR="$gateway_addr" \
        STS2_MOD_ADDR="$mod_addr" \
        STS2_GATEWAY_TOKEN="$gateway_token" \
        STS2_MOD_TOKEN="$runtime_token" \
        STS2_INSTANCE_ID=instance-1 \
        STS2_CALLER_ID=harness \
        STS2_SESSION_ID=session-1 \
        STS2_LEASE_ID=lease-1 \
        STS2_LEASE_EPOCH=1 \
        setsid "$gateway_binary" \
        </dev/null >/dev/null 2>&1 &
    gateway_pid=$!
    gateway_started=1
    gateway_identity=$(record_process_identity "$gateway_pid") \
        || die 'gateway process identity could not be recorded'
    gateway_group=$(printf '%s\n' "$gateway_identity" | sed -n '1p')
    gateway_session=$(printf '%s\n' "$gateway_identity" | sed -n '2p')
    wait_for_probe 'gateway listener' "$gateway_host" "$gateway_port" /health/ready 401 '' posix "$gateway_pid"

    bridge_output=$(printf '%s\n' "$runtime_token" \
        | "$windows_dotnet" "$bridge_dll_windows" \
            --game-executable "$game_exe_windows" \
            --working-directory "$game_dir_windows" \
            --bind-address "$bind_address" \
            --port "$network_port" 2>/dev/null) \
        || die 'Windows game launch bridge failed'
    game_pid=$(printf '%s\n' "$bridge_output" | tr -d '\r' \
        | awk -F= '$1 == "PID" && $2 ~ /^[0-9]+$/ { print $2; exit }')
    [[ "$bridge_output" == *'STARTED=TRUE'* && "$game_pid" =~ ^[0-9]+$ ]] \
        || die 'Windows game launch bridge did not confirm a game process'
    game_started=1

    wait_for_probe 'game listener' "$game_probe_host" "$network_port" /health/ready 401 '' windows "$game_pid"
    wait_for_probe 'authenticated game listener' "$game_probe_host" "$network_port" /health/ready 200 \
        "$runtime_token" windows "$game_pid"
    wait_for_probe 'authenticated gateway' "$gateway_host" "$gateway_port" /health/ready 200 \
        "$gateway_token" posix "$gateway_pid"

    STS2_GATEWAY_ADDR="$gateway_addr" \
        STS2_GATEWAY_TOKEN="$gateway_token" \
        STS2_MCP_BINARY="$mcp_binary" \
        STS2_INSTANCE_ID=instance-1 \
        STS2_CALLER_ID=harness \
        STS2_SESSION_ID=session-1 \
        STS2_LEASE_ID=lease-1 \
        STS2_LEASE_EPOCH=1 \
        STS2_MCP_SESSION_ID=mcp-session-1 \
        setsid "$harness_binary" \
        </dev/null >/dev/null 2>&1 &
    harness_pid=$!
    harness_started=1
    harness_identity=$(record_process_identity "$harness_pid") \
        || die 'harness process identity could not be recorded'
    harness_group=$(printf '%s\n' "$harness_identity" | sed -n '1p')
    harness_session=$(printf '%s\n' "$harness_identity" | sed -n '2p')
    wait_for_harness

    printf '%s\n' \
        'Token configured=TRUE' \
        'Listener enabled=TRUE' \
        'Gateway authenticated=TRUE' \
        'Harness ready=TRUE'
    if [[ "$keep_alive" == true ]]; then
        while game_pid_is_running; do
            sleep 1
        done
        die 'owned game process exited while the session was running'
    fi
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    main "$@"
fi
