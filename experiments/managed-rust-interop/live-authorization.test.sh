#!/usr/bin/env bash
set -Eeuo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
die() { printf '%s\n' "$1" >&2; exit 1; }
source "$script_dir/live-authorization.sh"
set_record() {
    local name
    for name in "${live_authorization_variables[@]}"; do
        export "$name=synthetic"
    done
    export STS2_LIVE_AUTHORIZATION_APPROVED=yes
    export STS2_LIVE_AUTHORIZATION_SCOPE='runtime-v2 live'
    export STS2_LIVE_AUTHORIZATION_PROCESS_ACTIONS='install launch stop terminate'
    export STS2_LIVE_AUTHORIZATION_PROFILE_MUTATIONS='disposable profile'
    export STS2_LIVE_AUTHORIZATION_LISTENER_ACTIONS='bind loopback connect loopback'
    export STS2_LIVE_AUTHORIZATION_NETWORK_ACTIONS='loopback only'
    export STS2_LIVE_AUTHORIZATION_EXPIRY_EPOCH=$((EPOCHSECONDS + 60))
    export STS2_LIVE_AUTHORIZATION_PROVIDER_CALLS=prohibited
}
set_record
validate_live_authorization
for name in "${live_authorization_variables[@]}" STS2_LIVE_AUTHORIZATION_APPROVED; do
    [[ ! -v "$name" ]] || die 'authorization metadata retained'
done
if env | rg -q '^live_authorization_deadline='; then die 'deadline exported'; fi
for name in "${live_authorization_variables[@]}"; do
    if (set_record; unset "$name"; validate_live_authorization) >/dev/null 2>&1; then
        die "missing field accepted: $name"
    fi
done
for invalid in 0 00099999999999 9999999999999 -1 not-a-date "$EPOCHSECONDS"; do
    if (set_record; export STS2_LIVE_AUTHORIZATION_EXPIRY_EPOCH=$invalid; validate_live_authorization) >/dev/null 2>&1; then
        die 'invalid or expired deadline accepted'
    fi
done
if (set_record; STS2_LIVE_AUTHORIZATION_NETWORK_ACTIONS='loopback internet'; validate_live_authorization) >/dev/null 2>&1; then
    die 'expanded network authority accepted'
fi
if (set_record; STS2_LIVE_AUTHORIZATION_PROVIDER_CALLS=allowed; validate_live_authorization) >/dev/null 2>&1; then
    die 'provider authority accepted'
fi
live_authorization_deadline=$((EPOCHSECONDS + 1))
if run_with_live_authorization sleep 3; then die 'work outlived authorization'; fi
printf 'Authorization missing fields, expiry, scope, metadata removal, bounded build=TRUE\n'
