#!/usr/bin/env bash
# Sourced by session-launcher.sh. The guardian owns its Windows job for the whole
# session; the shell owns only its recorded subprocess and bounded pipe protocol.
bridge_pid=''
bridge_group=''
bridge_session=''
bridge_input_fd=''
bridge_output_fd=''
bridge_started=0

close_bridge_pipes() {
    if [[ -n "$bridge_input_fd" ]]; then exec {bridge_input_fd}>&-; bridge_input_fd=''; fi
    if [[ -n "$bridge_output_fd" ]]; then exec {bridge_output_fd}<&-; bridge_output_fd=''; fi
}

stop_bridge_guardian() {
    local missing_receipt=0
    if (( bridge_started && ! game_started )); then missing_receipt=1; fi
    close_bridge_pipes
    if (( bridge_started )); then
        stop_posix_group "$bridge_group" "$bridge_session" "$bridge_pid" || return 1
        bridge_started=0
    fi
    # Job ownership makes cancellation safe, but an absent receipt is not
    # independent evidence of Windows process exit across the WSL boundary.
    (( missing_receipt == 0 ))
}

launch_owned_bridge() {
    assert_live_authorization_current
    local lease_seconds=$((live_authorization_deadline - EPOCHSECONDS))
    (( lease_seconds > 0 )) || die 'authorization expired before bridge launch'
    (( lease_seconds <= 3600 )) || lease_seconds=3600
    local allowance=$startup_timeout_seconds
    (( allowance <= lease_seconds )) || allowance=$lease_seconds
    local handshake_deadline=$((SECONDS + allowance))
    local identity original_input original_output remaining line field
    local -a receipt=()

    coproc STS2_BRIDGE { exec setsid "$@" --lease-seconds "$lease_seconds" 2>/dev/null; }
    bridge_pid=$STS2_BRIDGE_PID
    bridge_started=1
    original_output=${STS2_BRIDGE[0]}
    original_input=${STS2_BRIDGE[1]}
    exec {bridge_output_fd}<&"$original_output"
    exec {bridge_input_fd}>&"$original_input"
    exec {original_output}<&-
    exec {original_input}>&-
    identity=$(record_process_identity "$bridge_pid") || die 'bridge process identity unavailable'
    bridge_group=$(printf '%s\n' "$identity" | sed -n '1p')
    bridge_session=$(printf '%s\n' "$identity" | sed -n '2p')
    # One bounded credential line fits an empty POSIX pipe even if the child
    # has not read yet. No subsequent writes are needed: closing stdin cancels.
    printf '%s\n' "$runtime_token" >&"$bridge_input_fd" || die 'bridge credential pipe failed'
    for field in STARTED PID START_TICKS; do
        remaining=$((handshake_deadline - SECONDS))
        (( remaining > 0 )) || die 'Windows game launch handoff timed out'
        line=''
        IFS= read -r -n 128 -t "$remaining" line <&"$bridge_output_fd" \
            || die 'Windows game launch handoff timed out or ended early'
        line=${line%$'\r'}
        (( ${#line} < 128 )) || die 'Windows game launch receipt exceeded its bound'
        receipt+=("$line")
    done
    [[ ${receipt[0]} == STARTED=TRUE && ${receipt[1]} =~ ^PID=([1-9][0-9]*)$ ]] \
        || die 'Windows game launch receipt was invalid'
    game_pid=${BASH_REMATCH[1]}
    [[ ${receipt[2]} =~ ^START_TICKS=([1-9][0-9]*)$ ]] \
        || die 'Windows game launch receipt lacked process identity'
    game_start_ticks=${BASH_REMATCH[1]}
    game_started=1
}

build_provider_binary() {
    local provider_directory=$1
    local provider_name=$2
    local executable
    command -v jq >/dev/null 2>&1 || die 'jq is required to locate compiled provider artifacts'
    executable=$(run_with_live_authorization cargo build --locked \
        --manifest-path "$provider_directory/Cargo.toml" --bin "$provider_name" --message-format=json \
        | jq -er --arg name "$provider_name" 'select(.reason == "compiler-artifact" and
            .target.name == $name and (.target.kind | index("bin")) and .executable != null) | .executable') \
        || die 'provider build did not identify its executable'
    [[ "$executable" == /* && "$executable" != *$'\n'* && -x "$executable" ]] \
        || die 'provider build returned an unavailable executable'
    printf '%s' "$executable"
}
