#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
fixture=$(cat <<'JSON'
[
  {"event":"model_decision","model_execution_id":1,"action_id":"opaque-start","observation":{"state_id":"state-1","generation":1,"legal_actions":[{"action_id":"opaque-start","action":{"kind":"start_run"}}]}},
  {"event":"action_receipt","action_id":"opaque-start","operation_id":"op-1","status":"Accepted"},
  {"event":"operation_wait_completed","operation_id":"op-1","effect":"campaign.started","observation":{"state_id":"state-2","generation":2}},
  {"event":"map_telemetry","phase":"render","mode":"graph-image","image_verified":true,"image_bytes":120,"model_execution_id":2,"source_state_id":"state-2","generation":2},
  {"event":"map_telemetry","phase":"publication","model_execution_id":2,"source_state_id":"state-2","generation":2},
  {"event":"model_decision","model_execution_id":2,"action_id":"opaque-map","observation":{"state_id":"state-2","generation":2,"legal_actions":[{"action_id":"opaque-map","action":{"kind":"select_map_node"}}]}},
  {"event":"action_receipt","action_id":"opaque-map","operation_id":"op-2","status":"Accepted"},
  {"event":"map_artifact_action","action_id":"opaque-map","operation_id":"op-2","model_execution_id":2,"generation":2},
  {"event":"operation_wait_completed","operation_id":"op-2","effect":"map.selected","observation":{"state_id":"state-3","generation":3}},
  {"event":"campaign_map_post_observation","observation":{"state_id":"state-3","generation":3},"provider_calls":0,"game_actions":0,"delivered_to_provider":false},
  {"event":"campaign_map_post_capture","state_id":"state-3","generation":3,"after_model_execution_id":2,"map_status":"unavailable_not_map_decision","provider_calls":0,"game_actions":0,"delivered_to_provider":false},
  {"event":"campaign_map_bound_finished","verified_settlements":2}
]
JSON
)
fixture=$(jq 'map(
    if .phase == "render" or .phase == "publication" then .graph_digest=("a"*64) | .image_digest=("b"*64) else . end
    | if .phase == "publication" or .event == "map_artifact_action" then .bundle_digest=("c"*64) else . end
)' <<<"$fixture")
check() {
    local expected=$1 transform=$2
    local observed=false
    if jq -c "$transform | .[]" <<<"$fixture" \
        | jq -e -s -f "$script_dir/live-campaign-trace.jq" >/dev/null; then
        observed=true
    fi
    if [[ "$observed" != "$expected" ]]; then
        printf 'Trace test failed: expected=%s transform=%s\n' "$expected" "$transform" >&2
        exit 1
    fi
}
check true '.'
# A synchronous settled receipt is sufficient; a redundant wait event is not required.
check true 'map(if .event == "action_receipt" then .status="Settled" | .effect="settled" | .observation={generation:(if .operation_id=="op-1" then 2 else 3 end)} else . end) | map(select(.event != "operation_wait_completed"))'
check false 'map(select(.event != "operation_wait_completed"))'
check false 'map(select(.event != "campaign_map_post_capture"))'
check false 'map(select(.event != "campaign_map_bound_finished"))'
check false 'map(select(.event != "map_artifact_action"))'
check false 'map(if .phase == "render" then .image_verified=false else . end)'
check false 'map(if .action_id == "opaque-map" and .event == "model_decision" then .observation.legal_actions[0].action.kind="proceed" else . end)'
check false 'map(if .event == "operation_wait_completed" then .operation_id="wrong-operation" else . end)'
check false '. + [.[0]]'
check false '. + [{"event":"campaign_map_post_capture_failed"}]'
check false 'sort_by(if (.event | startswith("campaign_map_post")) or .event=="campaign_map_bound_finished" then 0 else 1 end)'
check false 'map(if .operation_id=="op-2" then .operation_id="op-1" else . end)'
check false 'map(if .event=="map_artifact_action" then .operation_id="orphan" else . end)'
check false 'map(if .phase=="render" then .generation=1 else . end)'
check false 'map(if .phase=="publication" then .image_digest=("d"*64) else . end)'
check false 'map(if .event=="campaign_map_post_capture" then .generation=1 else . end)'
printf '%s\n' 'live-campaign-trace: 17 checks passed'
