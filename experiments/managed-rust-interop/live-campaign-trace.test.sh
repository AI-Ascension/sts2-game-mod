#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
fixture=$(cat <<'JSON'
[
  {"event":"model_decision","action_id":"opaque-start","observation":{"legal_actions":[{"action_id":"opaque-start","action":{"kind":"start_run"}}]}},
  {"event":"action_receipt","action_id":"opaque-start","operation_id":"op-1","status":"Accepted"},
  {"event":"operation_wait_completed","operation_id":"op-1","effect":"campaign.started","observation":{}},
  {"event":"map_telemetry","phase":"render","mode":"graph-image","image_verified":true,"image_bytes":120},
  {"event":"model_decision","action_id":"opaque-map","observation":{"legal_actions":[{"action_id":"opaque-map","action":{"kind":"select_map_node"}}]}},
  {"event":"action_receipt","action_id":"opaque-map","operation_id":"op-2","status":"Accepted"},
  {"event":"map_artifact_action","action_id":"opaque-map","bundle_digest":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"},
  {"event":"operation_wait_completed","operation_id":"op-2","effect":"map.selected","observation":{}},
  {"event":"campaign_map_post_observation","observation":{},"provider_calls":0,"game_actions":0},
  {"event":"campaign_map_post_capture","map_status":"unavailable_not_map_decision","provider_calls":0,"game_actions":0,"delivered_to_provider":false},
  {"event":"campaign_map_bound_finished","verified_settlements":2}
]
JSON
)
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
check true 'map(if .event == "action_receipt" then .status="Settled" | .effect="settled" | .observation={} else . end) | map(select(.event != "operation_wait_completed"))'
check false 'map(select(.event != "operation_wait_completed"))'
check false 'map(select(.event != "campaign_map_post_capture"))'
check false 'map(select(.event != "campaign_map_bound_finished"))'
check false 'map(select(.event != "map_artifact_action"))'
check false 'map(if .phase == "render" then .image_verified=false else . end)'
check false 'map(if .action_id == "opaque-map" and .event == "model_decision" then .observation.legal_actions[0].action.kind="proceed" else . end)'
check false 'map(if .event == "operation_wait_completed" then .operation_id="wrong-operation" else . end)'
check false '. + [.[0]]'
check false '. + [{"event":"campaign_map_post_capture_failed"}]'
printf '%s\n' 'live-campaign-trace: 11 checks passed'
