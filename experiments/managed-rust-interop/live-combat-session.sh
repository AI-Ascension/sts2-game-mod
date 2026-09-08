#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
repo_root=$(cd -- "$script_dir/../.." && pwd -P)
host_dir='' user_dir='' artifacts='' addon_dir='' map_renderer='' map_renderer_sha256=''
gateway='' mcp='' harness='' provider=''
actual_map_renderer_sha256=''
powershell=powershell.exe
display='' width='' height='' window_mode='' run_kind=campaign campaign_mode=standard
seed='' seed_set=false max_runtime_seconds='' hold=300 replay=''
campaign_map=false
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
  [--window-mode windowed|fullscreen|borderless|maximized]
  [--run-kind campaign|demo] [--campaign-mode standard|practice] [--seed VALUE]
  [--max-runtime-seconds N]
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
        --window-mode) window_mode=${2:?}; shift ;;
        --run-kind) run_kind=${2:?}; shift ;;
        --campaign-mode) campaign_mode=${2:?}; shift ;;
        --seed) seed=${2:?}; seed_set=true; shift ;;
        --max-runtime-seconds) max_runtime_seconds=${2:?}; shift ;;
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
case "$run_kind" in
    campaign) max_runtime_default=3600; max_runtime_limit=3600 ;;
    demo) max_runtime_default=900; max_runtime_limit=900 ;;
    *) printf 'unsupported run kind\n' >&2; exit 2 ;;
esac
[[ -n "$max_runtime_seconds" ]] || max_runtime_seconds=$max_runtime_default
[[ "$max_runtime_seconds" =~ ^[0-9]+$ ]] || { printf 'max runtime must be an integer\n' >&2; exit 2; }
(( ${#max_runtime_seconds} <= 4 )) || { printf 'max runtime is too large\n' >&2; exit 2; }
max_runtime_seconds_value=$((10#$max_runtime_seconds))
(( max_runtime_seconds_value >= 60 && max_runtime_seconds_value <= max_runtime_limit )) || {
    printf 'max runtime must be between 60 and %s seconds for %s\n' "$max_runtime_limit" "$run_kind" >&2
    exit 2
}
max_runtime_seconds=$max_runtime_seconds_value
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
    [[ "$run_kind" == campaign && "$campaign_mode" == standard ]] || {
        printf '%s\n' 'Campaign/map mode requires a standard campaign' >&2
        exit 2
    }
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
    [[ "$seed_set" == false && -z "$replay" ]] || {
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
[[ -z "$seed" || "$seed" =~ ^[A-Za-z0-9_-]{1,128}$ ]] || exit 2
case "$window_mode" in ''|windowed|fullscreen|borderless|maximized) ;; *) exit 2 ;; esac
case "$run_kind" in
    campaign)
        case "$campaign_mode" in
            standard) [[ "$seed_set" == false ]] || { printf 'standard mode does not accept --seed\n' >&2; exit 2; } ;;
            practice) [[ -n "$seed" ]] || { printf 'practice mode requires --seed\n' >&2; exit 2; } ;;
            *) printf 'unsupported campaign mode\n' >&2; exit 2 ;;
        esac
        ;;
    demo)
        [[ "$campaign_mode" == standard ]] || { printf 'demo does not accept a campaign mode\n' >&2; exit 2; }
        [[ -n "$seed" ]] || seed=AIASCENSIONREPLAY1
        ;;
