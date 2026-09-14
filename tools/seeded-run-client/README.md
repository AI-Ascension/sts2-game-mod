# Seeded-run client

Minimal, reviewed client for the game-mod runtime route `POST /v2/seeded-run`
(`sts2-protocol/seeded-run-v1`). It exists so an operator can place a bounded standard seeded
run on an authorized disposable host and then read native projections such as
`/api/v4/runtime/expert-state`.

## Scope and honesty

- It assembles the canonical `start_request` and computes the selected-context digest locally,
  using the exact field order and SHA-256 canonicalization of the managed host.
- It reads the schema digest from `schemas/seeded-run-v1.schema.json` (or accepts `--schema-digest`).
- **Host-derived values are supplied by the operator** and are never fabricated: the runtime
  instance/session/lease identities and epoch, the host generation, the profile-baseline digest,
  and the game/mod compatibility identities and assembly digests.
- It does not launch the game, install the addon, or mutate any profile. Placement is a host action
  and remains subject to the host's `LIVE_AUTHORIZATION` and disposable-profile rules.

## Usage

```bash
cargo run --locked --offline --package sts2-seeded-run-client -- \
  --host 127.0.0.1 --port 15526 --token "$STS2_RUNTIME_TOKEN" \
  --caller-id opencode --instance-id <instance> --session-id <session> \
  --lease-id <lease> --lease-epoch <epoch> --generation <generation> \
  --correlation-id <corr> --operation-id run-1 --seed my-seed --run-mode seeded_training \
  --schema-file schemas/seeded-run-v1.schema.json \
  --context-id ctx-1 --ascension 0 --act <act-1> \
  --selection-policy standard_default --save-policy enabled \
  --profile-baseline-kind fresh --profile-baseline-identity <id> \
  --profile-baseline-digest <sha256> \
  --game-identity <simple-name/version> --game-digest <sha256-of-sts2.dll> \
  --mod-identity AIAscensionSTS2GameMod/1.0.0.0 --mod-digest <sha256-of-mod-dll> \
  --send
```

Omit `--send` (or pass `--print`) to print the request without contacting the host.

## Validation status

Unit tests pin the canonical digest against a fixed contract vector and validate the bounded field
grammar. Live placement against the exact host is an operator-run step: the profile-baseline digest
is captured by the mod at initialization, and the current host generation must match the request.
This tool does not claim that a run was placed.
