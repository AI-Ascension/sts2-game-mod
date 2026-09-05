#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
host_dir='' user_dir='' artifacts='' gateway='' mcp='' harness='' provider=''
powershell=powershell.exe
display=-1 width=1280 height=720 mode=windowed seed=AIASCENSIONREPLAY1 hold=300 replay=''
usage() {
    cat <<'HELP'
Usage: live-combat-session.sh --host-dir PATH --user-dir WINDOWS_PATH --artifacts-dir PATH
  --gateway-binary PATH --mcp-binary PATH --harness-binary PATH --provider-binary PATH
  [--display N] [--width N] [--height N]
  [--window-mode windowed|fullscreen|borderless|maximized] [--seed VALUE]
  [--hold-seconds 300] [--replay-trajectory PATH]
  [--powershell-binary PATH]
Requires an already prepared disposable host, accepted addon, Windows PowerShell,
WSL, curl, jq and openssl. Display indexes are zero-based; -1 selects the primary display.
No installation is performed.
HELP
}
while (($#)); do
    case "$1" in
        --help) usage; exit 0 ;;
        --host-dir) host_dir=${2:?}; shift ;;
        --user-dir) user_dir=${2:?}; shift ;;
        --artifacts-dir) artifacts=${2:?}; shift ;;
        --gateway-binary) gateway=${2:?}; shift ;;
        --mcp-binary) mcp=${2:?}; shift ;;
        --harness-binary) harness=${2:?}; shift ;;
        --provider-binary) provider=${2:?}; shift ;;
        --display) display=${2:?}; shift ;;
        --width) width=${2:?}; shift ;;
        --height) height=${2:?}; shift ;;
        --window-mode) mode=${2:?}; shift ;;
        --seed) seed=${2:?}; shift ;;
        --hold-seconds) hold=${2:?}; shift ;;
        --replay-trajectory) replay=${2:?}; shift ;;
        --powershell-binary) powershell=${2:?}; shift ;;
        *) printf 'Unknown option: %s\n' "$1" >&2; exit 2 ;;
    esac
    shift
done
for binary in "$gateway" "$mcp" "$harness" "$provider"; do
    [[ -x "$binary" ]] || { printf 'Missing executable\n' >&2; exit 2; }
done
[[ -f "$host_dir/override.cfg" && -n "$user_dir" && -n "$artifacts" ]] || exit 2
[[ "$display" =~ ^(-1|[0-9]+)$ && "$width" =~ ^[0-9]+$ && "$height" =~ ^[0-9]+$ ]] || exit 2
[[ "$hold" =~ ^[0-9]+$ && "$hold" -le 600 ]] || exit 2
[[ "$seed" =~ ^[A-Za-z0-9_-]{1,128}$ ]] || exit 2
case "$mode" in windowed|fullscreen|borderless|maximized) ;; *) exit 2 ;; esac
[[ -z "$replay" || -f "$replay" ]] || exit 2
for command in wslpath curl jq openssl "$powershell"; do command -v "$command" >/dev/null; done
umask 077
repo_root=$(cd -- "$script_dir/../.." && pwd -P)
artifacts=$(realpath -m -- "$artifacts")
case "$artifacts/" in "$repo_root/"*) printf 'Artifacts must be outside the repository\n' >&2; exit 2 ;; esac
mkdir -p -- "$artifacts"
run=$(mktemp -d "$artifacts/run-XXXXXXXX")
runtime_token=$(openssl rand -hex 32)
gateway_token=$(openssl rand -hex 32)
host_pid='' gateway_pid='' harness_pid=''
cleanup() {
    touch "$run/stop"
    for pid in "$harness_pid" "$gateway_pid"; do
        if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null || true
            wait "$pid" 2>/dev/null || true
        fi
    done
    if [[ -n "$host_pid" ]]; then wait "$host_pid" || printf 'Host guardian failed\n' >&2; fi
}
trap cleanup EXIT
trap 'exit 130' INT TERM
"$powershell" -NoProfile -File "$(wslpath -w "$script_dir/live-combat-demo.ps1")" \
    -HostDirectory "$(wslpath -w "$host_dir")" -UserDirectory "$user_dir" \
    -LogPath "$(wslpath -w "$run/game.log")" -StopFile "$(wslpath -w "$run/stop")" \
    -Seed "$seed" -Display "$display" -Width "$width" -Height "$height" -WindowMode "$mode" \
    <<<"$runtime_token" >"$run/guardian.log" 2>&1 &
host_pid=$!
printf 'Artifacts: %s\n' "$run"
ready=false
for ((attempt=0; attempt<90; attempt++)); do
    kill -0 "$host_pid" 2>/dev/null || { printf 'Host exited before readiness\n' >&2; exit 2; }
    status=$(curl --silent --max-time 1 --http1.1 --output /dev/null --write-out '%{http_code}' \
        --header 'Accept:' --user-agent '' --config - <<CURL || true
url = "http://127.0.0.1:15626/health/ready"
header = "Authorization: Bearer $runtime_token"
CURL
)
    if [[ "$status" == 200 ]]; then ready=true; break; fi
    sleep 1
done
[[ "$ready" == true ]] || { printf 'Host readiness timed out\n' >&2; exit 2; }
export STS2_GATEWAY_ADDR=127.0.0.1:15625 STS2_MOD_ADDR=127.0.0.1:15626
export STS2_GATEWAY_TOKEN="$gateway_token" STS2_INSTANCE_ID=demo-instance STS2_CALLER_ID=harness
export STS2_SESSION_ID=demo-session STS2_MCP_SESSION_ID=demo-mcp-session
export STS2_LEASE_ID=demo-lease STS2_LEASE_EPOCH=1
STS2_MOD_TOKEN="$runtime_token" "$gateway" >"$run/gateway.log" 2>&1 &
gateway_pid=$!
sleep 1
export STS2_RUNTIME_PROFILE=runtime-v3-gameplay STS2_MCP_BINARY="$mcp"
export STS2_EXO_BRIDGE_BINARY="$provider" STS2_EXO_BRIDGE_ARGS_JSON='[]'
export STS2_EXO_REVISION
STS2_EXO_REVISION=$(sha256sum "$provider"); STS2_EXO_REVISION=${STS2_EXO_REVISION%% *}
export STS2_PROVIDER_KIND=ollama STS2_COMBAT_DEMO=true STS2_EXO_FORWARD_VISIBLE_SEED=true
export STS2_OBJECTIVE='Win this combat while preserving HP.' STS2_MAX_STEPS=100
export STS2_REPLAY_TRAJECTORY="$replay"
jq -n --arg seed "$seed" --arg bridge "$STS2_EXO_REVISION" --arg mode "$mode" \
    --arg display "$display" --arg width "$width" --arg height "$height" --arg replay "$replay" \
    '{seed:$seed,bridge_sha256:$bridge,provider:"ollama",model:"gemma4:31b-cloud",
    display:$display,width:$width,height:$height,window_mode:$mode,replay:$replay}' >"$run/manifest.json"
sha256sum "$gateway" "$mcp" "$harness" "$provider" >"$run/binaries.sha256"
"$harness" >"$run/trajectory.jsonl" 2>"$run/harness.stderr" &
harness_pid=$!
if wait "$harness_pid"; then
    harness_pid=''
    printf 'Combat completed; keeping the game visible for %s seconds.\n' "$hold"
    sleep "$hold"
else
    harness_pid=''
    printf 'Combat failed; inspect the external harness error file.\n' >&2
    exit 2
fi
