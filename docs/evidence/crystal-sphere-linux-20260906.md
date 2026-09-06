# Confirmed native Linux Crystal Sphere settlement, 2026-09-06

The authorized visible Linux fixture replayed 384 recorded actions from a fresh seeded
practice start and reached Crystal Sphere. The replay exited zero with an explicit
`episode_replay_prefix_verified` checkpoint and zero provider calls. Its source contains
one rejected attempt, skipped under the replay admission checks. This is a composed
prefix, not a complete campaign replay.

Replay trajectory SHA-256:
`d469e0e02ed5228d0326a8738736a8df1991ed044c3c14e58fbb9b6b64a77bda`.

OpenAI Astra (`gpt-6-astra`) then continued from that settled checkpoint. Seven model
decisions produced six settled operations and one stale rejected choice:

- The native parent event opened Crystal Sphere.
- Three native cell reveals settled. Their initial unknown receipts were reconciled
  to settled receipts using the same operation identities.
- A subsequent stale reveal was rejected without an effect after the loot overlay appeared.
- Reward Proceed settled as `reward_proceed_returned_to_event`, exposing the sphere's
  native Proceed action.
- Sphere Proceed settled as `crystal_sphere_completed`. The independently observed map
  exposed two legal travel options.

The immutable trajectory excerpt ends at that map observation. Its SHA-256 is
`23287b5c0a861cb4ba46ac5d76a16525edee796db2f71dbb9cb6416866b19cb3`.
The installed Linux addon SHA-256 is
`206609129597c47f15b1fe9088e6ea361093e3500e5c87256466779911f0e51e`.

Earlier attempts exposed two distinct settlement errors: loot returned to the sphere
rather than the map, and the sphere node remained behind an already travelable map.
Those unresolved attempts were preserved and were not retried. Each corrected attempt
started in a fresh host and replayed the verified prefix before another model continuation.

This confirms the Linux native event and exit path, including the combat transitions
traversed by the prefix. Windows host compilation and CI remain separate evidence;
this record does not establish Windows Crystal Sphere settlement, full campaign completion,
complete seeded replay, or victory handling. The model-controlled campaign continued
after the excerpt was captured.
