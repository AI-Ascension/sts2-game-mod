#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

die() { printf 'two-peer preparation: %s\n' "$*" >&2; exit 2; }
usage() {
    cat >&2 <<'EOF'
usage: prepare-native-coop-two-peer.sh --host-game-dir DIR --client-game-dir DIR \
  --host-package-dir DIR --client-package-dir DIR --host-profile-id LABEL \
  --client-profile-id LABEL --port PORT --client-id ID --output-dir DIR

This writes a reviewable, no-launch plan and two environment files. It never copies addons,
starts Steam/the game, opens a socket, reads a save, or changes a profile.
EOF
    exit 2
}

host_game='' client_game='' host_package='' client_package='' host_profile='' client_profile=''
port='' client_id='' output=''
while [[ $# -gt 0 ]]; do
    case "$1" in
        --host-game-dir) host_game=${2:?}; shift 2 ;;
        --client-game-dir) client_game=${2:?}; shift 2 ;;
        --host-package-dir) host_package=${2:?}; shift 2 ;;
        --client-package-dir) client_package=${2:?}; shift 2 ;;
        --host-profile-id) host_profile=${2:?}; shift 2 ;;
        --client-profile-id) client_profile=${2:?}; shift 2 ;;
        --port) port=${2:?}; shift 2 ;;
        --client-id) client_id=${2:?}; shift 2 ;;
        --output-dir) output=${2:?}; shift 2 ;;
        -h|--help) usage ;;
        *) die "unknown argument: $1" ;;
    esac
done
[[ -n $host_game && -n $client_game && -n $host_package && -n $client_package && -n $host_profile \
    && -n $client_profile && -n $port && -n $client_id && -n $output ]] || usage
[[ $port =~ ^[0-9]+$ && $port -ge 1024 && $port -le 65535 ]] || die 'port must be 1024..65535'
[[ $client_id =~ ^[1-9][0-9]*$ ]] || die 'client ID must be a nonzero decimal native ID'
[[ $host_profile =~ ^[A-Za-z0-9._:-]+$ && $client_profile =~ ^[A-Za-z0-9._:-]+$ ]] \
    || die 'profile IDs are labels and must be simple non-secret identities'

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)
host_game=$(realpath -e -- "$host_game")
client_game=$(realpath -e -- "$client_game")
host_package=$(realpath -e -- "$host_package")
client_package=$(realpath -e -- "$client_package")
output=$(realpath -m -- "$output")
[[ $host_game != "$client_game" ]] || die 'host and client game directories must be distinct disposable copies'
[[ $output/ != "$repo_root/"* ]] || die 'output directory must be outside the repository'
[[ ! -e $output ]] || die 'output directory must be new and empty'
for game in "$host_game" "$client_game"; do
    [[ -f $game/SlayTheSpire2.exe && ! -L $game/SlayTheSpire2.exe ]] \
        || die "expected a regular SlayTheSpire2.exe under $game"
done

platform_files=(AIAscensionSTS2GameMod.dll AIAscensionSTS2GameMod.json)
for package in "$host_package" "$client_package"; do
    for artifact in "${platform_files[@]}" AIAscensionSTS2GameModNative.dll; do
        [[ -f $package/$artifact && ! -L $package/$artifact && -s $package/$artifact ]] \
            || die "missing regular Windows artifact: $package/$artifact"
    done
done
for artifact in "${platform_files[@]}" AIAscensionSTS2GameModNative.dll; do
    cmp -s -- "$host_package/$artifact" "$client_package/$artifact" \
        || die "host/client package differs: $artifact"
done

mkdir -p -- "$output"
umask 077
sha256sum "$host_game/SlayTheSpire2.exe" "$client_game/SlayTheSpire2.exe" \
    "$host_package/AIAscensionSTS2GameMod.dll" "$host_package/AIAscensionSTS2GameMod.json" \
    "$host_package/AIAscensionSTS2GameModNative.dll" > "$output/sha256sums.txt"
cat > "$output/host.env" <<EOF
STS2_NATIVE_COOP_AUTOSTART_ROLE=host
STS2_NATIVE_COOP_ENET_PORT=$port
STS2_NATIVE_COOP_HOST_ADDRESS=127.0.0.1
STS2_NATIVE_COOP_MAX_PLAYERS=2
STS2_NATIVE_COOP_AUTO_ADMIT_RUN=false
EOF
cat > "$output/client.env" <<EOF
STS2_NATIVE_COOP_AUTOSTART_ROLE=client
STS2_NATIVE_COOP_ENET_PORT=$port
STS2_NATIVE_COOP_HOST_ADDRESS=127.0.0.1
STS2_NATIVE_COOP_CLIENT_ID=$client_id
STS2_NATIVE_COOP_MAX_PLAYERS=2
STS2_NATIVE_COOP_AUTO_ADMIT_RUN=false
EOF
cat > "$output/two-peer-plan.md" <<EOF
# Two-peer native session plan (not executed)

Host game identity: \`$host_game\`
Client game identity: \`$client_game\`
Host profile label: \`$host_profile\`
Client profile label: \`$client_profile\`
Loopback ENet port: \`$port\`
Expected client native ID: \`$client_id\`

The package hash receipt is \`sha256sums.txt\`. The root operator must first copy each listed
package into its matching disposable game's \`mods/\` directory with a reversible backup; this
preparation script does not perform that mutation.

When separately authorized, launch the host using \`host.env\`, wait for the source-defined log
\`host_character_lobby_ready\` and record \`local_net_id\`. Launch the client using \`client.env\`.
The client deliberately omits a guessed host ID: JoinFlow obtains the actual \`HostNetId\`, and
the controller requires a connected service, distinct IDs, and the same lobby service before it
emits \`client native lobby identity confirmed\`.

Readiness mapping: host \`StartENetHost(port, 2)\` -> \`host_character_lobby_ready\`; client
\`ENetClientConnectionInitializer(clientId, 127.0.0.1, port)\` + \`JoinFlow.Begin\` ->
\`client_character_lobby_ready\`; both remain manual because auto-admission is false. The client
carrier then accepts only replies whose sender equals the actual connected \`HostNetId\`. The host
dispatcher maps a callback native sender ID to its canonical opaque actor, validates the frozen
operation ID, and returns its host receipt only to that same native peer.

No mouse/keyboard automation is authorized. Do not mark this plan successful until a real host
observation shows convergence and a client-originated action or map vote has one host-issued,
same-operation reply and a settled post-action observation.
EOF
printf 'Prepared review-only two-peer plan: %s\n' "$output"
