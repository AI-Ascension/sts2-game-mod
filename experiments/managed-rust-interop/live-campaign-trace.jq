# SPDX-License-Identifier: MIT
# Trace acceptance is evidence of the bounded harness sequence, not independent host semantics.
def current_kind:
  . as $decision
  | [.observation.legal_actions[]? | select(.action_id == $decision.action_id) | .action.kind]
  | if length == 1 then .[0] else null end;
def witnessed($events; $action):
  [$events[] | select(.event == "action_receipt" and .action_id == $action)] as $receipts
  | ([$receipts[].operation_id] | unique) as $operations
  | ($operations | length) == 1
    and ($operations[0] | type == "string" and length > 0)
    and (
      any($receipts[]; .status == "Settled" and (.effect | type == "string" and length > 0)
        and (.observation | type == "object"))
      or any($events[]; .event == "operation_wait_completed" and .operation_id == $operations[0]
        and (.effect | type == "string" and length > 0) and (.observation | type == "object"))
    );
. as $events
| [.[] | select(.event == "model_decision")] as $decisions
| ($decisions | length) == 2
  and ([$decisions[] | current_kind] == ["start_run", "select_map_node"])
  and all($decisions[]; witnessed($events; .action_id))
  and any($events[]; .event == "map_telemetry" and .phase == "render"
    and .mode == "graph-image" and .image_verified == true
    and (.image_bytes | type == "number" and . > 0))
  and any($events[]; .event == "map_artifact_action" and .action_id == $decisions[1].action_id
    and (.bundle_digest | type == "string" and test("^[0-9a-f]{64}$")))
  and any($events[]; .event == "campaign_map_post_observation"
    and (.observation | type == "object") and .provider_calls == 0 and .game_actions == 0)
  and any($events[]; .event == "campaign_map_post_capture"
    and (.map_status == "available" or .map_status == "unavailable_not_map_decision")
    and .provider_calls == 0 and .game_actions == 0 and .delivered_to_provider == false)
  and any($events[]; .event == "campaign_map_bound_finished" and .verified_settlements == 2)
  and all($events[]; .event != "campaign_map_post_capture_failed")
