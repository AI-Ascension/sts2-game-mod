# Feature-issue criterion-to-evidence map at exact pins

Date: 2026-09-20
Scope: the **23 open `sts2-game-mod` feature issues `#78`–`#100`**, mapped clause by clause at the exact
pins below. These are the issues whose own triage section carries the outstanding item *"Complete the
original criterion-to-evidence mapping at exact component/content/schema pins, including all named
companion owners and separately authorized native evidence"*, so this record is the concrete artefact that
item names.

Status: **every acceptance criterion of all 23 issues remains `unverified-native`.** No game was launched, no
addon installed, no profile mutated, no Steam account touched and no provider called for this record. What is
`confirmed-source` is the *pin resolution* and the *merge ancestry* of the source-only slices each issue
cites — that is, the mapping itself — not the feature acceptance it maps to.

Labels, used strictly: `confirmed` (executed or observed while writing this record, quoted with its command),
`source-derived` (read from source at the pinned commit), `inferred` (recorded elsewhere, not re-observed
here), `unverified` (no evidence).

## Exact pins

| Component | Pin | Basis |
| --- | --- | --- |
| `sts2-game-mod` source (this record) | `c881883b0eb1a63d26fff6a6262520518e0847ea` | **confirmed** — `git rev-parse origin/main` in the local clone, equal to the GitHub default-branch tip at **2026-09-20T16:10:26Z** |
| `sts2-harness` head | `9e3d20e033b5770508898e49cf3c0e4bd5fad2ed` | confirmed (GitHub API) |
| `sts2-gateway` head | `fb7e57c38a5b5732937023ee12a3e8a4af04cabe` | confirmed (GitHub API) |
| `sts2-mcp-server` head | `587a53ceb235aa5aaecf314577b075f3c45b6cbd` | confirmed (GitHub API) |
| `sts2-protocol` head | `bfe28e455de48d6d9db466bbcf6062ab5d85e9af` | confirmed (GitHub API) |

The companion-repo heads above are the default-branch tips at the moment of writing, not the pins the
individual companion merges below landed on. Two cited companion merges live in other repositories by
construction and therefore cannot be resolved in this clone; both were resolved live in their owner
repository instead, and both are recorded as `confirmed` for existence, not for acceptance:

- `5f092a9d17…` cited by `sts2-game-mod#78` — resolved in its owner repository (companion owner, not a `sts2-game-mod` commit)
- `34f68b182c…` cited by `sts2-game-mod#83` — resolved in its owner repository (companion owner, not a `sts2-game-mod` commit)
- `34f68b182c…` cited by `sts2-game-mod#84` — resolved in its owner repository (companion owner, not a `sts2-game-mod` commit)
- `34f68b182c…` cited by `sts2-game-mod#85` — resolved in its owner repository (companion owner, not a `sts2-game-mod` commit)
- `34f68b182c…` cited by `sts2-game-mod#98` — resolved in its owner repository (companion owner, not a `sts2-game-mod` commit)

## What was measured first-hand for this record

- **23 issues, 81 acceptance criteria, 0 of them checked** — i.e. the criteria checked in
  these issues is **zero**, which is the state the mapping has to explain rather than a defect in it.
- **39 merge commits cited across the 23 issues' triage sections.** Of these, **34** resolve
  in the local `sts2-game-mod` clone and every one of those is **an ancestor of the pin `c881883b0e…`**
  (`git merge-base --is-ancestor`, exit 0) — the source-only slices really are on the pinned default branch,
  so this map is anchored to a tree that contains them rather than to PR bodies.
- **5 citations (2 distinct commits)** are companion-repository merges and were
  resolved in their owner repositories (see above); they are not `sts2-game-mod` history and are not claimed as such.
- **89 dependency entries** appear across the 23 issues, de-duplicating to **16 unique**
  companion-owner issues across **6 repositories**; **12 are still open** and
  **4 are closed**.

### Companion-owner prerequisite state (measured live)

