# SPDX-License-Identifier: MIT
# Trace acceptance is evidence of the bounded harness sequence, not independent host semantics.
def digest: type == "string" and test("^[0-9a-f]{64}$");
def current_kind:
  . as $decision
  | [.observation.legal_actions[]? | select(.action_id == $decision.action_id) | .action.kind]
  | if length == 1 then .[0] else null end;
def receipts($rows; $decision):
  [$rows[] | select(.value.event == "action_receipt"
    and .value.action_id == $decision.value.action_id and .key > $decision.key)];
def witnesses($rows; $receipts; $operation):
  [$rows[] | select(.key >= $receipts[0].key and .value.operation_id == $operation)
    | select((.value.event == "action_receipt" and .value.status == "Settled")
      or .value.event == "operation_wait_completed")
    | select((.value.effect | type == "string" and length > 0)
      and (.value.observation | type == "object")
      and (.value.observation.generation | type == "number"))];
to_entries as $rows
| [$rows[] | select(.value.event == "model_decision")] as $d
| receipts($rows; $d[0]) as $r1 | receipts($rows; $d[1]) as $r2
| ([$r1[].value.operation_id] | unique) as $o1
| ([$r2[].value.operation_id] | unique) as $o2
| witnesses($rows; $r1; $o1[0]) as $w1 | witnesses($rows; $r2; $o2[0]) as $w2
| [$rows[] | select(.value.event == "map_telemetry" and .value.phase == "render"
    and .key < $d[1].key and .value.model_execution_id == $d[1].value.model_execution_id
    and .value.source_state_id == $d[1].value.observation.state_id
    and .value.generation == $d[1].value.observation.generation)] as $images
| [$rows[] | select(.value.event == "map_artifact_action"
    and .value.action_id == $d[1].value.action_id
    and .value.operation_id == $o2[0]
    and .value.model_execution_id == $d[1].value.model_execution_id
    and .value.generation == $d[1].value.observation.generation)] as $links
| [$rows[] | select(.value.event == "map_telemetry" and .value.phase == "publication"
    and .key < $d[1].key and .value.bundle_digest == $links[0].value.bundle_digest
    and .value.model_execution_id == $d[1].value.model_execution_id
    and .value.source_state_id == $d[1].value.observation.state_id
    and .value.generation == $d[1].value.observation.generation
    and .value.graph_digest == $images[0].value.graph_digest
    and .value.image_digest == $images[0].value.image_digest)] as $publications
| [$rows[] | select(.value.event == "campaign_map_post_observation")] as $post
| [$rows[] | select(.value.event == "campaign_map_post_capture")] as $capture
| [$rows[] | select(.value.event == "campaign_map_bound_finished")] as $finish
| ($d | length) == 2 and ([$d[].value | current_kind] == ["start_run", "select_map_node"])
  and ([$d[].value.model_execution_id] == [1, 2])
  and ($o1 | length) == 1 and ($o2 | length) == 1 and $o1[0] != $o2[0]
  and all([$o1[0], $o2[0]][]; type == "string" and length > 0)
  and ($w1 | length) > 0 and ($w2 | length) > 0
  and $w1[0].key < $d[1].key
  and ($images | length) == 1 and ($links | length) == 1 and ($publications | length) == 1
  and $w1[0].key < $images[0].key and $images[0].key < $publications[0].key
  and $r2[0].key < $links[0].key
  and $images[0].value.mode == "graph-image" and $images[0].value.image_verified == true
  and ($images[0].value.image_bytes | type == "number" and . > 0)
  and ($images[0].value.graph_digest | digest) and ($images[0].value.image_digest | digest)
  and ($links[0].value.bundle_digest | digest)
  and ($post | length) == 1 and ($capture | length) == 1 and ($finish | length) == 1
  and $w2[0].key < $post[0].key and $links[0].key < $post[0].key
  and $post[0].key < $capture[0].key and $capture[0].key < $finish[0].key
  and ($post[0].value.observation.state_id | type == "string" and length > 0)
  and ($post[0].value.observation.generation | type == "number")
  and $post[0].value.observation.generation >= $w2[0].value.observation.generation
  and $capture[0].value.state_id == $post[0].value.observation.state_id
  and $capture[0].value.generation == $post[0].value.observation.generation
  and $capture[0].value.after_model_execution_id == 2
  and ($capture[0].value.map_status == "available"
    or $capture[0].value.map_status == "unavailable_not_map_decision")
  and (if $capture[0].value.map_status == "available" then
    ($capture[0].value.bundle_digest | digest) else $capture[0].value.bundle_digest == null end)
  and all([$post[0].value, $capture[0].value][];
    .provider_calls == 0 and .game_actions == 0 and .delivered_to_provider == false)
  and $finish[0].value.verified_settlements == 2
  and $finish[0].key == ($rows | length) - 1
  and all($rows[] | select(.value.event == "action_receipt");
    (.value.action_id == $d[0].value.action_id and .value.operation_id == $o1[0])
    or (.value.action_id == $d[1].value.action_id and .value.operation_id == $o2[0]))
  and all($rows[] | select(.value.event == "operation_wait_completed");
    .value.operation_id == $o1[0] or .value.operation_id == $o2[0])
  and all($rows[]; .value.event != "campaign_map_post_capture_failed")
