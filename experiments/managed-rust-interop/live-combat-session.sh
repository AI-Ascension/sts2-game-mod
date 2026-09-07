#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
repo_root=$(cd -- "$script_dir/../.." && pwd -P)
host_dir='' user_dir='' artifacts='' addon_dir='' map_renderer='' map_renderer_sha256=''
gateway='' mcp='' harness='' provider=''
actual_map_renderer_sha256=''
powershell=powershell.exe
display='' width='' height='' mode='' seed=AIASCENSIONREPLAY1 hold=300 replay=''
campaign_map=false seed_requested=false
user_dir_preexisting=false
supported_sts2_sha256=a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52
supported_godot_sha256=0e4897ecdfb31456a97c7d8028dfb8d7dbdc632e2f73fc9b438d7b266a139289
usage() {
    cat <<'HELP'
Usage: live-combat-session.sh --host-dir PATH --user-dir WINDOWS_PATH --artifacts-dir PATH
  --gateway-binary PATH --mcp-binary PATH --harness-binary PATH --provider-binary PATH
  [--addon-dir PATH]
  [--map-renderer-binary PATH --map-renderer-sha256 HEX]
  [--display N] [--width N] [--height N]
  [--window-mode windowed|fullscreen|borderless|maximized] [--seed VALUE]
  [--campaign-map]
  [--hold-seconds 300] [--replay-trajectory PATH]
  [--powershell-binary PATH]
Requires an already prepared disposable host, accepted addon, Windows PowerShell,
WSL, curl, jq and openssl. Display indexes are zero-based; -1 selects the primary display.
Omitted video options use saved mod-menu choices, then the first-launch defaults.
No installation is performed.
HELP
}
while (($#)); do
    case "$1" in
        --help) usage; exit 0 ;;
        --host-dir) host_dir=${2:?}; shift ;;
        --user-dir) user_dir=${2:?}; shift ;;
        --artifacts-dir) artifacts=${2:?}; shift ;;
        --addon-dir) addon_dir=${2:?}; shift ;;
        --map-renderer-binary) map_renderer=${2:?}; shift ;;
        --map-renderer-sha256) map_renderer_sha256=${2:?}; shift ;;
        --gateway-binary) gateway=${2:?}; shift ;;
        --mcp-binary) mcp=${2:?}; shift ;;
        --harness-binary) harness=${2:?}; shift ;;
        --provider-binary) provider=${2:?}; shift ;;
        --display) display=${2:?}; shift ;;
        --width) width=${2:?}; shift ;;
        --height) height=${2:?}; shift ;;
        --window-mode) mode=${2:?}; shift ;;
        --seed) seed=${2:?}; seed_requested=true; shift ;;
        --campaign-map) campaign_map=true ;;
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
provider_metadata=$("$provider" --describe)
provider_kind=$(jq -er '.kind' <<<"$provider_metadata")
provider_name=$(jq -er '.provider' <<<"$provider_metadata")
provider_model=$(jq -er '.model' <<<"$provider_metadata")
case "$provider_kind:$provider_name:$provider_model" in
    openai-astra:openai:gpt-6-astra) command -v codex >/dev/null; command -v timeout >/dev/null ;;
    ollama:ollama:gemma4:31b-cloud) ;;
    *) printf 'Unsupported provider identity\n' >&2; exit 2 ;;
esac
[[ -f "$host_dir/override.cfg" && -n "$user_dir" && -n "$artifacts" ]] || exit 2
host_dir=$(realpath -m -- "$host_dir")
artifacts=$(realpath -m -- "$artifacts")
[[ -d "$host_dir" ]] || { printf '%s\n' 'Host directory is missing' >&2; exit 2; }
case "$host_dir/" in "$repo_root/"*) printf '%s\n' 'Host directory must be outside the repository' >&2; exit 2 ;; esac
case "$artifacts/" in "$repo_root/"*) printf '%s\n' 'Artifacts must be outside the repository' >&2; exit 2 ;; esac
if [[ -z "$addon_dir" ]]; then addon_dir="$host_dir/mods"; fi
addon_dir=$(realpath -m -- "$addon_dir")
case "$addon_dir/" in "$repo_root/"*) printf '%s\n' 'Addon directory must be outside the repository' >&2; exit 2 ;; esac
if ! command -v "$powershell" >/dev/null 2>&1 \
    && [[ -x '/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe' ]]; then
    powershell='/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe'