esac
video_args=()
[[ -z "$display" ]] || video_args+=(-Display "$display")
[[ -z "$width" ]] || video_args+=(-Width "$width")
[[ -z "$height" ]] || video_args+=(-Height "$height")
[[ -z "$window_mode" ]] || video_args+=(-WindowMode "$window_mode")
[[ -z "$replay" || -f "$replay" ]] || exit 2
[[ -z "$replay" || "$campaign_mode" == practice ]] || { printf 'fresh-process replay requires practice mode and its explicit seed\n' >&2; exit 2; }
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
verify_campaign_map_trace() {
    local trace=$1
    [[ -s "$trace" ]] || { printf '%s\n' 'Bounded trace is empty' >&2; return 1; }
    jq -e -s -f "$script_dir/live-campaign-trace.jq" "$trace" >/dev/null || {
        printf '%s\n' 'Bounded trace lacks the required sequence, witnesses, image, artifact, or fresh capture.' >&2
        return 1
    }
    printf '%s\n' 'Bounded campaign/map trace accepted: two verified actions and a fresh read-only capture.'
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
guardian_args=(
    -HostDirectory "$(wslpath -w "$host_dir")" -UserDirectory "$user_dir"
    -LogPath "$(wslpath -w "$run/game.log")" -StopFile "$(wslpath -w "$run/stop")"
    -RunKind "$run_kind" -MaxRuntimeSeconds "$max_runtime_seconds"
)
[[ "$run_kind" == demo ]] || guardian_args+=( -CampaignMode "$campaign_mode" )
[[ -z "$seed" ]] || guardian_args+=( -Seed "$seed" )
[[ "$campaign_map" != true ]] || guardian_args+=( -CampaignMapBound )
"$powershell" -NoProfile -File "$(wslpath -w "$script_dir/live-combat-demo.ps1")" \
    "${guardian_args[@]}" "${video_args[@]}" \
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
if [[ "$run_kind" == demo ]]; then
    export STS2_PROVIDER_KIND="$provider_kind" STS2_COMBAT_DEMO=true STS2_LIVE_EPISODE=true
    export STS2_OBJECTIVE='Win this combat while preserving HP.' STS2_MAX_STEPS=100
else
    export STS2_PROVIDER_KIND="$provider_kind" STS2_COMBAT_DEMO=false STS2_LIVE_EPISODE=true
    export STS2_OBJECTIVE='Complete the campaign while preserving HP.' STS2_MAX_STEPS=1024
fi
export STS2_EXO_FORWARD_VISIBLE_SEED=true
if [[ "$provider_kind" == openai-astra ]]; then
    export STS2_EXO_INHERITED_ENV_JSON='["HOME","PATH"]'
else
    export STS2_EXO_INHERITED_ENV_JSON='[]'
fi
if [[ "$campaign_map" == true ]]; then
    unset STS2_COMBAT_DEMO
    mkdir -p -- "$run/map-artifacts"
    export STS2_MAP_ARTIFACT_ROOT="$run/map-artifacts"
    export STS2_CAMPAIGN_MAP_BOUND=true
    export STS2_LIVE_EPISODE=true STS2_MAP_MODE=graph-image STS2_MAP_RENDERER_BINARY="$map_renderer"
    export STS2_MAP_RENDERER_SHA256="$map_renderer_sha256" STS2_MAX_STEPS=2
    export STS2_RECOVERY_MAX_ATTEMPTS=1 STS2_EXO_TIMEOUT_MILLIS=90000
    export STS2_OBJECTIVE='Start one standard campaign run, inspect the complete current map image, then select one currently legal map node.'
    export STS2_HARD_CONSTRAINTS_JSON='["Use exactly one host-legal start_run action.","After setup, use exactly one current host-legal select_map_node action.","Stop after the first settled map selection and never enter combat actions, rewards, shops, or events."]'
else
    unset STS2_MAP_ARTIFACT_ROOT
    unset STS2_CAMPAIGN_MAP_BOUND
fi
export STS2_REPLAY_TRAJECTORY="$replay"
manifest_campaign_mode=$campaign_mode
[[ "$run_kind" == campaign ]] || manifest_campaign_mode=''
jq -n --arg seed "$seed" --arg run_kind "$run_kind" --arg max_runtime "$max_runtime_seconds" \
    --arg bridge "$STS2_EXO_REVISION" --arg campaign_mode "$manifest_campaign_mode" \
    --arg window_mode "$window_mode" \
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
    '{run_kind:$run_kind,max_runtime_seconds:($max_runtime | tonumber),
    seed:($seed | if . == "" then null else . end),
    campaign_mode:($campaign_mode | if . == "" then null else . end),
    bridge_sha256:$bridge,provider:$provider,model:$model,
    display:($display | if . == "" then null else tonumber end),
    width:($width | if . == "" then null else tonumber end),
    height:($height | if . == "" then null else tonumber end),
    window_mode:($window_mode | if . == "" then null else . end),
    host_build:$host_build,host_release:$host_release,
    host_assemblies:{sts2_dll_sha256:$sts2_sha256,godotsharp_sha256:$godot_sha256},
    accepted_addon_hashes:{managed_dll_sha256:$addon_managed_sha256,native_dll_sha256:$addon_native_sha256,manifest_sha256:$addon_manifest_sha256},
    accepted_addon_dir:$addon_dir,baseline_sha256:$baseline,
    user_dir_preexisting:$user_dir_preexisting,
    campaign_map_bound:$campaign_map,
    campaign_action_contract:(if $campaign_map then {start_run:1,select_map_node:1} else null end),
    provider_call_policy:(if $campaign_map then {max_model_decisions:2,max_steps:2,recovery_attempts:1,timeout_millis:90000,sequence_guard:true,post_settlement_capture:"read-only"} else null end),
    decision_and_receipt_artifact:"trajectory.jsonl",
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
    printf 'Harness failed; inspect the external harness error file.\n' >&2
    exit 2
fi