| Companion issue | State | Title |
| --- | --- | --- |
| `AI-Ascension/ascension-workflow-studio#107` | **open** | Add explicit real-run target selection and capability-checked admission |
| `AI-Ascension/sts2-game-mod#80` | **open** | Capture complete native STS2 checkpoints at explicitly supported decision bounda |
| `AI-Ascension/sts2-game-mod#83` | **open** | Expose an exact-build content manifest with origin and revision provenance |
| `AI-Ascension/sts2-game-mod#84` | **open** | Provide field-level availability, completeness and detail recovery for game read |
| `AI-Ascension/sts2-game-mod#85` | **open** | Implement global game-content enumeration, search and definition lookup |
| `AI-Ascension/sts2-game-mod#86` | **open** | Provide complete card definitions, variants, upgrade comparisons and acquisition |
| `AI-Ascension/sts2-game-mod#87` | **open** | Inspect complete live card state, modifiers and resolved cost semantics |
| `AI-Ascension/sts2-game-mod#90` | **open** | Expose power and status definitions, durations, stacking and live sources |
| `AI-Ascension/sts2-game-mod#92` | **open** | Expose character reference data, starting loadouts, pools and unlock requirement |
| `AI-Ascension/sts2-game-mod#95` | **open** | Provide enemy, boss and elite definitions with move sets and behavior rules |
| `AI-Ascension/sts2-gateway#50` | **open** | Expose owned game-process launch, attach, stop and restart for workflow executio |
| `AI-Ascension/sts2-gateway#52` | **closed** | Route bounded game-information reads through the authenticated gateway |
| `AI-Ascension/sts2-harness#127` | **closed** | Validate and deliver game-information tool results through agent and replay boun |
| `AI-Ascension/sts2-harness#94` | **open** | Execute Studio-authored workflows through authoritative live harness ports |
| `AI-Ascension/sts2-mcp-server#51` | **closed** | Expose typed searchable game-information tools with bounded detail retrieval |
| `AI-Ascension/sts2-protocol#47` | **closed** | Define versioned game-information query, identity and snapshot contracts |

The **12 open** rows are the material fact behind the unmapped item: the triage text asks for a
mapping "including all named companion owners", and a mapping cannot discharge a companion that has not
delivered. Mapping a criterion onto an open prerequisite records a route, not an acceptance.

## Per-issue criterion map

Each block quotes the issue's acceptance criteria verbatim from its body at the pins above, records the
source-only slices the issue itself credits (with verified merge ancestry), names the companion owners, and
quotes the issue's own handoff sentence. **No criterion is marked discharged.**

### `#78` — Workflows can explicitly identify/select an allowed game save profile and provision an isolated disposable automation profile without overwriting existing player data.

