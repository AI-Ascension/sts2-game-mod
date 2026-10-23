# ADR 0084: Owned live fixed-card-cost facts

- Status: Accepted bounded source implementation; exact-host extraction and parity unverified
- Date: 2026-10-23
- Tracking: game-mod #280; partial component of game-mod #207 and game-core #13

## Decision

The existing owner-thread live-card capture supplies only Core's `resource.fixed_card_cost`
inputs, in catalog order: raw player `energy`, then raw fixed `card_cost`. They are copied before
the native boundary, not taken from the clamped gameplay projection or caller-supplied values.
X-cost and unreadable cost-kind information do not become a fixed cost. Unavailable values retain
availability with no invented zero. Evidence is exclusively `source_derived`; all other admitted
Core records remain explicitly unsupported. This code evaluates no game rule.

Authenticated fixed routes are `GET /api/v1/game-information/facts/capabilities` (empty body,
callback 26) and `POST /api/v1/game-information/facts/query` (callback 27). Exact methods, paths,
credentials, transport context, closed canonical requests and bounded payloads are enforced. The
existing host-thread dispatcher owns the managed work. Capabilities describe this bounded source
surface, not native parity or downstream adoption.

The managed binding is retained from the existing lookup owner and checked against instance,
caller, session, lease, lease epoch, run and authority epoch. A live request must select an already
retained card snapshot; it cannot manufacture a capture by echoing arbitrary refs. The owner checks
native run/player/card lifetime, profile, raw values, generation, content and locale without
advancing the selected card epoch. Failed replacement or invalidation revokes prior retention
before host-thread work. Deferred callbacks cannot resurrect an earlier retained result.

Content is regenerated with the actual canonical manifest producer. Retention binds the complete
manifest witness: catalog generation, adapter compatibility, content-set revision, localized-text
revision and inventory revision, plus locale. Inventory revision alone is insufficient because
same-locale text and other producer witnesses can change without changing it. The query checks
again after content extraction. The native mapper compares the same owned witness, scope,
occurrence and snapshot fences against the accepted Protocol codec before emitting a response.

`sts2_game_mod_live_facts_respond` takes an owned JSON input and output buffers with explicit `u32`
lengths/capacity and `i32` status. Its input is bounded to 16 MiB, output to 262,144 bytes. It resets
output length before refusal and never returns a host pointer. The bridge carries only copied
request/source/observation data; no credentials or proprietary objects enter the facts payload.

## Evidence and exclusions

Controlled-source tests link actual capture, retention and association code. A separate cross-
language probe loads the real built native library, exercises its actual manifest producer and
canonical facts codec, and tests raw values, unavailable inputs, authentication and stale fences.
Rust probes cover exact authenticated routes, closed input, unsupported rules and ABI limits.

These are executable source/component checks, not evidence that a proprietary host discovers,
loads or runs this adapter. Controlled source shapes are original test doubles, not copied host
implementation. Locale identities are compared exactly; unsupported forms are not normalized to
make a source appear compatible. No game installation, profile, provider, package or release is
changed. Other inputs/rules, Gateway/MCP/Harness adoption and replay, independently justified
expected outputs, and exact-host/native parity remain separate. #207 and game-core #13 stay open.
