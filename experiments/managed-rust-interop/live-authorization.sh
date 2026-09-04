#!/usr/bin/env bash

# This file is sourced by the owned launch paths after their local `die`
# function is defined. It deliberately accepts only non-secret authorization
# metadata; credentials are generated later by the launcher.

live_authorization_variables=(
    STS2_LIVE_AUTHORIZATION_SCOPE
    STS2_LIVE_AUTHORIZATION_HOST_IDENTITY
    STS2_LIVE_AUTHORIZATION_HOST_INSTALL_LABEL
    STS2_LIVE_AUTHORIZATION_PROFILE_IDENTITY
    STS2_LIVE_AUTHORIZATION_PROCESS_ACTIONS
    STS2_LIVE_AUTHORIZATION_PROFILE_MUTATIONS
    STS2_LIVE_AUTHORIZATION_LISTENER_ACTIONS
    STS2_LIVE_AUTHORIZATION_NETWORK_ACTIONS
    STS2_LIVE_AUTHORIZATION_CLEANUP_OWNER
    STS2_LIVE_AUTHORIZATION_RESTORE_POINT
    STS2_LIVE_AUTHORIZATION_EXPIRY_EPOCH
    STS2_LIVE_AUTHORIZATION_PUBLICATION_AUTHORITY
    STS2_LIVE_AUTHORIZATION_PROVIDER_CALLS
)

live_authorization_has_phrase() {
    local values=${1:-}
    local phrase=${2:-}

    values=${values,,}
    phrase=${phrase,,}
    values=${values//,/ }
    values=${values//|/ }
    values=${values//;/ }
    values=${values//$'\t'/ }
    while [[ "$values" == *'  '* ]]; do
        values=${values//  / }
    done
    [[ " $values " == *" $phrase "* ]]
}

validate_live_authorization() {
    local name
    local value
    local scope
    local process_actions
    local profile_mutations
    local listener_actions
    local network_actions
    local deadline
    local now

    [[ ${STS2_LIVE_AUTHORIZATION_APPROVED:-no} == yes ]] \
        || die 'LIVE_AUTHORIZATION approved=yes is required before a live launch'

    for name in "${live_authorization_variables[@]}"; do
        value=${!name:-}
        [[ -n "$value" ]] || die "LIVE_AUTHORIZATION field is missing: $name"
        (( ${#value} <= 1024 )) || die "LIVE_AUTHORIZATION field is too long: $name"
        [[ "$value" != *$'\n'* && "$value" != *$'\r'* ]] \
            || die "LIVE_AUTHORIZATION field contains a line break: $name"
    done

    scope=${STS2_LIVE_AUTHORIZATION_SCOPE}
    process_actions=${STS2_LIVE_AUTHORIZATION_PROCESS_ACTIONS}
    profile_mutations=${STS2_LIVE_AUTHORIZATION_PROFILE_MUTATIONS}
    listener_actions=${STS2_LIVE_AUTHORIZATION_LISTENER_ACTIONS}
    network_actions=${STS2_LIVE_AUTHORIZATION_NETWORK_ACTIONS}
    deadline=${STS2_LIVE_AUTHORIZATION_EXPIRY_EPOCH}

    live_authorization_has_phrase "$scope" live \
        || die 'LIVE_AUTHORIZATION scope must state the live operation'
    live_authorization_has_phrase "$scope" runtime-v2 \
        || die 'LIVE_AUTHORIZATION scope must name Runtime-v2'
    for value in install launch stop terminate; do
        live_authorization_has_phrase "$process_actions" "$value" \
            || die "LIVE_AUTHORIZATION process actions must authorize: $value"
    done
    live_authorization_has_phrase "$profile_mutations" disposable \
        || die 'LIVE_AUTHORIZATION profile mutations must name a disposable profile'
    live_authorization_has_phrase "$listener_actions" 'bind loopback' \
        || die 'LIVE_AUTHORIZATION listener actions must authorize loopback bind'
    live_authorization_has_phrase "$listener_actions" 'connect loopback' \
        || die 'LIVE_AUTHORIZATION listener actions must authorize loopback connect'
    [[ "$network_actions" == 'loopback only' ]] \
        || die 'LIVE_AUTHORIZATION network actions must authorize loopback only'

    [[ "$deadline" =~ ^[1-9][0-9]{0,11}$ ]] \
        || die 'LIVE_AUTHORIZATION expiry_or_cleanup_deadline must be an epoch integer'
    now=${EPOCHSECONDS:-0}
    [[ "$now" =~ ^[0-9]+$ && "$now" != 0 ]] \
        || die 'Bash EPOCHSECONDS is unavailable for the authorization deadline check'
    (( deadline > now )) \
        || die 'LIVE_AUTHORIZATION expiry_or_cleanup_deadline has passed'
    [[ ${STS2_LIVE_AUTHORIZATION_PROVIDER_CALLS} == prohibited ]] \
        || die 'provider calls are prohibited unless a separate authorization seam is approved'

    # Keep only the deadline locally, never in the child environment. Recheck
    # admission after builds and while supervising owned processes.
    live_authorization_deadline=$deadline
    export -n live_authorization_deadline

    # Do not let authorization metadata leak into any later child environment.
    unset STS2_LIVE_AUTHORIZATION_APPROVED
    unset "${live_authorization_variables[@]}"
}

assert_live_authorization_current() {
    [[ ${live_authorization_deadline:-} =~ ^[1-9][0-9]{0,11}$ ]] \
        || die 'LIVE_AUTHORIZATION has not been validated'
    (( EPOCHSECONDS < live_authorization_deadline )) \
        || die 'LIVE_AUTHORIZATION deadline passed; stopping owned work'
}

run_with_live_authorization() {
    assert_live_authorization_current
    local remaining=$((live_authorization_deadline - EPOCHSECONDS))
    (( remaining > 0 )) || die 'LIVE_AUTHORIZATION deadline passed before command admission'
    command -v timeout >/dev/null 2>&1 || die 'timeout is required for bounded authorized work'
    timeout --kill-after=2 "$remaining" "$@"
}