Criterion state: **0 of 5 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | List/select/create-disposable round-trip through the real contract with exact target profile and authoritative readback; summaries disclose no save payload or host paths. | `unverified-native` |
| 2 | In-use profile, active run, stale digest, wrong baseline, concurrent edit, unsupported slot and unknown existing directory reject without mutation. | `unverified-native` |
| 3 | Lost response/restart does not create duplicate profiles or switch the wrong slot; source identity is retained for reconciliation. | `unverified-native` |
| 4 | Disposable host tests prove existing profile contents and selection remain unchanged on every failed operation. | `unverified-native` |
| 5 | UI cannot confuse save slot with MCP/provider/workflow profile, or turn a read grant into profile mutation authority. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `2e7ccb68d0…` — Merge pull request #118 from AI-Ascension/codex/issue-78-profile-safety
- `5f092a9d17…` — companion-repository merge (resolved in its owner repository)
- `6395d8b282…` — feat(profile): cover safe source-only save profile discovery (#125)
- `ed5ef30acd…` — feat: add safe owner-local profile discovery boundary

Companion owners named as prerequisites: `AI-Ascension/sts2-harness#94`, `AI-Ascension/sts2-gateway#50`, `AI-Ascension/ascension-workflow-studio#107`.

Owner handoff, quoted: "Continue from the merged save-profile safety fixtures. Game-mod owns real host-thread selection and authoritative readback; gateway #51 and harness #102 own provisioning and workflow integration. Verify durable reconciliation and the original disposable-host/profile/UI criteria before closure."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#79` — Close the source-to-native evidence gap for the existing standard seeded-run adapter without mistaking earlier practice-mode replay for standard-run proof.

Criterion state: **0 of 4 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | A static/disposable probe test establishes argument/authority and data-redaction checks without requiring a game in default CI. | `unverified-native` |
| 2 | Authorized native lane proves supported standard setup, canonical seed, causal first state and settled operation at exact pins. | `unverified-native` |
| 3 | Duplicate/uncertain/stale attempts do not start another run and preserve explicit outcome evidence. | `unverified-native` |
| 4 | Pre-existing profiles/processes remain unchanged and cleanup/rollback is recorded; an unavailable native lane remains unverified and open. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `b77ed8d396…` — Merge pull request #113 from AI-Ascension/codex/issue-79-seeded-probe

Companion owners named as prerequisites: none.

Owner handoff, quoted: "Use the merged static standard-context admission probe as preparation, then obtain the named native-test authorization and exact supported host/addon/profile. Verify native setup, causal first state, settlement, duplicate/uncertain/stale handling and cleanup against the original criteria."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#80` — None

Criterion state: **0 of 6 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Pinned synthetic canonical vectors agree byte-for-byte with the protocol witness; rejection covers duplicate keys, numeric edge cases, unknown required values and size bounds. | `unverified-native` |
| 2 | Capture the same quiescent native boundary twice with no gameplay effect: canonical state bytes match while occurrence identities remain distinct. Deliberately changing an RNG cursor or hidden future-affecting field changes identity even if the public observation is identical. | `unverified-native` |
| 3 | Race capture against a queued action, save and room transition: either obtain one coherent boundary or explicit rejection, never mixed state. Capture itself does not advance RNG or mutate gameplay. | `unverified-native` |
| 4 | Every advertised phase has schema coverage and an exact-host capture test, including later-turn combat and later-floor map state. Unsupported phases return typed rejection without fabricated artifacts. | `unverified-native` |
| 5 | Private payloads and exact digests are absent from model/tool/browser/log error output; public-reference schema conformance passes. | `unverified-native` |
| 6 | Run applicable repository Rust, policy and managed gates; document exact host/source hashes and separate synthetic, build and native capture evidence. Restoration remains explicitly unverified here. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `1404461cb9…` — Merge pull request #114 from AI-Ascension/codex/issue-80-checkpoint-inventory
- `18f3e8554a…` — Add checkpoint capture admission barrier and operation ledger (source-only slice of #80) (#152)
- `5dc9326a1e…` — feat: source-only restricted canonical checkpoint encoder (Refs #80) (#147)
- `c947a93098…` — fix: enforce pinned canonical checkpoint profile (#148)
- `ec68305b67…` — feat(checkpoint): add fail-closed capture port and vectors

Companion owners named as prerequisites: none.

Owner handoff, quoted: "Continue from the merged restricted canonical encoder and protocol-vector correction. Game-mod must implement and independently verify exact-build native field/RNG extraction and a coherent host-thread capture boundary for every advertised phase. Keep all native phases unavailable until their separate evidence passes."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#81` — None

Criterion state: **0 of 7 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Native capture -> fresh-process restore -> recapture matches canonical bytes at every advertised phase, including a later-floor and later-turn checkpoint. | `unverified-native` |
| 2 | Restore twice to independent destinations and apply the same bounded action sequence: compare each supported state boundary and controlled external input, not only terminal outcome. | `unverified-native` |
| 3 | Restore the same checkpoint twice and choose two different legal actions: both settle, produce separate occurrences, and leave the source restorable. | `unverified-native` |
| 4 | Wrong host/profile/coverage, missing blob, forged producer receipt, oversized data and digest mismatch reject before effects. | `unverified-native` |
| 5 | Inject interruption before effect, after effect and before receipt persistence; reconcile accurately with no duplicate restore, and no action admission on an unknown destination. | `unverified-native` |
| 6 | Old epoch/provider proposal/catalog is rejected after restore. Failed destination cleanup cannot erase source evidence or be reported as successful restoration. | `unverified-native` |
| 7 | Run managed/Rust owner gates and retain exact native compatibility and continuation evidence; untested phases remain unavailable. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `3802b3c419…` — Merge pull request #119 from AI-Ascension/codex/issue-81-restore-boundary

Companion owners named as prerequisites: `AI-Ascension/sts2-game-mod#80`.

Owner handoff, quoted: "Use the merged restore-boundary ADR and the source codec delivered under #80 to implement the inverse codec, pre-effect closure checks, quarantine, reconciliation and destination recapture. Complete #80/#78 and the named gateway gates before native integration."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#82` — Establish, for an exact supported host build, whether the native seed controls every future-affecting RNG stream and external input across process launches. Publish a usable capability result rather than assuming one StringSeed proves complete determinism.

Criterion state: **0 of 5 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Inventory entries are tied to exact native assembly/source evidence and tests; every gameplay-relevant discovered stream has a derivation and lifecycle classification. | `unverified-native` |
| 2 | At least three independent cold launches of each selected fixture agree on the complete initial RNG witness; repeat reads do not advance any stream. | `unverified-native` |
| 3 | Controlled same-action probes cover every advertised RNG category; changing a stream cursor is detectable even when public observations match. | `unverified-native` |
| 4 | Timing/poll frequency and process identity perturbations either preserve gameplay state or produce a recorded unsupported/failure result. Unknown/new streams cannot silently pass. | `unverified-native` |
| 5 | A proprietary-host-free synthetic suite checks witness schema and rejection; an authorized exact-host record is separately required for any native certification. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `a88e55107c…` — Merge pull request #117 from AI-Ascension/codex/issue-82-rng-audit
- `e01a7d6607…` — feat: add bounded RNG audit witness

Companion owners named as prerequisites: none.

Owner handoff, quoted: "Continue from the merged private synthetic RNG witness. Tie discovered streams, derivation and lifecycle coverage to exact native evidence, then obtain the authorized cold-launch and same-action comparisons required by this issue. Synthetic fingerprints do not certify native RNG."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#83` — An agent can identify the exact content set behind a lookup and determine whether definitions from two sessions are comparable.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Two identical installations give equivalent semantic manifests; locale change changes text identity without changing entity identity. | `unverified-native` |
| 2 | Adding/removing/overriding a synthetic content package changes the appropriate manifest/definition revisions and expires old queries. | `unverified-native` |
| 3 | The exact-host inventory counts match the owner registry, including duplicate display names and unsupported family reporting; no content is silently omitted. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `34f68b182c…` — companion-repository merge (resolved in its owner repository)
- `491d44ea05…` — Merge pull request #115 from AI-Ascension/codex/issue-83-content-manifest
- `510b9503ad…` — Require registry evidence for content manifests and separate adapter identity (#149)
- `af7120d87c…` — feat: add source-only content manifest producer
- `e401ee0629…` — Merge pull request #151 from AI-Ascension/codex/game-mod-83-manifest-doc-fix-20260915

Companion owners named as prerequisites: `AI-Ascension/sts2-protocol#47`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Protocol and named consumers must agree/admit a complete typed manifest representation for package/version/order, inventory totals, unhandled families and override chains. Then compose the payload through gateway/MCP/harness and obtain separately authorized exact-host registry comparison. Existing v1 consumer pins and merged source-only manifest work do not satisfy those gates."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#84` — An agent can tell why a value is absent and retrieve supported details instead of interpreting null as zero or assuming a bounded result is complete.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Fixtures exercise zero, empty, unsupported, denied, stale, reflection failure and genuinely unknown values with distinct machine-readable outcomes. | `unverified-native` |
| 2 | Basic-to-detail traversal recovers an omitted supported field and rejects an old epoch; a bounded collection can be fully traversed without loss. | `unverified-native` |
| 3 | Unknown/protected field requests, overlarge detail sections and failed native access cannot become successful empty results. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `21f1fdf938…` — Merge pull request #116 from AI-Ascension/codex/issue-84-availability
- `34f68b182c…` — companion-repository merge (resolved in its owner repository)
- `f7c8a2b807…` — feat: add owner-local field availability and detail recovery

Companion owners named as prerequisites: `AI-Ascension/sts2-protocol#47`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Continue from the merged owner-local availability/completeness and detail-recovery fixtures. Verify the feature-specific shared contract mapping and agent/replay delivery through harness #127, then validate native field extraction and original stale/protected/oversized acceptance cases."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#85` — An agent can discover and inspect game content that is not currently in its hand, inventory, room or run.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | List every available kind and reconcile IDs/counts with the authoritative registry; an item absent from the current run is searchable and retrievable. | `unverified-native` |
| 2 | Duplicate names, non-ASCII text, aliases, empty queries, unknown IDs and pagination boundaries give deterministic results. | `unverified-native` |
| 3 | Content reload invalidates old cursors; repeated queries do not advance host RNG, create gameplay entities or change profile progress. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `29ecd5dfd8…` — feat: add source-only content index producer
- `34f68b182c…` — companion-repository merge (resolved in its owner repository)

Companion owners named as prerequisites: `AI-Ascension/sts2-protocol#47`, `AI-Ascension/sts2-game-mod#83`, `AI-Ascension/sts2-game-mod#84`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Continue from the merged bounded content index. Map exact-build native definition registries to the manifest, compose the approved protocol/gateway/MCP/harness path, and verify registry-to-query coverage, cursor invalidation and read-only behavior."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#86` — An agent can inspect any card and compare every supported upgrade/variant before acquiring or playing it.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Registry-to-query coverage accounts for every card definition and supported variant, with no sample-only allowlist. | `unverified-native` |
| 2 | Fixtures cover ordinary, special/X-cost, multiple-upgrade, generated, duplicate-title and dynamic-effect cards; compare output matches authoritative before/after definitions. | `unverified-native` |
| 3 | Exact-host reads leave live deck, upgrade state, profile and RNG unchanged; missing formulas are explicitly unavailable rather than fabricated. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `520febdc3f…` — feat: add source-only card definitions

Companion owners named as prerequisites: `AI-Ascension/sts2-game-mod#85`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Continue from the merged owner-local source/fixture slice below. Game-mod must map exact-build native data to the defined records; protocol and gateway/MCP/harness owners must agree and verify the complete feature-specific agent/replay payload. Resolve the existing predecessors and obtain the original native/read-only/visibility acceptance evidence before closure."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#87` — An agent can distinguish two copies of a card and read the actual modifications and costs applying to each at the current decision.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Two same-definition cards with different upgrades or temporary modifiers return different instance details and correct common definition linkage. | `unverified-native` |
| 2 | A cost-changing effect expires correctly across turns; special and alternate costs retain semantics and unknown contributors are explicit. | `unverified-native` |
| 3 | Concurrent card movement/upgrade invalidates old detail instead of returning mixed state; reads make no gameplay changes. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `9358479091…` — feat: add owner-local live card state projection

Companion owners named as prerequisites: `AI-Ascension/sts2-game-mod#86`, `AI-Ascension/sts2-game-mod#84`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Continue from the merged owner-local source/fixture slice below. Game-mod must map exact-build native data to the defined records; protocol and gateway/MCP/harness owners must agree and verify the complete feature-specific agent/replay payload. Resolve the existing predecessors and obtain the original native/read-only/visibility acceptance evidence before closure."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#88` — An agent can learn what any relic does and inspect the state of an owned relic that affects the next decision.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Catalog enumeration matches relic registry; queryable definitions exist for relics absent from the run. | `unverified-native` |
| 2 | Fixtures and native cases cover passive, charged, turn-counter, room-counter and multiple-counter relics; changes match the host at the same snapshot. | `unverified-native` |
| 3 | No-state relics report not_applicable and failed state reads report unavailable; neither becomes an invented zero charge. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `715fba2b80…` — feat: add source-only relic definitions and live state

Companion owners named as prerequisites: `AI-Ascension/sts2-game-mod#85`, `AI-Ascension/sts2-game-mod#84`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Continue from the merged owner-local source/fixture slice below. Game-mod must map exact-build native data to the defined records; protocol and gateway/MCP/harness owners must agree and verify the complete feature-specific agent/replay payload. Resolve the existing predecessors and obtain the original native/read-only/visibility acceptance evidence before closure."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#89` — An agent can compare potions and understand the visible effect and modifiers of a potion before using or buying it.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Every registry potion can be retrieved with effect data; a non-owned potion is inspectable through reference search. | `unverified-native` |
| 2 | A modifier changes the appropriate effective parameter while base definition stays unchanged; replaced/discarded slot references become stale. | `unverified-native` |
| 3 | Unavailable/invalid targets and random effects are represented honestly; repeated reads leave potion inventory and RNG unchanged. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `4968f7ae66…` — feat: add source-only potion definitions and live state

Companion owners named as prerequisites: `AI-Ascension/sts2-game-mod#85`, `AI-Ascension/sts2-game-mod#84`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Continue from the merged owner-local source/fixture slice below. Game-mod must map exact-build native data to the defined records; protocol and gateway/MCP/harness owners must agree and verify the complete feature-specific agent/replay payload. Resolve the existing predecessors and obtain the original native/read-only/visibility acceptance evidence before closure."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#90` — An agent can interpret each observed buff/debuff and understand when and how it changes.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Fixtures cover stackable, nonstacking, duration-based, multi-counter and amountless statuses, including duplicate definitions from different sources. | `unverified-native` |
| 2 | Turn/round/room expiry and stacking transitions agree with pinned host-visible state and reference rules. | `unverified-native` |
| 3 | Hidden sources remain denied/unknown and missing state is never reported as no status; all catalog statuses are accounted for. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `3053f58c59…` — feat: add source-only power and status state

Companion owners named as prerequisites: `AI-Ascension/sts2-game-mod#85`, `AI-Ascension/sts2-game-mod#84`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Continue from the merged owner-local source/fixture slice below. Game-mod must map exact-build native data to the defined records; protocol and gateway/MCP/harness owners must agree and verify the complete feature-specific agent/replay payload. Resolve the existing predecessors and obtain the original native/read-only/visibility acceptance evidence before closure."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#91` — An agent can resolve terms encountered in card, relic, potion, status and event descriptions into precise definitions.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Every keyword reference in supported entity definitions resolves or has an explicit unavailability record. | `unverified-native` |
| 2 | Aliases, duplicate names, parameterized text, locale fallback and cyclic references yield deterministic bounded results. | `unverified-native` |
| 3 | An agent can follow card -> keyword -> related rule without a web lookup or raw host access. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `b427fe71c3…` — feat: add source-only mechanics glossary and cross-references

Companion owners named as prerequisites: `AI-Ascension/sts2-game-mod#85`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Continue from the merged owner-local source/fixture slice below. Game-mod must map exact-build native data to the defined records; protocol and gateway/MCP/harness owners must agree and verify the complete feature-specific agent/replay payload. Resolve the existing predecessors and obtain the original native/read-only/visibility acceptance evidence before closure."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#92` — An agent can compare playable characters and understand their starting configuration and special mechanics.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Enumerating the character registry matches queryable definitions and each starting deck/relic reference resolves. | `unverified-native` |
| 2 | At the exact supported build, observed new-run starting values agree with the selected character/configuration definition. | `unverified-native` |
| 3 | Locked character requirements and unavailable mode variants are reported without modifying progress; locale changes preserve identity. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `6f40ddb344…` — feat: add source-only character reference catalog

Companion owners named as prerequisites: `AI-Ascension/sts2-game-mod#85`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Continue from the merged owner-local source/fixture slice below. Game-mod must map exact-build native data to the defined records; protocol and gateway/MCP/harness owners must agree and verify the complete feature-specific agent/replay payload. Resolve the existing predecessors and obtain the original native/read-only/visibility acceptance evidence before closure."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#93` — An agent can read all decision-relevant public state for each supported character, including special resources and controlled secondary entities.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | An exact-build mechanic inventory maps every public field to a query path or unresolved acceptance item; no supported character closes with an untracked gap. | `unverified-native` |
| 2 | Each inventoried mechanic has creation/change/expiry or removal tests and at least one native observation comparison. | `unverified-native` |
| 3 | Two owners with different secondary entities/resources remain isolated; stale/unknown mechanic variants reject or disclose unsupported without dropping data. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `a66a4e51dc…` — feat: project character resources and secondary entities (#135)

Companion owners named as prerequisites: `AI-Ascension/sts2-game-mod#92`, `AI-Ascension/sts2-game-mod#84`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Continue from the merged owner-local source/fixture slice below. Game-mod must map exact-build native data to the defined records; protocol and gateway/MCP/harness owners must agree and verify the complete feature-specific agent/replay payload. Resolve the existing predecessors and obtain the original native/read-only/visibility acceptance evidence before closure."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#94` — An agent can distinguish the permanent deck from the current combat piles and inspect public counters and resolving state.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Draw, play, discard, exhaust, generate, destroy and shuffle cases preserve correct counts and instance membership across all supported zones. | `unverified-native` |
| 2 | Public unordered composition does not expose secret order; known top-card information is included only under its permitted visibility. | `unverified-native` |
| 3 | Counters reset at the correct boundaries and a concurrent resolving action returns one coherent snapshot or a typed stale/busy result. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `6260fac0bf…` — feat: add source-only combat bookkeeping projection

Companion owners named as prerequisites: `AI-Ascension/sts2-game-mod#87`, `AI-Ascension/sts2-game-mod#84`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Continue from the merged owner-local source/fixture slice below. Game-mod must map exact-build native data to the defined records; protocol and gateway/MCP/harness owners must agree and verify the complete feature-specific agent/replay payload. Resolve the existing predecessors and obtain the original native/read-only/visibility acceptance evidence before closure."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#95` — An agent can inspect any enemy's reference stats, possible moves, phases and conditional behavior without encountering it first.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Every enemy registry entry is discoverable and all reported move/encounter/status references resolve. | `unverified-native` |
| 2 | Fixtures cover a multi-phase boss, compound move, conditional transition, summon and difficulty-scaled stats; supported native cases agree with documented rules. | `unverified-native` |
| 3 | A fair-play query reveals reference possibilities but not the live unrevealed next move; unknown transition weights remain explicit. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `34126197b2…` — feat: expose enemy definitions, move sets and behavior rules (#137)

Companion owners named as prerequisites: `AI-Ascension/sts2-game-mod#85`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Continue from the merged owner-local source/fixture slice below. Game-mod must map exact-build native data to the defined records; protocol and gateway/MCP/harness owners must agree and verify the complete feature-specific agent/replay payload. Resolve the existing predecessors and obtain the original native/read-only/visibility acceptance evidence before closure."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#96` — An agent can read all public components of an enemy's current intention, including non-damage effects and intended targets.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Attack-plus-debuff, defend-plus-buff, multi-hit, summon and targetless cases round-trip structured components without loss. | `unverified-native` |
| 2 | Public ally/enemy target references are correct in co-op; hidden target sentinels remain unavailable. | `unverified-native` |
| 3 | Native UI comparison verifies every advertised intent family and state changes invalidate earlier intent detail. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `6d38bb3676…` — feat: expose structured enemy intents and public target information (#136)

Companion owners named as prerequisites: `AI-Ascension/sts2-game-mod#95`, `AI-Ascension/sts2-game-mod#90`, `AI-Ascension/sts2-game-mod#84`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Continue from the merged owner-local source/fixture slice below. Game-mod must map exact-build native data to the defined records; protocol and gateway/MCP/harness owners must agree and verify the complete feature-specific agent/replay payload. Resolve the existing predecessors and obtain the original native/read-only/visibility acceptance evidence before closure."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#97` — An agent can inspect act structures, encounter pools and map-generation rules independently of the currently displayed map.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Manifest enumeration accounts for every act and encounter in the supported build and resolves each enemy/pool reference. | `unverified-native` |
| 2 | Mode/difficulty variants and encounter weights retain exact units/normalization semantics; unverified values do not become asserted probabilities. | `unverified-native` |
| 3 | Existing runtime-map-v1 conformance remains green and reference lookup does not reveal actual hidden future room contents. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `efe3f8cad9…` — feat: expose act, encounter and map-generation reference data (#143)

Companion owners named as prerequisites: `AI-Ascension/sts2-game-mod#85`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Continue from the merged owner-local source/fixture slice below. Game-mod must map exact-build native data to the defined records; protocol and gateway/MCP/harness owners must agree and verify the complete feature-specific agent/replay payload. Resolve the existing predecessors and obtain the original native/read-only/visibility acceptance evidence before closure."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#98` — An agent can query permitted known map topology during combat or another screen and distinguish retained knowledge from currently verified navigation state.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Open map -> query -> close map -> query from combat returns correct known topology with honest freshness, without opening the screen. | `unverified-native` |
| 2 | Before any permitted observation, data forbidden by policy stays unavailable; act transition/restore/new run cannot reuse an old map as current. | `unverified-native` |
| 3 | Retained map reads cannot produce actionable stale travel references; legacy map tests and authoritative generation fences remain valid. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `02288250ef…` — feat: retain already-public map knowledge while the map screen is closed (#144)
- `34f68b182c…` — companion-repository merge (resolved in its owner repository)

Companion owners named as prerequisites: `AI-Ascension/sts2-protocol#47`, `AI-Ascension/sts2-game-mod#84`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Continue from the merged retained-map reader and synthetic freshness fixtures. Connect the exact-host public-map source and approved consumer path, then verify open-map/close-map/act-transition/restore cases without promoting retained topology to live travel authority."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#99` — An agent can read an event and understand each offered choice, and can look up the event's reference branches before encountering it.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | A multi-page event, conditional disabled option, numeric cost, random outcome and selector-producing choice have complete inspectable payloads. | `unverified-native` |
| 2 | Rendered text and visible requirements match the supported host UI; all catalog events and branches are accounted for. | `unverified-native` |
| 3 | Reading/reference search causes no selection or RNG effect; stale option references and hidden outcomes are rejected/withheld correctly. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `01309d5fe9…` — feat: source-only event reference catalog (Refs #99) (#145)

Companion owners named as prerequisites: `AI-Ascension/sts2-game-mod#85`, `AI-Ascension/sts2-game-mod#84`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Continue from the merged owner-local source/fixture slice below. Game-mod must map exact-build native data to the defined records; protocol and gateway/MCP/harness owners must agree and verify the complete feature-specific agent/replay payload. Resolve the existing predecessors and obtain the original native/read-only/visibility acceptance evidence before closure."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

### `#100` — An agent can compare the exact contents and conditions of every offered reward and inspect the rules that generated the reward pool.

Criterion state: **0 of 3 checked** at the pins.

| # | Acceptance criterion (verbatim) | Status |
| --- | --- | --- |
| 1 | Multiple reward groups, card pick/skip, full potion slots, modified currency and special reward fixtures expose sufficient data to choose correctly. | `unverified-native` |
| 2 | Every offered content reference resolves; claiming or advancing a room invalidates old offer references and never returns a later offer under the old ID. | `unverified-native` |
| 3 | Host UI values match live reward details and reference queries do not consume RNG or claim rewards. | `unverified-native` |

Source-only evidence credited by the issue (merge ancestry verified against the pin):
- `169c030c5a…` — feat: add source-only reward offer catalog (Refs #100) (#146)

Companion owners named as prerequisites: `AI-Ascension/sts2-game-mod#85`, `AI-Ascension/sts2-game-mod#84`, `AI-Ascension/sts2-gateway#52`, `AI-Ascension/sts2-mcp-server#51`, `AI-Ascension/sts2-harness#127`.

Owner handoff, quoted: "Continue from the merged owner-local source/fixture slice below. Game-mod must map exact-build native data to the defined records; protocol and gateway/MCP/harness owners must agree and verify the complete feature-specific agent/replay payload. Resolve the existing predecessors and obtain the original native/read-only/visibility acceptance evidence before closure."

Discharge route: this issue closes only when its own criteria are met at these pins **and** the named
companion owners above have delivered, with the separately authorized native evidence recorded. Neither is
satisfied at this date; no box was ticked by this record.

## What this establishes, and what it does not

**Establishes**, at the pins above: that the source-only slices credited by these 23 issues are on the pinned
default branch (ancestry verified, not inferred from PR bodies); that the companion-owner prerequisite set is
known and its live state is recorded; and that the criteria of all 23 issues are unchecked, with a per-issue
route recorded for each.

**Does not establish**: any feature acceptance. A merged source-only slice is a partial contribution to the
source half of an end-to-end feature, and the issues' own text says so — "merged UI code or a fixture-only
demonstration is not end-to-end completion". No criterion here is native-verified, integrated-verified, or
independently reviewed, and marking one as discharged from this record would overstate it.

**A limit named rather than absorbed**: the pins are the *current* default-branch tips, while the staged
native campaign set is older (see `seeded-run-criterion-evidence-map-20260917.md`, section "Staged set versus
current heads"). A native campaign that runs against the staged set does not discharge these rows; it must
either re-pin to the set it ran or rebuild at these pins and record the new hashes.

## External gate

The native half of every row above requires an explicitly authorized native-test session with an exact
supported host/addon build and an isolated disposable profile. That authorization is external to this
environment, so no row is claimed here on its behalf.

## Related records

- [Seeded-run criterion-to-evidence map at exact pins](seeded-run-criterion-evidence-map-20260917.md)
- [Checkpoint coverage inventory](checkpoint-coverage-inventory-20260917.md)
- [Exact restore staging](exact-restore-staging-20260916.md)

