# Decision registry

Each normative decision has one four-digit identifier matching its filename and `# ADR NNNN:`
heading. Historical decisions retain their identifier; changing status does not free a number.
Repository policy rejects duplicate identifiers and mismatched numbered filenames/headings.
Old URLs may retain thin `# Moved:` redirects to the normative document; redirects contain no
decision body and do not reserve another identifier. Local link validation checks their targets.

| ID | Decision |
| --- | --- |
| 0001 | [Managed loader and Rust native boundary](0001-managed-loader-rust-native-boundary.md) |
| 0002 | [Initial game compatibility baseline](0002-initial-game-compatibility-baseline.md) |
| 0003 | [Rust-first implementation](0003-rust-first-with-managed-loader-exception.md) |
| 0004 | [Non-destructive scaffold](0004-non-destructive-target-scaffold.md) |
| 0005 | [Ownership and dependency direction](0005-mod-ownership-and-dependency-direction.md) |
| 0006 | [Sixth-target protocol scope](0006-current-sixth-target-protocol-scope.md) |
| 0007 | [Wave2 initialization](0007-wave2-codebase-initialization.md) |
| 0008 | [Minimal POC seam](0008-minimal-poc-game-mod-seam.md) |
| 0009 | [Runtime addon load smoke](0009-runtime-addon-load-smoke.md) |
| 0010 | [Runtime-v1 host probe](0010-runtime-v1-host-probe.md) |
| 0011 | [Optional ModConfig settings, historical](0011-optional-mod-settings.md) |
| 0012 | [Runtime listener settings](0012-runtime-listener-settings.md) |
| 0013 | [Ephemeral runtime-session launcher](0013-ephemeral-runtime-session-launcher.md) |
| 0014 | [Runtime-v2 fake boundary](0014-runtime-v2-fake-boundary.md) |
| 0015 | [First-party Workshop package](0015-steam-workshop-first-party-package.md) |
| 0016 | [Runtime-v2 host adapter candidate](0016-runtime-v2-host-adapter-candidate.md) |
| 0018 | [Neutral Runtime-v3 host-thread bridge, source-only](0018-runtime-v3-gameplay-bridge.md) |
| 0019 | [Repeat-seed practice replay](0019-repeat-seed-practice-replay.md) |
| 0030 | [Visible combat intent projection](0030-visible-combat-intent-projection.md) |
| 0031 | [Native standard seeded-run adapter](0031-native-standard-seeded-run-adapter.md) |
| 0032 | [Native co-op source candidate](0032-native-coop-source-candidate.md) |
| 0034 | [Rust release provenance tooling boundary](0034-rust-release-provenance-tooling.md) |
| 0035 | [Additive permitted map projection](0035-runtime-map-projection.md) |
| 0036 | [Additive Runtime-v4 rest-option action transport](0036-runtime-v4-rest-option-action.md) |
| 0037 | [Native checkpoint coverage inventory](0037-native-checkpoint-coverage-inventory.md) |
| 0038 | [Game-content manifest boundary](0038-game-content-manifest-boundary.md) |
| 0039 | [Field availability and completeness boundary](0039-field-availability-and-completeness-boundary.md) |
| 0040 | [Seeded-run RNG and entropy audit boundary](0040-seeded-run-rng-and-entropy-audit.md) |
| 0041 | [Save-profile selection and disposable boundary](0041-save-profile-selection-and-disposable-boundary.md) |
| 0042 | [Native checkpoint restore boundary](0042-native-checkpoint-restore-boundary.md) |
| 0043 | [Native checkpoint capture port](0043-native-checkpoint-capture-port.md) |
| 0044 | [Owner-local live card state projection](0044-live-card-state-projection.md) |
| 0045 | [Owner-local power and status state](0045-power-status-state.md) |
| 0046 | [Owner-local combat bookkeeping and public draw-pile projection](0046-combat-bookkeeping-projection.md) |
| 0047 | [Owner-local character resource and secondary-entity state](0047-character-resource-secondary-entity-state.md) |
| 0048 | [Owner-local structured enemy intents and public targets](0048-structured-enemy-intents.md) |
| 0049 | [Owner-local enemy, boss, elite, and minion reference definitions](0049-enemy-definitions.md) |
| 0050 | [Owner-local act, encounter, and map-generation reference data](0050-act-encounter-reference.md) |
| 0051 | [Owner-local retained map knowledge](0051-retained-map-knowledge.md) |
| 0052 | [Owner-local event definition and branch reference data](0052-event-reference.md) |
| 0053 | [Owner-local reward offer definition and generation reference data](0053-reward-offer-reference.md) |
| 0054 | [Restricted canonical checkpoint encoder and conformance witness](0054-restricted-canonical-checkpoint-encoder.md) |
| 0055 | [Checkpoint capture admission barrier and operation ledger](0055-checkpoint-capture-admission-barrier.md) |
| 0056 | [Pinned read-only ModelDb registry probe](0056-pinned-readonly-modeldb-registry-probe.md) |
| 0057 | [Closed checkpoint payload schema v1 and typed model](0057-checkpoint-payload-schema-v1.md) |
| 0058 | [Owner-local read-only game settings reference data](0058-settings-reference.md) |
| 0060 | [Owner-local read-only run configuration reference](0060-run-configuration-reference.md) |

The Runtime-v2 fake and Workshop decisions formerly both used 0011. Their content is unchanged apart from identifier
and references, and the old paths remain redirects. The original 0011 settings record remains
historical rather than being silently rewritten to claim the current built-in implementation.

The Runtime-v2 host adapter uses 0016; its old 0014 path is a redirect only.
Decision 0017 remains reserved for the bounded Runtime-v3 card proposal retained on a separate
branch. That reservation does not mean the bounded proposal has merged.
