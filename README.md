<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/AI-Ascension/.github/main/profile/assets/banner-dark.svg">
  <img alt="AI-Ascension — Inspect how AI requests to a game get fenced, one Rust contract at a time. Runtime: unverified. Deterministic tests: confirmed." src="https://raw.githubusercontent.com/AI-Ascension/.github/main/profile/assets/banner-light.svg" width="100%">
</picture>

# Slay the Spire 2 Mod

Part of [Ascension](https://github.com/AI-Ascension/sts2-harness), the AI
Ascension flagship toolkit. The repository slug remains `sts2-game-mod`
until an approved rename; **The Climb — by AI Ascension** presents only the
host evidence that is actually available.

> **AI-Ascension · tier 1: game-process adapter** — Game-process adapter: a bounded main-thread work queue, versioned ABI check, and HTTP request admission limits.
>
> **Status:** deterministic tests, managed load-smoke, and bounded native STS2 v0.107.1 Windows/Linux runtime-v3 evidence `confirmed` · model-controlled campaigns reached Defeat and replayed in fresh processes; forced native Victory fixtures confirm terminal observation only · model-played Victory, all campaign paths, and broader compatibility `unverified`. See [live combat scope](docs/LIVE_COMBAT_DEMO.md) and the [dated evidence](docs/evidence/).
> **Proof:** [45-second browser replay](https://ai-ascension.github.io/proof.html) · [Evidence ledger](https://ai-ascension.github.io/evidence.html) · [This repository on the map](https://ai-ascension.github.io/repositories.html#sts2-game-mod)
> **Owner:** The mod owner is responsible for the managed loader package, host boundary, main-thread queue, ABI gate, HTTP admission, and Rust/native seam; the game host stays authoritative.
> **Contribute:** [Organization guide](https://github.com/AI-Ascension/.github/blob/main/CONTRIBUTING.md) · [First tasks](https://ai-ascension.github.io/contributing.html)
>
> AI-Ascension is an independent project. It is not affiliated with or endorsed by Mega Crit or Valve and grants no rights to game files, assets, or marks.

Status: the target-owned boundary seams and one deterministic `poc-v1` fake mapping compile and
have tests. A thin managed loader package loads the Rust companion and has passed a real load-smoke
launch against the recorded STS2 host. The bounded runtime probe and the runtime-v3 gameplay bridge
have also been exercised in authorized disposable profiles. Current native evidence is scoped to
STS2 v0.107.1 on the tested Windows and Linux guests; it records model-controlled setup-to-Defeat
campaigns and fresh replays, plus separate forced terminal-observation fixtures.

The native standard `seeded-run-v1` adapter is present at source/component level at current mod main
[`caae865986d2274736d92b4f9be2bbda24bab83d`](https://github.com/AI-Ascension/sts2-game-mod/commit/caae865986d2274736d92b4f9be2bbda24bab83d).
It exposes authenticated start and read-only reconciliation routes, accepts only the bounded
standard Ironclad context, binds a fresh profile baseline and ordered native acts, and requires
canonical seed readback plus a `run_started` witness before settlement. Its copied protocol artifact
is `seeded-run-v1` at schema digest `5c659f344be78f84e8d783986925d462714f933cac95d18943358992f7d3e2b8`,
from protocol main `d3ab5fca7d9d74bb31eeb3e5b343d8024ee44404`. Source/build and synthetic probes do
not prove a live seeded run; independent disposable-host verification, save isolation, and broader
compatibility remain unverified. See [ADR 0031](docs/decisions/0031-native-standard-seeded-run-adapter.md).

The additive Runtime-v4 expert state/action bridge is present at source/component level at current
mod main
[`caae865986d2274736d92b4f9be2bbda24bab83d`](https://github.com/AI-Ascension/sts2-game-mod/commit/caae865986d2274736d92b4f9be2bbda24bab83d).
Its copied protocol artifacts are checksum-verified against current protocol main
`d3ab5fca7d9d74bb31eeb3e5b343d8024ee44404`; the expert-state and expert-action schema digests are
`0ee034d5da83f34e9fa0ba23038738d56ef8cfccb1c6e752af3ab63d212c8e42` and
`393318bda8c3522c0ecbacc78b95471a9f4dc3f825169d2048f4c74a7b7f2929`. Rust/managed source and
synthetic route/admission checks cover the bounded expert surface. Live expert gameplay, potion
settlement, exact-host/package builds, and broader compatibility remain `unverified`.

The additive `runtime-map-v1` read profile is present at source/component level. It serves
authenticated `GET /api/map/v1/snapshot`, copies only bounded player-visible topology and current
legal bindings, and rechecks the gameplay generation before returning owned values. The copied
artifact and source probes pass; live map extraction, provider delivery, and settled navigation
remain `unverified` (see [ADR 0035](docs/decisions/0035-runtime-map-projection.md)).

The source-only `runtime-v4-expert-rest-action-v1` candidate adds native rest-option actions and
generation-fenced Smith/Mend selector follow-ups with option-specific completion witnesses and
same-operation recovery. Its protocol manifest remains `candidate` / `none_admitted`; gateway,
MCP, harness, live rest settlement, exact-host/package, and release evidence remain `unverified`
(see [ADR 0036](docs/decisions/0036-runtime-v4-rest-option-action.md)).

## Responsibility and consumers

The mod owner maintains the managed loader, host translation, main-thread boundary, authoritative
local HTTP adapter, and narrow Rust/native seam. The game host is the authority for live state and
mutations. The gateway consumes the mod's owner-local HTTP contract; MCP and harness traffic
reaches it only through their separate gateway and coordinator responsibilities.

This target does not own domain policy, gateway lifecycle or routing, MCP framing, or
model/provider orchestration. It consumes the checked-in `sts2-protocol/poc-v1` release-like
artifact as inert data; it does not link a protocol implementation or a sibling repository.

## Current contents

- [experiments/managed-rust-interop/](experiments/managed-rust-interop/) contains the managed .NET 9
  loader package, its unique Rust companion library, the manifest, and the reproducible packaging
  command. The package now contains the load-smoke/ABI path, bounded runtime probe source, and
  the opt-in repeat-seed practice replay settings path.
- [crates/host/](crates/host/) owns the host port, bounded main-thread queue, dispatcher, and
  versioned ABI descriptor validation.
- [crates/http-adapter/](crates/http-adapter/) owns a transport-free HTTP request boundary and
  bounded admission guard; it does not open a listener or define public routes.
- [crates/game-mod/](crates/game-mod/) composes those seams and makes admission-versus-pump
  behavior explicit for the managed host integration.
- [protocol-artifact/poc-v1/](protocol-artifact/poc-v1/) is the offline copied artifact consumed by
  the deterministic mod/core boundary test.
- [protocol-artifact/runtime-v2/](protocol-artifact/runtime-v2/) is the offline copied release-like
  artifact for the bounded Runtime-v2 fake seam; it is pinned to schema digest
  `f7963b19c8ed5bbdc02c08e83c7a2e16c4771ed5eb798b29a8208d7a917a86c2` and has no sibling checkout
  dependency.
- [protocol-artifact/runtime-v4-expert/](protocol-artifact/runtime-v4-expert/) and
  [protocol-artifact/runtime-v4-expert-action/](protocol-artifact/runtime-v4-expert-action/) are
  inert copied artifacts for the additive expert state/action bridge. Their schema digests are
  `0ee034d5da83f34e9fa0ba23038738d56ef8cfccb1c6e752af3ab63d212c8e42` and
  `393318bda8c3522c0ecbacc78b95471a9f4dc3f825169d2048f4c74a7b7f2929`.
- [protocol-artifact/seeded-run-v1/](protocol-artifact/seeded-run-v1/) is the copied explicit
  seeded-launch contract at schema digest
  `5c659f344be78f84e8d783986925d462714f933cac95d18943358992f7d3e2b8`.
- [protocol-artifact/runtime-map-v1/](protocol-artifact/runtime-map-v1/) is the inert copied map
  read artifact, pinned to schema digest
  `ceab0d2dfc471d1ec36d12edaf4654b8c7fdced06548bf47265e11c63f98115b`.
- [protocol-artifact/runtime-v4-expert-rest-action/](protocol-artifact/runtime-v4-expert-rest-action/)
  is the inert copied rest-action candidate, pinned to schema digest
  `bb3555fae28eb1f79d08a15e9884696a579e4c20836f5016509f17e0f4c36fbd` and not admitted to any
  external consumer.
- `crates/game-mod/src/poc/` maps state reads and one typed `use_budget` action through a narrow
  `PocCorePort`, records correlation/instance/generation metadata, and emits one settled-effect
  witness for an accepted action.
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) records the target boundary and dependency graph.
- [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md) records evidence levels and host claims.
- [docs/repeat-seed.md](docs/repeat-seed.md) records the opt-in custom-run replay scope, safety
  invariants, and runtime evidence boundary.
- [docs/evidence/runtime-addon-load-smoke-20260902.md](docs/evidence/runtime-addon-load-smoke-20260902.md)
  records the exact installed-host load-smoke inputs and observed log marker.
- [docs/evidence/runtime-v1-host-live-20260902.md](docs/evidence/runtime-v1-host-live-20260902.md)
  records the focused runtime probe against the exact installed host and disposable profile.
- [docs/evidence/native-victory-observation-20260906.md](docs/evidence/native-victory-observation-20260906.md)
  records forced Windows/Linux terminal observation with the fixture excluded from normal builds.
- [docs/evidence/train-gpu-lifecycle-20260906.md](docs/evidence/train-gpu-lifecycle-20260906.md)
  records the authorized Train GPU lifecycle and post-boot isolated combat scope.
- [docs/decisions/](docs/decisions/) records the managed/native, ownership, scaffold, and
  sixth-target and Wave 2 initialization decisions.
- [tools/repo-policy/](tools/repo-policy/) is the target-local Rust governance checker.

The managed loader, packaging, authenticated runtime routes, and runtime-v3 gameplay host source
are implemented for the reviewed bounded path. The Rust POC core port remains a fake seam. Native
evidence confirms the exact v0.107.1 Windows/Linux campaign and replay paths recorded by the harness;
it does not establish every character, seed, branch, host patch, or multiplayer behavior.

The native co-op source candidate is now wired through the managed game-thread callback path and
native listener routes 16 through 20. It binds host actions, shared votes, peer rejoin, and
same-operation recovery to the first-party multiplayer service and shares the pending-mutation gate
with the seeded and gameplay profiles. `coop-native-v1` remains an unadmitted candidate contract:
its protocol schema/artifact and gateway/MCP/harness consumers are owned by separate targets and
are not present in this target. Source probes and an exact-host compile cover the managed boundary;
they do not establish a live two-peer session, native effect settlement, disconnect/rejoin behavior,
or multiplayer compatibility. The admitted `coop-synchronization-v1` profile remains a separate
read-only gateway/MCP coordinator-report contract.

## Steam Workshop package

The target now contains a first-party Steam Workshop package boundary. The Rust
sts2-game-mod module validates the sts2-workshop-manifest-v1 shape, exact compatibility metadata,
file roles, path safety, and install-state decisions. The managed loader validates an installed
package's exact allowlist, reparse-point status, file sizes, SHA-256 values, and deterministic
content digest before the native companion is loaded.

tools/workshop/package-item.sh stages the current three-file runtime payload, writes
sts2-workshop-manifest.json and SHA256SUMS, and emits an operator-only VDF beside the Workshop
content directory. It requires the consumer App ID, published file ID, game/package versions,
source revision, and preview image as inputs; none are stored in this repository. The fixture-only
self-test is bash tools/workshop/test-package-item.sh.

The first-party package may contain executable mod files, but only the exact allowlisted files
under the configured App ID and published file ID are accepted. Arbitrary third-party Workshop
DLLs, native libraries, scripts, archives, and renamed executables are rejected. Steam publication,
subscription/download callbacks, game discovery, and host compatibility remain unverified.

The Runtime-v2 seam described above is intentionally fake-only: its deterministic tests prove bounded
admission, exactly-once in-memory application, retained receipts, and reconciliation for argument-free
`end_turn`. Those tests do not characterize the separate managed/native Runtime-v3 host path; see the
dated campaign records above for that path's bounded evidence and limits.

## Evidence and provenance

The foundation and boundary seam are original target documentation and source tailored from the project
standards. No product implementation source was copied from another implementation. No proprietary
host file, save, credential, personal path, or generated output is distributed. The local load-smoke
uses the operator's installed host assembly without adding it to this repository or release output.

The POC behavior is test-confirmed only for the local fake core port: a state read, one accepted
action, and one zero-unit rejection preserve the bounded state and effect witness. The managed
loader has also passed load-smoke in STS2 v0.107.1 and logged a successful Rust ABI call. The
runtime-v1 probe confirmed live HTTP, managed main-thread dispatch, and a host-visible overlay
witness in that exact host. Later dated evidence confirms bounded runtime-v3 model-controlled
campaigns and fresh-process replays to Defeat on Windows and Linux, while a separate forced native
fixture confirms Victory observation and disabled input. The fixture does not prove a model-played
Victory; broader compatibility remains unverified. See the [Windows campaign and replay](https://github.com/AI-Ascension/sts2-harness/blob/main/docs/evidence/seeded-astra-campaign-20260906.md)
and [Linux campaign and replay](https://github.com/AI-Ascension/sts2-harness/blob/main/docs/evidence/linux-seeded-campaign-20260906.md)
records.

The repeat-seed path is separately build-confirmed against the operator-supplied STS2 v0.107.1
assemblies and remains limited to an isolated single-player Custom-run scope. The dated campaign
records use an explicit seed and fresh-process replay through the normal runtime path; they do not
establish support for later-floor checkpoints, standard/daily/multiplayer modes, another host
version, or every settings and cleanup permutation.

## Local validation

Run the policy checker from this directory:

~~~text
cargo run --locked --offline --package repo-policy -- --strict
~~~

Then run formatting, Clippy, and workspace tests as documented in
[docs/TESTING.md](docs/TESTING.md). These local gates prove repository and source-level invariants
only; they do not promote host compatibility beyond the evidence level recorded above.

## Bounded runtime slice

The interop package now includes an authenticated runtime adapter with two fixed routes (default
bind address `127.0.0.1`): `GET /api/v1/runtime/state` and `POST /api/v1/runtime/action`. The native
listener validates bounded HTTP input and identity headers, then invokes the managed bridge. Managed
work is queued and executed from the Godot `SceneTree.ProcessFrame` callback. The only accepted
action is the safe, host-visible `show_runtime_probe`; acceptance requires the status overlay to be
observed and returns the fresh `status_overlay_visible` witness defined by the copied
[`runtime-v1` artifact](protocol-artifact/runtime-v1/README.md).

The listener requires `STS2_RUNTIME_TOKEN` and the built-in Runtime API toggle. Its bind address and
port are staged, persisted, and applied immediately by the AI-Ascension settings tab;
`STS2_RUNTIME_BIND_ADDRESS` and `STS2_RUNTIME_PORT` override those values for automation. The
listener is disabled when its toggle is off, its token is absent, or its address or port is invalid.
The exact STS2 v0.107.1 Windows x86-64 probe is recorded as confirmed in the dated host evidence.
The separate runtime-v3 gameplay path has bounded native Windows/Linux campaign evidence, while
the supported action and observation surface remains limited to the recorded host/version and
the [live combat scope](docs/LIVE_COMBAT_DEMO.md); no other host or platform is implied.