fi
if [[ "$campaign_map" == true ]]; then
    map_renderer=$(realpath -m -- "$map_renderer")
    case "$map_renderer/" in
        "$repo_root/"*)
            printf '%s\n' 'Map renderer must be outside the repository' >&2
            exit 2
            ;;
    esac
    [[ -f "$map_renderer" && -x "$map_renderer" && ! -L "$map_renderer" ]] || {
        printf '%s\n' 'Map renderer must be an executable regular external file' >&2
        exit 2
    }
    actual_map_renderer_sha256=$(sha256sum "$map_renderer"); actual_map_renderer_sha256=${actual_map_renderer_sha256%% *}
    [[ "$actual_map_renderer_sha256" == "$map_renderer_sha256" ]] || {
        printf 'Map renderer SHA-256 mismatch: %s\n' "$actual_map_renderer_sha256" >&2
        exit 2
    }
    [[ "$provider_kind" == openai-astra ]] || {
        printf '%s\n' 'Campaign/map mode requires the OpenAI Astra provider' >&2
        exit 2
    }
    [[ "$seed_requested" == false && -z "$replay" ]] || {
        printf '%s\n' 'Campaign/map mode uses host-generated standard seed and cannot replay' >&2
        exit 2
    }
    [[ -n "$map_renderer" && "$map_renderer_sha256" =~ ^[0-9a-f]{64}$ ]] || {
        printf '%s\n' 'Campaign/map mode requires a pinned map renderer binary and SHA-256' >&2
        exit 2
    }
elif [[ -n "$map_renderer" || -n "$map_renderer_sha256" ]]; then
    printf '%s\n' 'Map renderer options require --campaign-map' >&2
    exit 2
fi
[[ -z "$display" || "$display" =~ ^(-1|[0-9]+)$ ]] || exit 2
[[ -z "$width" || "$width" =~ ^[0-9]+$ ]] || exit 2
[[ -z "$height" || "$height" =~ ^[0-9]+$ ]] || exit 2
[[ "$hold" =~ ^[0-9]+$ && "$hold" -le 600 ]] || exit 2
[[ "$seed" =~ ^[A-Za-z0-9_-]{1,128}$ ]] || exit 2
case "$mode" in ''|windowed|fullscreen|borderless|maximized) ;; *) exit 2 ;; esac
video_args=()
[[ -z "$display" ]] || video_args+=(-Display "$display")
[[ -z "$width" ]] || video_args+=(-Width "$width")
[[ -z "$height" ]] || video_args+=(-Height "$height")
[[ -z "$mode" ]] || video_args+=(-WindowMode "$mode")
[[ -z "$replay" || -f "$replay" ]] || exit 2
for command in wslpath curl jq openssl "$powershell"; do command -v "$command" >/dev/null; done
user_dir_wsl=$(wslpath -u "$user_dir") || {
    printf '%s\n' 'User directory must be an absolute Windows path' >&2
    exit 2
}
user_dir_wsl=$(realpath -m -- "$user_dir_wsl")
[[ "$user_dir_wsl" != / && "$user_dir_wsl" != "$host_dir" ]] || {
    printf '%s\n' 'User directory must be a separate disposable path' >&2
    exit 2
}
case "$user_dir_wsl/" in
    "$repo_root/"*|"$host_dir/"*|"$artifacts/"*|"$addon_dir/"*)
        printf '%s\n' 'User directory must be outside repository, artifacts, and addon paths' >&2
        exit 2
        ;;
esac
[[ ! -L "$user_dir_wsl" ]] || {
    printf '%s\n' 'User directory must not be a symbolic link' >&2
    exit 2
}
if [[ -e "$user_dir_wsl" ]]; then user_dir_preexisting=true; fi
umask 077
mkdir -p -- "$artifacts"
run=$(mktemp -d "$artifacts/run-XXXXXXXX")
runtime_token=$(openssl rand -hex 32)
gateway_token=$(openssl rand -hex 32)
host_pid='' gateway_pid='' harness_pid=''
host_data_dir="$host_dir/data_sts2_windows_x86_64"
game_exe="$host_dir/SlayTheSpire2.exe"
[[ -f "$host_data_dir/sts2.dll" && -f "$host_data_dir/GodotSharp.dll" && -f "$game_exe" ]] || {
    printf '%s\n' 'Disposable host must contain SlayTheSpire2.exe and its data assemblies' >&2
    exit 2
}
actual_sts2_sha256=$(sha256sum "$host_data_dir/sts2.dll"); actual_sts2_sha256=${actual_sts2_sha256%% *}
actual_godot_sha256=$(sha256sum "$host_data_dir/GodotSharp.dll"); actual_godot_sha256=${actual_godot_sha256%% *}
[[ "$actual_sts2_sha256" == "$supported_sts2_sha256" ]] || {
    printf 'Unsupported sts2.dll SHA-256: %s\n' "$actual_sts2_sha256" >&2
    exit 2
}
[[ "$actual_godot_sha256" == "$supported_godot_sha256" ]] || {
    printf 'Unsupported GodotSharp.dll SHA-256: %s\n' "$actual_godot_sha256" >&2
    exit 2
}
addon_files=(AIAscensionSTS2GameMod.dll AIAscensionSTS2GameModNative.dll AIAscensionSTS2GameMod.json)
[[ -d "$addon_dir" ]] || { printf '%s\n' 'Accepted addon directory is missing' >&2; exit 2; }
for addon_file in "${addon_files[@]}"; do
    [[ -f "$addon_dir/$addon_file" && ! -L "$addon_dir/$addon_file" ]] || {
        printf 'Accepted addon artifact is missing or linked: %s\n' "$addon_file" >&2
        exit 2
    }
done
addon_managed_sha256=$(sha256sum "$addon_dir/AIAscensionSTS2GameMod.dll"); addon_managed_sha256=${addon_managed_sha256%% *}
addon_native_sha256=$(sha256sum "$addon_dir/AIAscensionSTS2GameModNative.dll"); addon_native_sha256=${addon_native_sha256%% *}
addon_manifest_sha256=$(sha256sum "$addon_dir/AIAscensionSTS2GameMod.json"); addon_manifest_sha256=${addon_manifest_sha256%% *}
sha256sum "$host_data_dir/sts2.dll" "$host_data_dir/GodotSharp.dll" "$game_exe" \
    "$host_dir/override.cfg" "${addon_files[@]/#/$addon_dir/}" >"$run/baseline.sha256"
sha256sum "${addon_files[@]/#/$addon_dir/}" >"$run/addon.sha256"
campaign_args=(-Seed "$seed")
if [[ "$campaign_map" == true ]]; then
    campaign_args=(-Campaign -CampaignMode standard -CampaignMapBound)
fi
verify_campaign_map_trace() {
    local trace=$1 total starts maps other settled
    [[ -s "$trace" ]] || { printf '%s\n' 'Bounded trace is empty' >&2; return 1; }
    total=$(jq -s '[.[] | select(.event == "model_decision")] | length' "$trace")
    starts=$(jq -s '[.[] | select(.event == "model_decision")
        | select((.action_id // "") | startswith("start_run:"))] | length' "$trace")
    maps=$(jq -s '[.[] | select(.event == "model_decision")
        | select((.action_id // "") | startswith("select_map_node:"))] | length' "$trace")
    other=$(jq -s '[.[] | select(.event == "model_decision")
        | select((.action_id // "") | (startswith("start_run:") or startswith("select_map_node:")) | not)]
        | length' "$trace")
    settled=$(jq -s '[.[] | select(.event == "operation_wait_completed")
        | select((.operation_id // "") | startswith("episode-action-"))] | length' "$trace")
    [[ "$total" == "$((starts + maps))" && "$starts" == 1 && "$maps" == 1 \
        && "$other" == 0 && "$settled" == "$((starts + maps))" ]] || {
        printf 'Bounded trace rejected: total=%s start_run=%s select_map_node=%s other=%s settled=%s\n' \
            "$total" "$starts" "$maps" "$other" "$settled" >&2
        return 1
    }
    printf 'Bounded campaign/map trace accepted: start_run=%s select_map_node=%s settled=%s\n' \
        "$starts" "$maps" "$settled"
}
verify_baseline() {
    if sha256sum --check --quiet "$run/baseline.sha256"; then
        printf '%s\n' 'preserved' >"$run/baseline.status"
        printf '%s\n' 'Disposable host/addon baseline preserved.'
        return 0
    else
        printf '%s\n' 'changed' >"$run/baseline.status"
        printf '%s\n' 'Disposable host/addon baseline changed; inspect baseline.sha256.' >&2
        return 1
    fi
}
cleanup() {
    touch "$run/stop"
    for pid in "$harness_pid" "$gateway_pid"; do
        if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null || true
            wait "$pid" 2>/dev/null || true
        fi
    done
    if [[ -n "$host_pid" ]]; then wait "$host_pid" || printf 'Host guardian failed\n' >&2; fi
    if [[ -n "$run" && -f "$run/baseline.sha256" ]] && ! verify_baseline; then
        trap - EXIT
        exit 2
    fi
}
trap cleanup EXIT
trap 'exit 130' INT TERM
"$powershell" -NoProfile -File "$(wslpath -w "$script_dir/live-combat-demo.ps1")" \
    -HostDirectory "$(wslpath -w "$host_dir")" -UserDirectory "$user_dir" \
    -LogPath "$(wslpath -w "$run/game.log")" -StopFile "$(wslpath -w "$run/stop")" \
    "${campaign_args[@]}" "${video_args[@]}" \
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
export STS2_PROVIDER_KIND="$provider_kind" STS2_EXO_FORWARD_VISIBLE_SEED=true
if [[ "$provider_kind" == openai-astra ]]; then
    export STS2_EXO_INHERITED_ENV_JSON='["HOME","PATH"]'
else
    export STS2_EXO_INHERITED_ENV_JSON='[]'
fi
if [[ "$campaign_map" == true ]]; then
    unset STS2_COMBAT_DEMO
    mkdir -p -- "$run/map-artifacts"
    export STS2_MAP_ARTIFACT_ROOT="$run/map-artifacts"
    export STS2_LIVE_EPISODE=true STS2_MAP_MODE=graph-image STS2_MAP_RENDERER_BINARY="$map_renderer"
    export STS2_MAP_RENDERER_SHA256="$map_renderer_sha256" STS2_MAX_STEPS=2
    export STS2_RECOVERY_MAX_ATTEMPTS=1 STS2_EXO_TIMEOUT_MILLIS=90000
    export STS2_OBJECTIVE='Start one standard campaign run, inspect the complete current map image, then select one currently legal map node.'
    export STS2_HARD_CONSTRAINTS_JSON='["Use exactly one host-legal start_run action.","After setup, use exactly one current host-legal select_map_node action.","Stop after the first settled map selection and never enter combat actions, rewards, shops, or events."]'
else
    unset STS2_MAP_ARTIFACT_ROOT
    export STS2_COMBAT_DEMO=true STS2_OBJECTIVE='Win this combat while preserving HP.' STS2_MAX_STEPS=100
fi
export STS2_REPLAY_TRAJECTORY="$replay"
jq -n --arg seed "$(if [[ "$campaign_map" == true ]]; then printf ''; else printf '%s' "$seed"; fi)" \
    --arg bridge "$STS2_EXO_REVISION" --arg mode "$mode" \
    --arg provider "$provider_name" --arg model "$provider_model" \
    --arg map_renderer "$map_renderer" --arg map_renderer_sha256 "$map_renderer_sha256" \
    --arg host_build '0.107.1' --arg host_release '59260271' \
    --arg sts2_sha256 "$actual_sts2_sha256" --arg godot_sha256 "$actual_godot_sha256" \
    --arg addon_managed_sha256 "$addon_managed_sha256" --arg addon_native_sha256 "$addon_native_sha256" \
    --arg addon_manifest_sha256 "$addon_manifest_sha256" \
    --arg addon_dir "$addon_dir" --arg baseline "$run/baseline.sha256" \
    --argjson user_dir_preexisting "$user_dir_preexisting" \
    --arg display "$display" --arg width "$width" --arg height "$height" --arg replay "$replay" \
    --argjson campaign_map "$campaign_map" \
    '{seed:$seed,bridge_sha256:$bridge,provider:$provider,model:$model,
    host_build:$host_build,host_release:$host_release,
    host_assemblies:{sts2_dll_sha256:$sts2_sha256,godotsharp_sha256:$godot_sha256},
    accepted_addon_hashes:{managed_dll_sha256:$addon_managed_sha256,native_dll_sha256:$addon_native_sha256,manifest_sha256:$addon_manifest_sha256},
    accepted_addon_dir:$addon_dir,baseline_sha256:$baseline,
    user_dir_preexisting:$user_dir_preexisting,
    display:($display | if . == "" then null else tonumber end),
    width:($width | if . == "" then null else tonumber end),
    height:($height | if . == "" then null else tonumber end),
    window_mode:($mode | if . == "" then null else . end),
    campaign_map_bound:$campaign_map,
    campaign_action_contract:(if $campaign_map then {start_run:1,select_map_node:1} else null end),
    provider_call_policy:(if $campaign_map then {max_model_decisions:2,max_steps:2,recovery_attempts:1,timeout_millis:90000} else null end),
    map_rendering:(if $campaign_map then {mode:"graph-image",binary:$map_renderer,sha256:$map_renderer_sha256,image_required:true,artifact_root_relative:"map-artifacts"} else null end),
    video_values:"requested overrides; null uses saved preference or default; actual values are in game.log",
    replay:$replay}' >"$run/manifest.json"
sha256sum "$gateway" "$mcp" "$harness" "$provider" >"$run/binaries.sha256"
"$harness" >"$run/trajectory.jsonl" 2>"$run/harness.stderr" &
harness_pid=$!
if wait "$harness_pid"; then
    harness_pid=''
    if [[ "$campaign_map" == true ]]; then
        verify_campaign_map_trace "$run/trajectory.jsonl"
        printf 'Bounded campaign/map episode completed; keeping the game visible for %s seconds.\n' "$hold"
    else
        printf 'Combat completed; keeping the game visible for %s seconds.\n' "$hold"
    fi
    sleep "$hold"
else
    harness_pid=''
    if [[ "$campaign_map" == true ]] && verify_campaign_map_trace "$run/trajectory.jsonl"; then
        printf 'Bounded campaign/map guard reached; keeping the game visible for %s seconds.\n' "$hold"
        sleep "$hold"
    else
        printf 'Combat failed; inspect the external harness error file.\n' >&2
        exit 2
    fi
fi
